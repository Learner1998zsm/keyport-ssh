import { useState, type ReactNode } from "react";
import { AlertTriangle, Check, Copy, ExternalLink, LoaderCircle } from "lucide-react";

import { copyText, type KeyportError } from "../lib/api";
import { cx } from "../lib/utils";

export function Spinner({ size = 18 }: { size?: number }) {
  return <LoaderCircle size={size} className="spin" aria-hidden="true" />;
}

export function CopyButton({
  value,
  label = "复制",
  copiedLabel = "已复制",
  variant = "text",
  className,
  onError,
}: {
  value: string;
  label?: string;
  copiedLabel?: string;
  variant?: "text" | "icon";
  className?: string;
  onError?: (error: unknown) => void;
}) {
  const [copied, setCopied] = useState(false);

  async function handleCopy() {
    try {
      await copyText(value);
      setCopied(true);
      window.setTimeout(() => setCopied(false), 1800);
    } catch (error) {
      onError?.(error);
    }
  }

  if (variant === "icon") {
    return (
      <button
        type="button"
        className={cx("icon-copy", className)}
        onClick={handleCopy}
        aria-label={copied ? copiedLabel : label}
        title={copied ? copiedLabel : label}
      >
        {copied ? <Check size={16} aria-hidden="true" /> : <Copy size={16} aria-hidden="true" />}
      </button>
    );
  }

  return (
    <button type="button" className={cx("copy-button", className)} onClick={handleCopy}>
      {copied ? <Check size={15} aria-hidden="true" /> : <Copy size={15} aria-hidden="true" />}
      <span>{copied ? copiedLabel : label}</span>
    </button>
  );
}

export function ErrorNotice({
  error,
  compact = false,
  children,
  onRetry,
  retryLabel = "重试",
  retrying = false,
}: {
  error: KeyportError;
  compact?: boolean;
  children?: ReactNode;
  onRetry?: () => void;
  retryLabel?: string;
  retrying?: boolean;
}) {
  return (
    <div className={cx("error-notice", compact && "error-notice--compact")} role="alert">
      <AlertTriangle className="error-notice__icon" size={18} aria-hidden="true" />
      <div className="error-notice__body">
        <div className="error-notice__meta">
          <span className="error-notice__code">{error.code.replaceAll("_", " ")}</span>
        </div>
        <p className="error-notice__message">{error.message}</p>
        {error.hint ? <p className="error-notice__hint">{error.hint}</p> : null}
        {error.detail ? (
          <details className="raw-detail">
            <summary>查看技术详情</summary>
            <pre>{error.detail}</pre>
          </details>
        ) : null}
        <div className="error-notice__actions">
          {children}
          {onRetry ? (
            <button type="button" className="text-button" onClick={onRetry} disabled={retrying}>
              {retrying ? <Spinner size={14} /> : <ExternalLink size={14} aria-hidden="true" />}
              {retryLabel}
            </button>
          ) : null}
        </div>
      </div>
    </div>
  );
}

export function StatusDot({ tone = "idle" }: { tone?: "ready" | "idle" | "warning" | "danger" }) {
  return <span className={cx("status-dot", `status-dot--${tone}`)} aria-hidden="true" />;
}

export function InlineCode({ children }: { children: ReactNode }) {
  return <code className="inline-code">{children}</code>;
}
