import { useRef, useState, type ChangeEvent, type FormEvent } from "react";
import {
  AlertTriangle,
  ArrowRight,
  Download,
  Fingerprint,
  Globe2,
  LoaderCircle,
  LockKeyhole,
  Server,
  ShieldCheck,
  UserRound,
} from "lucide-react";

import type {
  EnvironmentReport,
  FieldErrors,
  FormField,
  FormState,
} from "../types";
import type { KeyportError } from "../lib/api";
import { validateForm, hasErrors } from "../lib/validation";
import { cx } from "../lib/utils";
import { ErrorNotice, Spinner } from "./Ui";

const FIELD_ORDER: FormField[] = ["name", "host", "user", "port"];

export function ManifestForm({
  form,
  onChange,
  onSubmit,
  busy,
  environment,
  environmentLoading,
  environmentError,
  installBusy,
  installError,
  submitError,
  onInstallOpenSsh,
  onRetryEnvironment,
}: {
  form: FormState;
  onChange: (field: FormField, value: string) => void;
  onSubmit: () => void;
  busy: boolean;
  environment: EnvironmentReport | null;
  environmentLoading: boolean;
  environmentError: KeyportError | null;
  installBusy: boolean;
  installError: KeyportError | null;
  submitError: KeyportError | null;
  onInstallOpenSsh: () => void;
  onRetryEnvironment: () => void;
}) {
  const [touched, setTouched] = useState<Partial<Record<FormField, boolean>>>({});
  const [attempted, setAttempted] = useState(false);
  const [errors, setErrors] = useState<FieldErrors>({});
  const nameRef = useRef<HTMLInputElement>(null);
  const hostRef = useRef<HTMLInputElement>(null);
  const userRef = useRef<HTMLInputElement>(null);
  const portRef = useRef<HTMLInputElement>(null);

  const refs = {
    name: nameRef,
    host: hostRef,
    user: userRef,
    port: portRef,
  };

  function refreshErrors(nextForm: FormState) {
    setErrors(validateForm(nextForm));
  }

  function handleChange(field: FormField, value: string) {
    const nextForm = { ...form, [field]: value };
    onChange(field, value);
    if (touched[field] || attempted) {
      refreshErrors(nextForm);
    }
  }

  function handleBlur(field: FormField) {
    setTouched((current) => ({ ...current, [field]: true }));
    refreshErrors(form);
  }

  function showError(field: FormField): string | undefined {
    return touched[field] || attempted ? errors[field] : undefined;
  }

  function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setAttempted(true);
    const nextErrors = validateForm(form);
    setErrors(nextErrors);
    if (hasErrors(nextErrors)) {
      const first = FIELD_ORDER.find((field) => nextErrors[field]);
      if (first) {
        refs[first].current?.focus();
      }
      return;
    }
    if (!busy && environment?.openssh.available) {
      onSubmit();
    }
  }

  const opensshAvailable = environment?.openssh.available === true;
  const canSubmit = !busy && !installBusy && !environmentLoading && opensshAvailable;
  const envTone = environmentError
    ? "danger"
    : environmentLoading
      ? "idle"
      : opensshAvailable
        ? "ready"
        : "warning";

  return (
    <section className={cx("paper-card manifest", busy && "is-busy")} aria-labelledby="manifest-title">
      <div className="manifest__strip">
        <span className="micro-label">KP / NEW CONNECTION</span>
        <span className="manifest__serial">FORM&nbsp;01</span>
      </div>

      <div className="manifest__body">
        <header className="manifest__head">
          <div>
            <span className="eyebrow eyebrow--ink">BOARDING MANIFEST</span>
            <h2 id="manifest-title">填写靠港信息</h2>
          </div>
          <div className={cx("env-stamp", `env-stamp--${envTone}`)}>
            {environmentLoading ? (
              <Spinner size={14} />
            ) : opensshAvailable ? (
              <ShieldCheck size={14} aria-hidden="true" />
            ) : (
              <AlertTriangle size={14} aria-hidden="true" />
            )}
            <span>
              {environmentLoading
                ? "CHECKING"
                : opensshAvailable
                  ? "OPENSSH READY"
                  : environmentError
                    ? "ENV OFFLINE"
                    : "OPENSSH MISSING"}
            </span>
          </div>
        </header>

        <EnvironmentSetup
          environment={environment}
          environmentError={environmentError}
          environmentLoading={environmentLoading}
          installBusy={installBusy}
          installError={installError}
          onInstallOpenSsh={onInstallOpenSsh}
          onRetryEnvironment={onRetryEnvironment}
        />

        {submitError ? (
          <ErrorNotice error={submitError} compact>
            <span className="error-notice__aside">密钥与配置可能已写入本机，重试不会删除文件。</span>
          </ErrorNotice>
        ) : null}

        <form className="manifest-form" onSubmit={handleSubmit} autoComplete="off" noValidate>
          <div className="field field--full">
            <div className="field__heading">
              <label htmlFor="connection-name">
                连接名称 <span className="required-mark">必填</span>
              </label>
              <span className="field__code">01 / ALIAS</span>
            </div>
            <div className={cx("control", showError("name") && "control--error")}>
              <span className="control__affix" aria-hidden="true">
                id_ed25519_
              </span>
              <input
                ref={nameRef}
                id="connection-name"
                name="connection-name"
                value={form.name}
                placeholder="home"
                spellCheck={false}
                autoCapitalize="off"
                autoCorrect="off"
                onChange={(event: ChangeEvent<HTMLInputElement>) => handleChange("name", event.target.value)}
                onBlur={() => handleBlur("name")}
                aria-invalid={Boolean(showError("name"))}
                aria-required="true"
                aria-describedby={
                  showError("name")
                    ? "connection-name-hint connection-name-error"
                    : "connection-name-hint"
                }
              />
              <Fingerprint size={17} className="control__icon" aria-hidden="true" />
            </div>
            <p className="field__hint" id="connection-name-hint">
              用作 <code>ssh</code> 别名与密钥后缀，建议使用小写英文。
            </p>
            {showError("name") ? <FieldError id="connection-name-error" message={showError("name")!} /> : null}
          </div>

          <div className="field field--full">
            <div className="field__heading">
              <label htmlFor="connection-host">
                服务器地址 <span className="required-mark">必填</span>
              </label>
              <span className="field__code">02 / HOST</span>
            </div>
            <div className={cx("control", showError("host") && "control--error")}>
              <Globe2 size={17} className="control__icon control__icon--left" aria-hidden="true" />
              <input
                ref={hostRef}
                id="connection-host"
                name="connection-host"
                value={form.host}
                placeholder="10.126.126.3"
                spellCheck={false}
                autoCapitalize="off"
                autoCorrect="off"
                inputMode="url"
                className="has-leading-icon"
                onChange={(event) => handleChange("host", event.target.value)}
                onBlur={() => handleBlur("host")}
                aria-invalid={Boolean(showError("host"))}
                aria-required="true"
                aria-describedby={
                  showError("host")
                    ? "connection-host-hint connection-host-error"
                    : "connection-host-hint"
                }
              />
            </div>
            <p className="field__hint" id="connection-host-hint">
              填写 IPv4、IPv6 或主机名，不需要加用户名或斜杠。
            </p>
            {showError("host") ? <FieldError id="connection-host-error" message={showError("host")!} /> : null}
          </div>

          <div className="field-row">
            <div className="field">
              <div className="field__heading">
                <label htmlFor="connection-user">
                  登录用户 <span className="required-mark">必填</span>
                </label>
                <span className="field__code">03 / USER</span>
              </div>
              <div className={cx("control", showError("user") && "control--error")}>
                <UserRound size={17} className="control__icon control__icon--left" aria-hidden="true" />
                <input
                  ref={userRef}
                  id="connection-user"
                  name="connection-user"
                  value={form.user}
                  placeholder="z"
                  spellCheck={false}
                  autoCapitalize="off"
                  autoCorrect="off"
                  className="has-leading-icon"
                  onChange={(event) => handleChange("user", event.target.value)}
                  onBlur={() => handleBlur("user")}
                  aria-invalid={Boolean(showError("user"))}
                  aria-required="true"
                  aria-describedby={showError("user") ? "connection-user-error" : undefined}
                />
              </div>
              {showError("user") ? <FieldError id="connection-user-error" message={showError("user")!} /> : null}
            </div>

            <div className="field field--port">
              <div className="field__heading">
                <label htmlFor="connection-port">端口</label>
                <span className="field__code">04 / PORT</span>
              </div>
              <div className={cx("control", showError("port") && "control--error")}>
                <Server size={17} className="control__icon control__icon--left" aria-hidden="true" />
                <input
                  ref={portRef}
                  id="connection-port"
                  name="connection-port"
                  value={form.port}
                  placeholder="22"
                  inputMode="numeric"
                  maxLength={5}
                  className="has-leading-icon"
                  onChange={(event) => handleChange("port", event.target.value.replace(/\D/g, "").slice(0, 5))}
                  onBlur={() => handleBlur("port")}
                  aria-invalid={Boolean(showError("port"))}
                  aria-describedby={showError("port") ? "connection-port-error" : undefined}
                />
              </div>
              {showError("port") ? <FieldError id="connection-port-error" message={showError("port")!} /> : null}
            </div>
          </div>

          <div className="manifest-form__footer">
            <div className="manifest-form__note">
              <LockKeyhole size={15} aria-hidden="true" />
              <span>
                密码稍后只用于一次公钥交接，<br className="note-break" />
                不写入磁盘，也不记录日志。
              </span>
            </div>
            <button type="submit" className="btn btn--primary btn--wide" disabled={!canSubmit}>
              {busy ? <Spinner size={17} /> : <ArrowRight size={17} aria-hidden="true" />}
              <span>{busy ? "正在签发与配置…" : "生成并配置"}</span>
            </button>
          </div>
        </form>

      </div>

      {busy ? <div className="busy-sheen" aria-hidden="true" /> : null}
    </section>
  );
}

function FieldError({ id, message }: { id: string; message: string }) {
  return (
    <p className="field__error" id={id} role="alert">
      <AlertTriangle size={13} aria-hidden="true" />
      {message}
    </p>
  );
}

function EnvironmentSetup({
  environment,
  environmentError,
  environmentLoading,
  installBusy,
  installError,
  onInstallOpenSsh,
  onRetryEnvironment,
}: {
  environment: EnvironmentReport | null;
  environmentError: KeyportError | null;
  environmentLoading: boolean;
  installBusy: boolean;
  installError: KeyportError | null;
  onInstallOpenSsh: () => void;
  onRetryEnvironment: () => void;
}) {
  if (environmentError) {
    return (
      <ErrorNotice
        error={environmentError}
        compact
        onRetry={onRetryEnvironment}
        retryLabel={environmentLoading ? "重新检查中…" : "重新检查环境"}
        retrying={environmentLoading}
      />
    );
  }

  if (environment?.openssh.available || environmentLoading) {
    return null;
  }

  if (!environment) {
    return (
      <div className="setup-banner setup-banner--loading" aria-live="polite">
        <Spinner size={18} />
        <p>正在检查本机 OpenSSH、ssh-agent 和密钥目录…</p>
      </div>
    );
  }

  return (
    <div className="setup-banner">
      <div className="setup-banner__icon">
        <Download size={20} aria-hidden="true" />
      </div>
      <div className="setup-banner__copy">
        <span className="eyebrow eyebrow--ink">ONE THING FIRST</span>
        <h3>本机还没有准备好 OpenSSH</h3>
        <p>
          缺少 <code>{environment.openssh.missing.join(", ")}</code>。Keyport
          使用系统 OpenSSH 生成密钥、托管 agent 并测试连接。
        </p>
        {environment.openssh.autoInstallSupported ? (
          <div className="setup-banner__actions">
            <button
              type="button"
              className="btn btn--ink"
              onClick={onInstallOpenSsh}
              disabled={installBusy}
            >
              {installBusy ? <Spinner size={16} /> : <Download size={16} aria-hidden="true" />}
              <span>{installBusy ? "正在下载并安装…" : "自动下载并安装 OpenSSH"}</span>
            </button>
            <span className="setup-banner__hint">
              {installBusy ? "可能需要管理员授权，请留意系统弹窗。" : "安装完成后会自动重新检查。"}
            </span>
          </div>
        ) : (
          <div className="setup-banner__manual">
            <span>请手动安装后再检查环境：</span>
            <code>{environment.openssh.installHint}</code>
            <button type="button" className="text-button" onClick={onRetryEnvironment}>
              <LoaderCircle size={14} aria-hidden="true" />
              重新检查
            </button>
          </div>
        )}
        {installError ? <ErrorNotice error={installError} compact /> : null}
      </div>
    </div>
  );
}
