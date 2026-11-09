# Changelog

All notable changes to Keyport SSH are documented in this file.

## v0.2.0 — 2026-11-09

### 新增

- **高级连接选项**：新增可折叠的高级面板，可选择远端 `authorized_keys` 路径：
  - `~/.ssh/authorized_keys`（OpenSSH 默认）
  - `/etc/dropbear/authorized_keys`（iStoreOS / Dropbear root 预设）
  - 自定义绝对路径或 `~/` 用户目录路径
- **路径安全**：Rust 端严格校验远端路径（ASCII 白名单、拒绝相对路径、`.`/`..`、空路径段），并对路径与公钥执行 POSIX shell 转义；远端命令模板固定，避免命令注入。
- **Dropbear 权限处理**：写入 Dropbear 路径时自动创建父目录、将授权文件设为 `600`，但不修改 `/etc` 下任何目录权限。
- **Ed25519 / RSA 密钥类型**：新生成的密钥可选择 Ed25519 或 RSA 3072；RSA 用于不识别 Ed25519 的旧版 Dropbear / 老设备。密钥文件分别使用 `id_ed25519_<name>` 与 `id_rsa_<name>`。
- **检查更新**：顶部版本按钮可主动查询 GitHub Releases，展示最新版本、发布日期和版本说明，并提供官方下载入口；只读取公开信息，不静默安装、不上传本机数据。

### 改进

- 整体提升提示性文字、表单说明、错误提示、交接弹窗与完成页的字号和可读性。
- 准备页、密码交接弹窗、通行证预览和完成页现在会明确展示远端 `authorized_keys` 路径与密钥类型。
- 公钥安装命令会校验 `KEYPORT_OK` 标记，避免远端“退出码为 0 但实际未写入”的假成功。

### 安装包

- Windows x64：MSI
- Linux x64：DEB / RPM
- macOS universal：DMG

## v0.1.0 — 2026-11-09

- 首个公开版本：密钥签发、ssh-agent 托管、SSH 配置托管、一次性密码公钥交接、主机指纹钉住与连接测试。
