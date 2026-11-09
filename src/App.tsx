import { useCallback, useEffect, useRef, useState } from "react";
import {
  AlertTriangle,
  ArrowUpRight,
  DownloadCloud,
  FolderOpen,
  KeyRound,
  LockKeyhole,
  RefreshCw,
  ShieldAlert,
  X,
} from "lucide-react";

import { AccessModal } from "./components/AccessModal";
import { ManifestForm } from "./components/ManifestForm";
import { PreparedCard } from "./components/PreparedCard";
import { ResultView } from "./components/ResultView";
import { SideRail } from "./components/SideRail";
import { UpdateModal } from "./components/UpdateModal";
import { VoyageCard } from "./components/VoyageCard";
import { Spinner } from "./components/Ui";
import { api, isDemo, normalizeError, type KeyportError } from "./lib/api";
import { RELEASES_PAGE_URL, APP_VERSION } from "./lib/version";
import { cx } from "./lib/utils";
import { toConnectionInput } from "./lib/validation";
import type {
  EnvironmentReport,
  FinishResult,
  FormField,
  FormState,
  JourneyScreen,
  PrepareResult,
  ToastMessage,
  UpdateInfo,
} from "./types";

const EMPTY_FORM: FormState = {
  name: "",
  host: "",
  user: "",
  port: "22",
  keyType: "ed25519",
  remotePathPreset: "openssh",
  customRemotePath: "",
};

export default function App() {
  const [environment, setEnvironment] = useState<EnvironmentReport | null>(null);
  const [environmentLoading, setEnvironmentLoading] = useState(true);
  const [environmentError, setEnvironmentError] = useState<KeyportError | null>(null);
  const [form, setForm] = useState<FormState>(EMPTY_FORM);
  const [screen, setScreen] = useState<JourneyScreen>("form");
  const [preparing, setPreparing] = useState(false);
  const [prepare, setPrepare] = useState<PrepareResult | null>(null);
  const [prepareError, setPrepareError] = useState<KeyportError | null>(null);
  const [installBusy, setInstallBusy] = useState(false);
  const [installError, setInstallError] = useState<KeyportError | null>(null);
  const [accessOpen, setAccessOpen] = useState(false);
  const [result, setResult] = useState<FinishResult | null>(null);
  const [reuseCandidate, setReuseCandidate] = useState<KeyportError | null>(null);
  const [retryingTest, setRetryingTest] = useState(false);
  const [toast, setToast] = useState<ToastMessage | null>(null);
  const [updateOpen, setUpdateOpen] = useState(false);
  const [updateChecking, setUpdateChecking] = useState(false);
  const [updateInfo, setUpdateInfo] = useState<UpdateInfo | null>(null);
  const [updateError, setUpdateError] = useState<KeyportError | null>(null);
  const toastTimer = useRef<number | null>(null);
  const mainRef = useRef<HTMLElement>(null);

  const notify = useCallback((text: string, tone: ToastMessage["tone"] = "neutral") => {
    if (toastTimer.current) {
      window.clearTimeout(toastTimer.current);
    }
    setToast({ id: Date.now(), text, tone });
    toastTimer.current = window.setTimeout(() => setToast(null), 4200);
  }, []);

  const loadEnvironment = useCallback(async () => {
    setEnvironmentLoading(true);
    setEnvironmentError(null);
    try {
      const report = await api.diagnose();
      setEnvironment(report);
    } catch (error) {
      setEnvironmentError(normalizeError(error));
    } finally {
      setEnvironmentLoading(false);
    }
  }, []);

  useEffect(() => {
    void loadEnvironment();
    return () => {
      if (toastTimer.current) {
        window.clearTimeout(toastTimer.current);
      }
    };
  }, [loadEnvironment]);

  useEffect(() => {
    mainRef.current?.scrollTo({ top: 0, behavior: "smooth" });
  }, [screen]);

  function handleFieldChange(field: FormField, value: string) {
    setForm((current) => ({ ...current, [field]: value }));
    if (prepareError) {
      setPrepareError(null);
    }
  }

  async function runPrepare(reuseExistingKey: boolean) {
    if (preparing) {
      return;
    }
    setPreparing(true);
    setPrepareError(null);
    setReuseCandidate(null);
    try {
      const prepared = await api.prepareConnection(toConnectionInput(form, reuseExistingKey));
      setPrepare(prepared);
      setEnvironment((current) => (current ? { ...current, agent: prepared.agent } : current));
      setScreen("prepared");
      setAccessOpen(true);
    } catch (error) {
      const info = normalizeError(error);
      if (info.code === "KEY_EXISTS") {
        setReuseCandidate(info);
      } else {
        setPrepareError(info);
      }
    } finally {
      setPreparing(false);
    }
  }

  function handleSubmit() {
    void runPrepare(false);
  }

  function handleResumePassword() {
    if (!prepare) {
      return;
    }
    setAccessOpen(true);
  }

  function handleFinished(finished: FinishResult) {
    setResult(finished);
    setEnvironment((current) => (current ? { ...current, agent: finished.agent } : current));
    setAccessOpen(false);
    setScreen("result");
    notify(
      finished.test.ok ? "连接已验证，钥匙顺利靠岸。" : "公钥已安装，但连接测试仍需要处理。",
      finished.test.ok ? "success" : "danger",
    );
  }

  function handleRestart() {
    setForm(EMPTY_FORM);
    setPrepare(null);
    setPrepareError(null);
    setResult(null);
    setScreen("form");
    setAccessOpen(false);
    setInstallError(null);
    mainRef.current?.scrollTo({ top: 0, behavior: "smooth" });
  }

  function handleBackToForm() {
    setPrepare(null);
    setScreen("form");
    setAccessOpen(false);
    setPrepareError(null);
  }

  async function handleInstallOpenSsh() {
    setInstallBusy(true);
    setInstallError(null);
    try {
      const install = await api.installOpenSsh();
      setEnvironment(install.environment);
      setEnvironmentError(null);
      notify("OpenSSH 已就绪，可以继续配置。", "success");
    } catch (error) {
      setInstallError(normalizeError(error));
    } finally {
      setInstallBusy(false);
    }
  }

  async function handleOpenFolder() {
    try {
      await api.openSshFolder();
      if (isDemo) {
        notify("预览模式下不会真的打开文件夹。", "neutral");
      }
    } catch (error) {
      notify(normalizeError(error).message, "danger");
    }
  }

  async function handleRetryTest() {
    if (!result) {
      return;
    }
    setRetryingTest(true);
    try {
      const test = await api.testConnection(result.alias);
      setResult((current) => (current ? { ...current, test } : current));
      notify(
        test.ok ? "试航通过，连接已经恢复正常。" : "测试仍未通过，请检查服务器 sshd 与文件权限。",
        test.ok ? "success" : "danger",
      );
    } catch (error) {
      const info = normalizeError(error);
      setResult((current) =>
        current
          ? {
              ...current,
              test: {
                ok: false,
                message: info.message,
                stdout: "",
                stderr: info.hint ?? "",
                exitCode: null,
                durationMs: 0,
              },
            }
          : current,
      );
      notify(info.message, "danger");
    } finally {
      setRetryingTest(false);
    }
  }

  function handleCopyError(error: unknown) {
    notify(`复制失败：${normalizeError(error).message}`, "danger");
  }

  async function runUpdateCheck() {
    if (updateChecking) {
      return;
    }
    setUpdateChecking(true);
    setUpdateError(null);
    try {
      const info = await api.checkUpdate();
      setUpdateInfo(info);
    } catch (error) {
      setUpdateError(normalizeError(error));
    } finally {
      setUpdateChecking(false);
    }
  }

  function handleOpenUpdate() {
    setUpdateOpen(true);
    if (!updateInfo && !updateChecking) {
      void runUpdateCheck();
    }
  }

  async function handleOpenRelease(url: string) {
    try {
      await api.openReleasePage(url);
      if (isDemo) {
        notify("预览模式下不会真的打开浏览器。", "neutral");
      }
    } catch (error) {
      notify(normalizeError(error).message, "danger");
    }
  }

  return (
    <div className="app">
      <a className="skip-link" href="#main">
        跳到主要内容
      </a>
      <SideRail
        environment={environment}
        screen={screen}
        demo={isDemo}
        loading={environmentLoading}
        onOpenFolder={() => void handleOpenFolder()}
      />

      <main className="main" id="main" ref={mainRef}>
        <header className="topbar">
          <div className="topbar__meta">
            <span className={cx("topbar__signal", environmentLoading && "is-pulsing")} aria-hidden="true" />
            <span>{environment?.platformLabel ?? "LOCAL"}</span>
            <span className="topbar__slash">/</span>
            <span>{environment?.arch ?? "detecting"}</span>
            {isDemo ? <span className="topbar__demo">PREVIEW</span> : null}
          </div>
          <div className="topbar__actions">
            <button
              type="button"
              className="icon-button"
              onClick={() => void handleOpenFolder()}
              aria-label="打开 SSH 配置目录"
              title="打开 .ssh 目录"
            >
              <FolderOpen size={17} aria-hidden="true" />
            </button>
            <button
              type="button"
              className="icon-button"
              onClick={() => void loadEnvironment()}
              disabled={environmentLoading || installBusy}
              aria-label="重新检查本机环境"
              title="重新检查环境"
            >
              <RefreshCw size={16} className={environmentLoading ? "spin" : undefined} aria-hidden="true" />
            </button>
            <button
              type="button"
              className={cx("version-button", updateInfo?.updateAvailable && "version-button--update")}
              onClick={handleOpenUpdate}
              aria-label={updateInfo?.updateAvailable ? "发现新版本，查看更新" : "检查更新"}
              title="检查更新"
            >
              {updateInfo?.updateAvailable ? (
                <span className="version-button__dot" aria-hidden="true" />
              ) : (
                <DownloadCloud size={13} aria-hidden="true" />
              )}
              <span>v{APP_VERSION}</span>
              <span className="version-button__label">
                {updateInfo?.updateAvailable ? "有更新" : "检查更新"}
              </span>
            </button>
          </div>
        </header>

        {screen === "result" && result ? (
          <ResultView
            result={result}
            environment={environment}
            onRestart={handleRestart}
            onOpenFolder={() => void handleOpenFolder()}
            onCopyError={handleCopyError}
            onRetryTest={() => void handleRetryTest()}
            retryingTest={retryingTest}
          />
        ) : (
          <>
            <Hero environment={environment} />

            <div className="deck">
              <div className="deck__main">
                {screen === "prepared" && prepare ? (
                  <PreparedCard
                    prepare={prepare}
                    onOpenPassword={handleResumePassword}
                    onReset={handleBackToForm}
                    onOpenFolder={() => void handleOpenFolder()}
                    onCopyError={handleCopyError}
                  />
                ) : (
                  <ManifestForm
                    form={form}
                    onChange={handleFieldChange}
                    onSubmit={handleSubmit}
                    busy={preparing}
                    environment={environment}
                    environmentLoading={environmentLoading}
                    environmentError={environmentError}
                    installBusy={installBusy}
                    installError={installError}
                    submitError={prepareError}
                    onInstallOpenSsh={() => void handleInstallOpenSsh()}
                    onRetryEnvironment={() => void loadEnvironment()}
                  />
                )}
              </div>
              <VoyageCard form={form} prepare={screen === "prepared" ? prepare : null} />
            </div>

            <footer className="main-footer">
              <span>
                <LockKeyhole size={12} aria-hidden="true" />
                密码一次使用，不落盘 · SSH 私钥全程留在本机
              </span>
              <span className="main-footer__mark">LOCAL BY DESIGN / KEYPORT SSH</span>
            </footer>
          </>
        )}
      </main>

      <AccessModal
        open={accessOpen}
        prepare={prepare}
        onClose={() => setAccessOpen(false)}
        onFinished={handleFinished}
      />

      <UpdateModal
        open={updateOpen}
        checking={updateChecking}
        info={updateInfo}
        error={updateError}
        onClose={() => setUpdateOpen(false)}
        onRetry={() => void runUpdateCheck()}
        onOpenRelease={(url) => void handleOpenRelease(url)}
        onOpenAllReleases={() => void handleOpenRelease(RELEASES_PAGE_URL)}
      />

      <ReuseDialog
        error={reuseCandidate}
        busy={preparing}
        onCancel={() => setReuseCandidate(null)}
        onConfirm={() => void runPrepare(true)}
      />

      <div className="toast-region" aria-live="polite" aria-atomic="true">
        {toast ? (
          <div key={toast.id} className={cx("toast", `toast--${toast.tone}`)} role="status">
            {toast.tone === "success" ? (
              <span className="toast__mark">✓</span>
            ) : toast.tone === "danger" ? (
              <AlertTriangle size={16} aria-hidden="true" />
            ) : (
              <span className="toast__mark toast__mark--neutral">i</span>
            )}
            <span>{toast.text}</span>
          </div>
        ) : null}
      </div>
    </div>
  );
}

function Hero({ environment }: { environment: EnvironmentReport | null }) {
  return (
    <section className="hero" aria-labelledby="hero-title">
      <div className="hero__copy">
        <span className="eyebrow">SSH KEY CONCIERGE / 01</span>
        <h1 id="hero-title">
          Every key
          <br />
          needs a <em>harbor.</em>
        </h1>
        <p className="hero__lede">
          生成 Ed25519 或 RSA 密钥，自动托管到 ssh-agent，写好 SSH 配置，
          再把公钥安全地送到你指定的 authorized_keys。
        </p>
        <div className="hero__specs" aria-label="安全特性">
          <span>ED25519 / RSA</span>
          <span className="hero__spec-dot" aria-hidden="true" />
          <span>ONE-TIME PASSWORD</span>
          <span className="hero__spec-dot" aria-hidden="true" />
          <span>LOCAL FIRST</span>
        </div>
      </div>

      <div className="hero-orbit" aria-hidden="true">
        <span className="hero-orbit__label hero-orbit__label--top">SECURE HANDOFF</span>
        <span className="hero-orbit__ring hero-orbit__ring--outer" />
        <span className="hero-orbit__ring hero-orbit__ring--mid" />
        <span className="hero-orbit__ring hero-orbit__ring--inner" />
        <span className="hero-orbit__orbit hero-orbit__orbit--one" />
        <span className="hero-orbit__orbit hero-orbit__orbit--two" />
        <span className="hero-orbit__core">
          <KeyRound size={21} />
        </span>
        <span className="hero-orbit__tick hero-orbit__tick--port">PORT 22</span>
        <span className="hero-orbit__tick hero-orbit__tick--local">LOCAL</span>
        <span className="hero-orbit__label hero-orbit__label--bottom">
          {environment?.openssh.version ? "OPENSSH / READY" : "CHECKING LOCAL"}
        </span>
      </div>
    </section>
  );
}

function ReuseDialog({
  error,
  busy,
  onCancel,
  onConfirm,
}: {
  error: KeyportError | null;
  busy: boolean;
  onCancel: () => void;
  onConfirm: () => void;
}) {
  const dialogRef = useRef<HTMLDivElement>(null);
  const cancelRef = useRef<HTMLButtonElement>(null);
  const cancelHandler = useRef(onCancel);
  cancelHandler.current = onCancel;

  useEffect(() => {
    if (!error) {
      return;
    }
    const previous = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    const timer = window.setTimeout(() => cancelRef.current?.focus(), 30);
    const previousOverflow = document.body.style.overflow;
    document.body.style.overflow = "hidden";
    function onKeyDown(event: globalThis.KeyboardEvent) {
      if (event.key === "Escape") {
        cancelHandler.current();
        return;
      }
      if (event.key !== "Tab" || !dialogRef.current) {
        return;
      }
      const focusable = Array.from(
        dialogRef.current.querySelectorAll<HTMLElement>(
          'button:not([disabled]), input:not([disabled]), [tabindex="0"]',
        ),
      ).filter((element) => element.getClientRects().length > 0);
      if (focusable.length === 0) {
        return;
      }
      const first = focusable[0];
      const last = focusable[focusable.length - 1];
      if (event.shiftKey && document.activeElement === first) {
        event.preventDefault();
        last.focus();
      } else if (!event.shiftKey && document.activeElement === last) {
        event.preventDefault();
        first.focus();
      }
    }
    document.addEventListener("keydown", onKeyDown);
    return () => {
      window.clearTimeout(timer);
      document.removeEventListener("keydown", onKeyDown);
      document.body.style.overflow = previousOverflow;
      previous?.focus();
    };
  }, [error]);

  if (!error) {
    return null;
  }

  return (
    <div
      className="modal-scrim modal-scrim--confirm"
      onMouseDown={(event) => {
        if (event.target === event.currentTarget) {
          onCancel();
        }
      }}
    >
      <div
        className="confirm-modal"
        role="dialog"
        aria-modal="true"
        aria-labelledby="reuse-title"
        ref={dialogRef}
      >
        <div className="confirm-modal__icon" aria-hidden="true">
          <ShieldAlert size={22} />
        </div>
        <span className="eyebrow eyebrow--ink">ALREADY AT THE DOCK</span>
        <h2 id="reuse-title">这里已经有一把同名钥匙。</h2>
        <p>{error.message}</p>
        <p className="confirm-modal__hint">{error.hint}</p>
        {error.detail ? <code className="confirm-modal__path">{error.detail}</code> : null}
        <div className="confirm-modal__note">
          <AlertTriangle size={15} aria-hidden="true" />
          Keyport 永远不会覆盖已有私钥。复用会继续使用当前这份密钥。
        </div>
        <div className="confirm-modal__actions">
          <button ref={cancelRef} type="button" className="btn btn--ghost-ink" onClick={onCancel}>
            <X size={16} aria-hidden="true" />
            换一个名称
          </button>
          <button type="button" className="btn btn--ink" onClick={onConfirm} disabled={busy}>
            {busy ? <Spinner size={16} /> : <KeyRound size={16} aria-hidden="true" />}
            复用现有密钥
            <ArrowUpRight size={15} aria-hidden="true" />
          </button>
        </div>
      </div>
    </div>
  );
}
