use std::time::Duration;

use semver::Version;
use serde::{Deserialize, Serialize};

use crate::error::{codes, AppResult, CommandError};
use crate::util::truncate;

const RELEASE_PAGE_PREFIX: &str = "https://github.com/Learner1998zsm/keyport-ssh/releases";
const LATEST_RELEASE_API: &str =
    "https://api.github.com/repos/Learner1998zsm/keyport-ssh/releases/latest";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(15);
const MAX_RELEASE_NOTES: usize = 4000;
const USER_AGENT: &str = "Keyport-SSH-update-check";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    pub current_version: String,
    pub latest_version: String,
    pub update_available: bool,
    pub release_name: Option<String>,
    pub release_url: String,
    pub download_url: String,
    pub release_notes: Option<String>,
    pub published_at: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GithubRelease {
    tag_name: String,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    html_url: Option<String>,
    #[serde(default)]
    body: Option<String>,
    #[serde(default)]
    published_at: Option<String>,
    #[serde(default)]
    assets: Vec<GithubAsset>,
}

#[derive(Debug, Deserialize)]
struct GithubAsset {
    name: String,
    browser_download_url: String,
}

pub fn check_for_update() -> AppResult<UpdateInfo> {
    let release = fetch_latest_release()?;
    let current_raw = env!("CARGO_PKG_VERSION");
    let current = parse_version(current_raw).ok_or_else(|| {
        CommandError::new(codes::INTERNAL, "当前应用的版本号无法识别。").detail(current_raw)
    })?;
    let latest = parse_version(&release.tag_name).ok_or_else(|| {
        CommandError::new(
            codes::UPDATE_CHECK_FAILED,
            "GitHub 返回的最新版本号无法识别。",
        )
        .hint("请直接访问 GitHub Releases 页面查看版本。")
        .detail(release.tag_name.clone())
    })?;

    let release_url = release
        .html_url
        .filter(|url| is_allowed_release_url(url))
        .unwrap_or_else(|| format!("{RELEASE_PAGE_PREFIX}/tag/{}", release.tag_name));
    let download_url = select_download_url(&release.assets).unwrap_or_else(|| release_url.clone());

    Ok(UpdateInfo {
        current_version: current.to_string(),
        latest_version: latest.to_string(),
        update_available: latest > current,
        release_name: release.name,
        release_url,
        download_url,
        release_notes: release
            .body
            .map(|body| truncate(body.trim(), MAX_RELEASE_NOTES)),
        published_at: release.published_at,
    })
}

fn fetch_latest_release() -> AppResult<GithubRelease> {
    let agent = ureq::AgentBuilder::new().timeout(REQUEST_TIMEOUT).build();
    let response = agent
        .get(LATEST_RELEASE_API)
        .set("Accept", "application/vnd.github+json")
        .set("X-GitHub-Api-Version", "2022-11-28")
        .set("User-Agent", USER_AGENT)
        .call();

    match response {
        Ok(response) => {
            let body = response.into_string().map_err(|error| {
                CommandError::new(
                    codes::UPDATE_CHECK_FAILED,
                    "无法读取 GitHub 返回的更新信息。",
                )
                .detail(error.to_string())
            })?;
            serde_json::from_str(&body).map_err(|error| {
                CommandError::new(
                    codes::UPDATE_CHECK_FAILED,
                    "GitHub 返回的数据格式无法识别。",
                )
                .detail(error.to_string())
            })
        }
        Err(ureq::Error::Status(404, _)) => Err(CommandError::new(
            codes::UPDATE_CHECK_FAILED,
            "GitHub 上还没有已发布的版本。",
        )
        .hint("当前可能是首次发布前的开发版本，可以稍后再试。")),
        Err(ureq::Error::Status(status, _)) if status == 403 || status == 429 => Err(
            CommandError::new(codes::UPDATE_CHECK_FAILED, "GitHub 暂时限制了更新查询。")
                .hint("请稍后再试，或直接访问 GitHub Releases 页面。"),
        ),
        Err(ureq::Error::Status(status, _)) => Err(CommandError::new(
            codes::UPDATE_CHECK_FAILED,
            "GitHub 返回了异常的更新查询结果。",
        )
        .detail(format!("HTTP {status}"))),
        Err(ureq::Error::Transport(error)) => Err(CommandError::new(
            codes::UPDATE_CHECK_FAILED,
            "无法连接 GitHub 检查更新。",
        )
        .hint("请检查网络连接或代理设置后重试。")
        .detail(error.to_string())),
    }
}

fn parse_version(raw: &str) -> Option<Version> {
    let trimmed = raw.trim();
    let without_prefix = trimmed
        .strip_prefix('v')
        .or_else(|| trimmed.strip_prefix('V'))
        .unwrap_or(trimmed);
    Version::parse(without_prefix).ok()
}

fn select_download_url(assets: &[GithubAsset]) -> Option<String> {
    let extensions: &[&str] = if cfg!(windows) {
        &[".msi", ".exe"]
    } else if cfg!(target_os = "macos") {
        &[".dmg", ".app.tar.gz"]
    } else {
        &[".deb", ".rpm", ".appimage"]
    };

    extensions.iter().find_map(|extension| {
        assets
            .iter()
            .find(|asset| {
                asset.name.to_ascii_lowercase().ends_with(extension)
                    && is_allowed_release_url(&asset.browser_download_url)
            })
            .map(|asset| asset.browser_download_url.clone())
    })
}

fn is_allowed_release_url(url: &str) -> bool {
    let value = url.trim();
    !value.is_empty()
        && value.len() <= 2048
        && !value.chars().any(char::is_whitespace)
        && !value.contains('\\')
        && !value
            .split('/')
            .any(|segment| segment == "." || segment == "..")
        && (value == RELEASE_PAGE_PREFIX || value.starts_with(&format!("{RELEASE_PAGE_PREFIX}/")))
}

/// Open a GitHub Releases page (or one of its asset links) in the default
/// browser. Only this repository's release URLs are accepted.
pub fn open_release_page(url: &str) -> AppResult<()> {
    let value = url.trim();
    if !is_allowed_release_url(value) {
        return Err(CommandError::new(
            codes::VALIDATION,
            "只允许打开 Keyport 的 GitHub Releases 页面。",
        ));
    }
    open::that_detached(value).map_err(|error| {
        CommandError::new(codes::INTERNAL, "无法打开浏览器。").detail(error.to_string())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn semver_tags_ignore_the_v_prefix() {
        assert_eq!(parse_version("v0.2.0").unwrap().to_string(), "0.2.0");
        assert_eq!(parse_version(" 1.4.10 ").unwrap().to_string(), "1.4.10");
        assert!(parse_version("release-2026").is_none());
        assert!(Version::parse("0.10.0").unwrap() > Version::parse("0.9.0").unwrap());
    }

    #[test]
    fn only_keyport_release_urls_are_allowed() {
        assert!(is_allowed_release_url(RELEASE_PAGE_PREFIX));
        assert!(is_allowed_release_url(&format!(
            "{RELEASE_PAGE_PREFIX}/tag/v0.2.0"
        )));
        assert!(is_allowed_release_url(&format!(
            "{RELEASE_PAGE_PREFIX}/download/v0.2.0/Keyport.msi"
        )));
        assert!(!is_allowed_release_url("https://example.com/releases"));
        assert!(!is_allowed_release_url(
            "https://github.com/Learner1998zsm/keyport-ssh.evil/releases"
        ));
        assert!(!is_allowed_release_url(&format!(
            "{RELEASE_PAGE_PREFIX}/tag/v0.2.0 something"
        )));
        assert!(!is_allowed_release_url(&format!(
            "{RELEASE_PAGE_PREFIX}/../evil"
        )));
        assert!(!is_allowed_release_url("file:///etc/passwd"));
    }

    #[test]
    fn download_selection_skips_foreign_urls() {
        let assets = vec![GithubAsset {
            name: "Keyport_0.2.0_x64_en-US.msi".into(),
            browser_download_url: "https://example.com/evil.msi".into(),
        }];
        assert!(select_download_url(&assets).is_none());
    }

    #[test]
    fn download_selection_prefers_the_current_platform() {
        let assets = vec![
            GithubAsset {
                name: "Keyport_0.2.0_universal.dmg".into(),
                browser_download_url: format!("{RELEASE_PAGE_PREFIX}/download/v0.2.0/mac.dmg"),
            },
            GithubAsset {
                name: "Keyport_0.2.0_x64_en-US.msi".into(),
                browser_download_url: format!("{RELEASE_PAGE_PREFIX}/download/v0.2.0/win.msi"),
            },
            GithubAsset {
                name: "keyport_0.2.0_amd64.deb".into(),
                browser_download_url: format!("{RELEASE_PAGE_PREFIX}/download/v0.2.0/deb.deb"),
            },
            GithubAsset {
                name: "evil_0.2.0_x64_en-US.msi".into(),
                browser_download_url: "https://example.com/evil.msi".into(),
            },
        ];
        let selected = select_download_url(&assets).unwrap();
        if cfg!(windows) {
            assert_eq!(
                selected,
                format!("{RELEASE_PAGE_PREFIX}/download/v0.2.0/win.msi")
            );
        } else if cfg!(target_os = "macos") {
            assert_eq!(
                selected,
                format!("{RELEASE_PAGE_PREFIX}/download/v0.2.0/mac.dmg")
            );
        } else {
            assert_eq!(
                selected,
                format!("{RELEASE_PAGE_PREFIX}/download/v0.2.0/deb.deb")
            );
        }
    }
}
