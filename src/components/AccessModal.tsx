import {
  useEffect,
  useRef,
  useState,
  type FormEvent,
  type KeyboardEvent as ReactKeyboardEvent,
} from "react";
import {
  AlertTriangle,
  Check,
  ChevronRight,
  Eye,
  EyeOff,
  FileKey2,
  Fingerprint,
  KeyRound,
  LockKeyhole,
  ShieldAlert,
  ShieldCheck,
  X,
} from "lucide-react";

import type { FinishResult, HostProbe, PrepareResult } from "../types";
import { api, normalizeError, type KeyportError } from "../lib/api";
import { cx } from "../lib/utils";
import { ErrorNotice, Spinner } from "./Ui";

export function AccessModal({
  open,
  prepare,
  onClose,
  onFinished,
}: {
  open: boolean;
  prepare: PrepareResult | null;
  onClose: () => void;
  onFinished: (result: FinishResult) => void;
}) {
  const [probe, setProbe] = useState<HostProbe | null>(null);
  const [probeLoading, setProbeLoading] = useState(false);
  const [probeError, setProbeError] = useState<KeyportError | null>(null);
  const [password, setPassword] = useState("");
  const [showPassword, setShowPassword] = useState(false);
  const [trustHost, setTrustHost] = useState(false);
  const [submitting, setSubmitting] = useState(false);
  const [submitError, setSubmitError] = useState<KeyportError | null>(null);

  const dialogRef = useRef<HTMLDivElement>(null);
  const closeRef = useRef<HTMLButtonElement>(null);
  const passwordRef = useRef<HTMLInputElement>(null);
  const trustRef = useRef<HTMLInputElement>(null);
  const probeSequence = useRef(0);
  const previousFocus = useRef<HTMLElement | null>(null);

  async function startProbe() {
    if (!prepare) {
      return;
    }
    const sequence = ++probeSequence.current;
    setProbe(null);
    setProbeError(null);
    setProbeLoading(true);
    setTrustHost(false);
    setSubmitError(null);
    try {
      const nextProbe = await api.probeHost(prepare.host, prepare.port);
      if (sequence === probeSequence.current) {
        setProbe(nextProbe);
      }
    } catch (error) {
      if (sequence === probeSequence.current) {
        setProbeError(normalizeError(error));
      }
    } finally {
      if (sequence === probeSequence.current) {
        setProbeLoading(false);
      }
    }
  }

  useEffect(() => {
    if (!open || !prepare) {
      probeSequence.current += 1;
      return;
    }
    setPassword("");
    setShowPassword(false);
    setSubmitError(null);
    setSubmitting(false);
    void startProbe();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [open, prepare]);

  useEffect(() => {
    if (!open) {
      return;
    }
    previousFocus.current = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    const previousOverflow = document.body.style.overflow;
    document.body.style.overflow = "hidden";
    const focusTimer = window.setTimeout(() => closeRef.current?.focus(), 40);

    function handleEscape(event: globalThis.KeyboardEvent) {
      if (event.key === "Escape" && !submitting) {
        handleClose();
      }
    }
    document.addEventListener("keydown", handleEscape);
    return () => {
      window.clearTimeout(focusTimer);
      document.removeEventListener("keydown", handleEscape);
      document.body.style.overflow = previousOverflow;
      previousFocus.current?.focus();
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [open, submitting]);

  useEffect(() => {
    if (!open || probeLoading || !probe || submitting) {
      return;
    }
    if (probe.status === "changed") {
      closeRef.current?.focus();
    } else if (probe.status === "unknown") {
      trustRef.current?.focus();
    } else {
      passwordRef.current?.focus();
    }
  }, [open, probe, probeLoading, submitting]);

  function handleClose() {
    if (submitting) {
      return;
    }
    setPassword("");
    setShowPassword(false);
    probeSequence.current += 1;
    onClose();
  }

  function handleDialogKeyDown(event: ReactKeyboardEvent<HTMLDivElement>) {
    if (event.key !== "Tab") {
      return;
    }
    const dialog = dialogRef.current;
    if (!dialog) {
      return;
    }
    const focusable = Array.from(
      dialog.querySelectorAll<HTMLElement>(
        'a[href], button:not([disabled]), input:not([disabled]), [tabindex="0"]',
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

  async function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!prepare || !probe || submitting || !password) {
      return;
    }
    if (probe.status === "unknown" && !trustHost) {
      trustRef.current?.focus();
      return;
    }
    if (probe.status === "changed") {
      return;
    }

    const oneTimePassword = password;
    // Drop the React state reference before the IPC round-trip starts. The local
    // argument lives only for the duration of this function.
    setPassword("");
    setShowPassword(false);
    setSubmitError(null);
    setSubmitting(true);

    try {
      const result = await api.finishSetup({
        connection: {
          name: prepare.alias,
          host: prepare.host,
          user: prepare.user,
          port: prepare.port,
          reuseExistingKey: prepare.reusedKey,
          keyType: prepare.keyType,
          remoteAuthorizedKeysPath: prepare.remoteAuthorizedKeysPath,
        },
        password: oneTimePassword,
        expectedFingerprint: probe.fingerprint,
        trustHost: probe.status === "unknown" ? trustHost : true,
      });
      onFinished(result);
    } catch (error) {
      setSubmitError(normalizeError(error));
      setSubmitting(false);
      window.setTimeout(() => passwordRef.current?.focus(), 50);
    }
  }

  if (!open || !prepare) {
    return null;
  }

  const changed = probe?.status === "changed";
  const unknown = probe?.status === "unknown";
  const trusted = probe?.status === "trusted";
  const canSubmit = Boolean(
    probe &&
      !changed &&
      password.length > 0 &&
      (!unknown || trustHost) &&
      !submitting,
  );
  const activeStep = submitting ? 3 : probe ? 2 : 1;

  return (
    <div
      className="modal-scrim"
      onMouseDown={(event) => {
        if (event.target === event.currentTarget && !submitting) {
          handleClose();
        }
      }}
    >
      <div
        className="access-modal"
        role="dialog"
        aria-modal="true"
        aria-labelledby="access-modal-title"
        aria-describedby="access-modal-description"
        ref={dialogRef}
        onKeyDown={handleDialogKeyDown}
      >
        <div className="access-modal__band">
          <div className="access-modal__band-top">
            <span className="eyebrow">SECURE HANDOFF / ONE-TIME PASS</span>
            <button
              ref={closeRef}
              type="button"
              className="modal-close"
              onClick={handleClose}
              disabled={submitting}
              aria-label="关闭密码窗口"
            >
              <X size={19} aria-hidden="true" />
            </button>
          </div>
          <h2 id="access-modal-title">把钥匙送进港口。</h2>
          <p id="access-modal-description">
            {prepare.user}@{prepare.host}:{prepare.port} · {prepare.alias}
          </p>
          <ol className="handoff-steps" aria-label="交接进度">
            <li className={cx(activeStep >= 1 && "is-done", activeStep === 1 && "is-active")}>
              <span>{activeStep > 1 ? <Check size={11} /> : "01"}</span> 主机校验
            </li>
            <li className={cx(activeStep >= 2 && "is-done", activeStep === 2 && "is-active")}>
              <span>{activeStep > 2 ? <Check size={11} /> : "02"}</span> 一次密码
            </li>
            <li className={cx(activeStep >= 3 && "is-active")}>
              <span>03</span> 安装与试航
            </li>
          </ol>
        </div>

        <div className="access-modal__body">
          {probeLoading ? (
            <div className="probe-loader" aria-live="polite">
              <div className="probe-loader__radar" aria-hidden="true">
                <span className="probe-loader__ring" />
                <span className="probe-loader__ring probe-loader__ring--two" />
                <span className="probe-loader__sweep" />
                <Fingerprint size={22} />
              </div>
              <div>
                <span className="eyebrow eyebrow--ink">READING HOST KEY</span>
                <h3>正在确认对方的身份…</h3>
                <p>握手只交换主机公钥，不会在此阶段发送任何密码。</p>
              </div>
            </div>
          ) : null}

          {probeError && !probeLoading ? (
            <div className="modal-section">
              <ErrorNotice
                error={probeError}
                onRetry={() => void startProbe()}
                retryLabel="重新探测主机"
              />
              <div className="modal-actions modal-actions--end">
                <button type="button" className="btn btn--ghost-ink" onClick={handleClose}>
                  关闭
                </button>
              </div>
            </div>
          ) : null}

          {probe && !probeLoading && !submitting ? (
            <>
              <div
                className={cx(
                  "host-trust",
                  trusted && "host-trust--trusted",
                  unknown && "host-trust--unknown",
                  changed && "host-trust--changed",
                )}
              >
                <span className="host-trust__icon" aria-hidden="true">
                  {trusted ? <ShieldCheck size={21} /> : changed ? <ShieldAlert size={21} /> : <AlertTriangle size={21} />}
                </span>
                <div>
                  <span className="micro-label">
                    {trusted ? "HOST KEY TRUSTED" : changed ? "HOST KEY CHANGED" : "FIRST CONTACT"}
                  </span>
                  <h3>
                    {trusted
                      ? "这台服务器以前来过，指纹一致。"
                      : changed
                        ? "警告：主机密钥与旧记录不一致。"
                        : "第一次见这台服务器，请核对手指纹。"}
                  </h3>
                  <p>
                    {trusted
                      ? "known_hosts 中已有这把主机公钥，可以安全继续。"
                      : changed
                        ? "这可能意味着服务器被重装，也可能意味着中间人攻击。Keyport 已暂停操作。"
                        : "如果无法向服务器管理员确认指纹，请不要继续。"}
                  </p>
                </div>
              </div>

              <div className="host-identity-card">
                <div className="host-identity-card__head">
                  <span className="micro-label">
                    <Fingerprint size={12} aria-hidden="true" />
                    HOST KEY / SHA256
                  </span>
                  <span className="mono-tag">{probe.algorithm}</span>
                </div>
                <code className="host-fingerprint">{probe.fingerprint}</code>
                <div className="host-identity-meta">
                  <span>
                    TARGET <strong>{probe.hostToken}</strong>
                  </span>
                  <span>
                    RECORD <strong>{trusted ? "MATCH" : changed ? "CONFLICT" : "NEW"}</strong>
                  </span>
                </div>
                {changed && probe.recordedFingerprints.length > 0 ? (
                  <div className="recorded-fingerprints">
                    <span className="micro-label">RECORDED IN KNOWN_HOSTS</span>
                    {probe.recordedFingerprints.map((fingerprint) => (
                      <code key={fingerprint}>{fingerprint}</code>
                    ))}
                  </div>
                ) : null}
              </div>

              {changed ? (
                <div className="changed-note" role="alert">
                  <ShieldAlert size={16} aria-hidden="true" />
                  <p>
                    为了不把密码交给可能被冒充的主机，本次交接已经停止。请先确认服务器是否重装或更换过密钥。
                    如确认安全，可在终端移除旧记录后重新开始。
                  </p>
                </div>
              ) : null}

              {!changed ? (
                <form className="access-form" onSubmit={handleSubmit} autoComplete="off">
                  {unknown ? (
                    <label className="trust-check">
                      <input
                        ref={trustRef}
                        type="checkbox"
                        checked={trustHost}
                        onChange={(event) => setTrustHost(event.target.checked)}
                      />
                      <span className="trust-check__box" aria-hidden="true">
                        <Check size={13} />
                      </span>
                      <span>
                        我已通过可信渠道核对上面的主机指纹，并信任{" "}
                        <strong>{probe.hostToken}</strong>
                      </span>
                    </label>
                  ) : null}

                  <div className="access-target">
                    <FileKey2 size={17} aria-hidden="true" />
                    <div className="access-target__copy">
                      <span className="micro-label">DELIVERY TARGET / AUTHORIZED_KEYS</span>
                      <code>{prepare.remoteAuthorizedKeysPath}</code>
                      <span>
                        {prepare.keyType === "rsa" ? "RSA" : "Ed25519"} ·{" "}
                        {prepare.user}@{prepare.host}
                      </span>
                    </div>
                  </div>

                  <div className="field field--full access-password-field">
                    <div className="field__heading">
                      <label htmlFor="one-time-password">服务器登录密码</label>
                      <span className="field__code">SINGLE USE</span>
                    </div>
                    <div className="control control--password">
                      <LockKeyhole size={17} className="control__icon control__icon--left" aria-hidden="true" />
                      <input
                        ref={passwordRef}
                        id="one-time-password"
                        name="one-time-password"
                        type={showPassword ? "text" : "password"}
                        value={password}
                        placeholder={`${prepare.user} 的 SSH 登录密码`}
                        autoComplete="off"
                        spellCheck={false}
                        className="has-leading-icon has-trailing-button"
                        disabled={submitting}
                        onChange={(event) => setPassword(event.target.value)}
                      />
                      <button
                        type="button"
                        className="password-toggle"
                        onClick={() => setShowPassword((value) => !value)}
                        aria-label={showPassword ? "隐藏密码" : "显示密码"}
                      >
                        {showPassword ? <EyeOff size={17} aria-hidden="true" /> : <Eye size={17} aria-hidden="true" />}
                      </button>
                    </div>
                    <p className="field__hint">
                      服务器可能通过 password 或 keyboard-interactive 认证，Keyport 都会尝试一次。
                      密码只用一次，提交即从输入框清除。
                    </p>
                  </div>

                  {submitError ? (
                    <ErrorNotice
                      error={submitError}
                      compact
                      onRetry={
                        submitError.code === "HOST_KEY_CHANGED" ||
                        submitError.code === "HOST_KEY_UNTRUSTED" ||
                        submitError.code === "HOST_KEY_UNSUPPORTED"
                          ? () => void startProbe()
                          : undefined
                      }
                      retryLabel="重新探测主机"
                    />
                  ) : null}

                  <div className="modal-actions">
                    <button type="button" className="btn btn--ghost-ink" onClick={handleClose}>
                      取消
                    </button>
                    <button type="submit" className="btn btn--primary" disabled={!canSubmit}>
                      <ShieldCheck size={16} aria-hidden="true" />
                      安装公钥并测试连接
                      <ChevronRight size={16} aria-hidden="true" />
                    </button>
                  </div>
                  <p className="modal-privacy">
                    <LockKeyhole size={12} aria-hidden="true" />
                    Never stored. Never logged. 交接完成后立即丢弃密码引用。
                  </p>
                </form>
              ) : (
                <div className="modal-actions modal-actions--end">
                  <button type="button" className="btn btn--ghost-ink" onClick={handleClose}>
                    我会先核对服务器
                  </button>
                  <button type="button" className="btn btn--primary" onClick={() => void startProbe()}>
                    重新探测
                  </button>
                </div>
              )}
            </>
          ) : null}

          {submitting ? (
            <div className="handoff-progress" aria-live="polite">
              <div className="handoff-progress__visual" aria-hidden="true">
                <span className="handoff-progress__node">
                  <KeyRound size={17} />
                </span>
                <span className="handoff-progress__line">
                  <span />
                </span>
                <span className="handoff-progress__node handoff-progress__node--target">
                  <ShieldCheck size={17} />
                </span>
              </div>
              <span className="eyebrow eyebrow--ink">SECURE HANDOFF IN PROGRESS</span>
              <h3>正在把公钥送过海峡…</h3>
              <ul className="handoff-progress__list">
                <li className="is-active">
                  <Spinner size={14} /> 验证已确认的主机指纹
                </li>
                <li>
                  <span className="progress-dot" /> 一次性密码认证
                </li>
                <li>
                  <span className="progress-dot" /> 写入 {prepare.remoteAuthorizedKeysPath} 并运行 ssh 试航
                </li>
              </ul>
              <p>请保持窗口打开。密码已从输入框清除，将在交接结束后不再被引用。</p>
            </div>
          ) : null}
        </div>
      </div>
    </div>
  );
}
