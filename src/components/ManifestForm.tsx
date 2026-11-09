import { useRef, useState, type ChangeEvent, type FormEvent } from "react";
import {
  AlertTriangle,
  ArrowRight,
  Check,
  ChevronDown,
  Download,
  FileKey2,
  Fingerprint,
  Globe2,
  KeyRound,
  LoaderCircle,
  LockKeyhole,
  Server,
  ShieldCheck,
  SlidersHorizontal,
  UserRound,
} from "lucide-react";

import type {
  EnvironmentReport,
  FieldErrors,
  FormField,
  FormState,
} from "../types";
import type { KeyportError } from "../lib/api";
import {
  KEY_TYPE_OPTIONS,
  REMOTE_PATH_NOTES,
  REMOTE_PATH_OPTIONS,
  keyFileName,
  remotePathForForm,
} from "../lib/remotePath";
import { validateForm, hasErrors } from "../lib/validation";
import { cx } from "../lib/utils";
import { ErrorNotice, Spinner } from "./Ui";

const FIELD_ORDER: FormField[] = ["name", "host", "user", "port", "customRemotePath"];

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
  const [advancedOpen, setAdvancedOpen] = useState(false);
  const nameRef = useRef<HTMLInputElement>(null);
  const hostRef = useRef<HTMLInputElement>(null);
  const userRef = useRef<HTMLInputElement>(null);
  const portRef = useRef<HTMLInputElement>(null);
  const customRemotePathRef = useRef<HTMLInputElement>(null);

  const refs = {
    name: nameRef,
    host: hostRef,
    user: userRef,
    port: portRef,
    customRemotePath: customRemotePathRef,
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
    if (nextErrors.keyType || nextErrors.customRemotePath) {
      setAdvancedOpen(true);
    }
    if (hasErrors(nextErrors)) {
      const first = FIELD_ORDER.find((field) => nextErrors[field]);
      if (first) {
        const target = refs[first as keyof typeof refs];
        window.setTimeout(() => target?.current?.focus(), 0);
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
  const keyTypeLabel = form.keyType === "rsa" ? "RSA 3072" : "Ed25519";
  const effectiveRemotePath = remotePathForForm(form);
  const remotePathSummary = effectiveRemotePath || "等待填写自定义路径";
  const customPathError = showError("customRemotePath");
  const keyPrefix = form.keyType === "rsa" ? "id_rsa_" : "id_ed25519_";

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
                {keyPrefix}
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

          <div className={cx("advanced", advancedOpen && "advanced--open")}>
            <button
              type="button"
              className="advanced__toggle"
              aria-expanded={advancedOpen}
              aria-controls="advanced-options"
              onClick={() => setAdvancedOpen((current) => !current)}
            >
              <span className="advanced__toggle-icon" aria-hidden="true">
                <SlidersHorizontal size={15} />
              </span>
              <span className="advanced__toggle-copy">
                <strong>高级选项</strong>
                <span>
                  {keyTypeLabel} · {remotePathSummary}
                </span>
              </span>
              <ChevronDown size={16} className="advanced__chevron" aria-hidden="true" />
            </button>

            {advancedOpen ? (
              <div className="advanced__panel" id="advanced-options">
                <fieldset className="advanced-group">
                  <legend>密钥类型</legend>
                  <p className="advanced-group__hint">
                    Ed25519 是现代服务器首选；如果旧版 Dropbear 不识别 Ed25519，请改用 RSA。
                  </p>
                  <div className="choice-grid" role="radiogroup" aria-label="密钥类型">
                    {KEY_TYPE_OPTIONS.map((option) => (
                      <label
                        key={option.value}
                        className={cx("choice", form.keyType === option.value && "choice--active")}
                      >
                        <input
                          type="radio"
                          name="key-type"
                          value={option.value}
                          checked={form.keyType === option.value}
                          onChange={() => handleChange("keyType", option.value)}
                        />
                        <span className="choice__mark" aria-hidden="true">
                          <Check size={12} />
                        </span>
                        <span className="choice__copy">
                          <span className="choice__title">
                            <KeyRound size={13} aria-hidden="true" />
                            {option.label}
                            <em className="choice__tag">{option.tag}</em>
                          </span>
                          <span className="choice__desc">{option.description}</span>
                          <span className="choice__path">~/.ssh/{keyFileName(form.name.trim() || "home", option.value)}</span>
                        </span>
                      </label>
                    ))}
                  </div>
                  {showError("keyType") ? <FieldError id="key-type-error" message={showError("keyType")!} /> : null}
                </fieldset>

                <fieldset className="advanced-group">
                  <legend>远端公钥路径</legend>
                  <p className="advanced-group__hint">
                    决定公钥追加到服务器上的哪个 authorized_keys 文件。
                  </p>
                  <div className="choice-grid choice-grid--paths" role="radiogroup" aria-label="远端公钥路径">
                    {REMOTE_PATH_OPTIONS.map((option) => (
                      <label
                        key={option.value}
                        className={cx(
                          "choice",
                          form.remotePathPreset === option.value && "choice--active",
                        )}
                      >
                        <input
                          type="radio"
                          name="remote-path-preset"
                          value={option.value}
                          checked={form.remotePathPreset === option.value}
                          onChange={() => handleChange("remotePathPreset", option.value)}
                        />
                        <span className="choice__mark" aria-hidden="true">
                          <Check size={12} />
                        </span>
                        <span className="choice__copy">
                          <span className="choice__title">
                            <FileKey2 size={13} aria-hidden="true" />
                            {option.label}
                          </span>
                          <span className="choice__path">{option.path}</span>
                          <span className="choice__desc">{option.description}</span>
                        </span>
                      </label>
                    ))}
                  </div>

                  {form.remotePathPreset === "custom" ? (
                    <div className="field advanced-custom-path">
                      <div className={cx("control", customPathError && "control--error")}>
                        <FileKey2 size={17} className="control__icon control__icon--left" aria-hidden="true" />
                        <input
                          ref={customRemotePathRef}
                          id="custom-remote-path"
                          name="custom-remote-path"
                          value={form.customRemotePath}
                          placeholder="/data/ssh/authorized_keys"
                          spellCheck={false}
                          autoCapitalize="off"
                          autoCorrect="off"
                          maxLength={512}
                          className="has-leading-icon"
                          aria-label="自定义远端 authorized_keys 路径"
                          onChange={(event) => handleChange("customRemotePath", event.target.value)}
                          onBlur={() => handleBlur("customRemotePath")}
                          aria-invalid={Boolean(customPathError)}
                          aria-describedby={
                            customPathError
                              ? "custom-remote-path-hint custom-remote-path-error"
                              : "custom-remote-path-hint"
                          }
                        />
                      </div>
                      <p className="field__hint" id="custom-remote-path-hint">
                        支持 / 绝对路径或 ~/ 用户目录路径；不能包含空格、引号或 shell 特殊字符。
                        请确认目标是 authorized_keys 文件，而不要指向其他系统文件。
                      </p>
                      {customPathError ? (
                        <FieldError id="custom-remote-path-error" message={customPathError} />
                      ) : null}
                    </div>
                  ) : null}

                  <p className="advanced__mode-note">{REMOTE_PATH_NOTES[form.remotePathPreset]}</p>
                </fieldset>
              </div>
            ) : null}
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
