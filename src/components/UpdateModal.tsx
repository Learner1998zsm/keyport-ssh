import { useEffect, useRef, type KeyboardEvent as ReactKeyboardEvent } from "react";
import {
  ArrowUpRight,
  BadgeCheck,
  DownloadCloud,
  RefreshCw,
  ShieldCheck,
  X,
} from "lucide-react";

import type { UpdateInfo } from "../types";
import type { KeyportError } from "../lib/api";
import { APP_VERSION } from "../lib/version";
import { cx } from "../lib/utils";
import { ErrorNotice, Spinner } from "./Ui";

function publishedLabel(value: string | null): string | null {
  if (!value) {
    return null;
  }
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) {
    return null;
  }
  return new Intl.DateTimeFormat("zh-CN", {
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
  }).format(date);
}

export function UpdateModal({
  open,
  checking,
  info,
  error,
  onClose,
  onRetry,
  onOpenRelease,
  onOpenAllReleases,
}: {
  open: boolean;
  checking: boolean;
  info: UpdateInfo | null;
  error: KeyportError | null;
  onClose: () => void;
  onRetry: () => void;
  onOpenRelease: (url: string) => void;
  onOpenAllReleases: () => void;
}) {
  const dialogRef = useRef<HTMLDivElement>(null);
  const closeRef = useRef<HTMLButtonElement>(null);
  const previousFocus = useRef<HTMLElement | null>(null);
  const onCloseRef = useRef(onClose);

  useEffect(() => {
    onCloseRef.current = onClose;
  }, [onClose]);

  useEffect(() => {
    if (!open) {
      return;
    }
    previousFocus.current =
      document.activeElement instanceof HTMLElement ? document.activeElement : null;
    const previousOverflow = document.body.style.overflow;
    document.body.style.overflow = "hidden";
    const focusTimer = window.setTimeout(() => closeRef.current?.focus(), 40);

    function handleEscape(event: globalThis.KeyboardEvent) {
      if (event.key === "Escape") {
        onCloseRef.current();
      }
    }
    document.addEventListener("keydown", handleEscape);
    return () => {
      window.clearTimeout(focusTimer);
      document.removeEventListener("keydown", handleEscape);
      document.body.style.overflow = previousOverflow;
      previousFocus.current?.focus();
    };
  }, [open]);

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

  if (!open) {
    return null;
  }

  const published = publishedLabel(info?.publishedAt ?? null);
  const releaseTitle = info?.releaseName?.trim() || (info ? `v${info.latestVersion}` : "");

  return (
    <div
      className="modal-scrim"
      onMouseDown={(event) => {
        if (event.target === event.currentTarget) {
          onClose();
        }
      }}
    >
      <div
        className="update-modal"
        role="dialog"
        aria-modal="true"
        aria-labelledby="update-modal-title"
        ref={dialogRef}
        onKeyDown={handleDialogKeyDown}
      >
        <header className="update-modal__head">
          <div>
            <span className="eyebrow">RELEASE CHANNEL / GITHUB</span>
            <h2 id="update-modal-title">检查更新</h2>
          </div>
          <button
            ref={closeRef}
            type="button"
            className="modal-close"
            onClick={onClose}
            aria-label="关闭更新窗口"
          >
            <X size={19} aria-hidden="true" />
          </button>
        </header>

        <div className="update-modal__body">
          {checking ? (
            <div className="update-loading" aria-live="polite">
              <Spinner size={26} />
              <div>
                <span className="eyebrow eyebrow--ink">CONTACTING GITHUB</span>
                <h3>正在查询最新版本…</h3>
                <p>只读取公开的 Releases 信息，不会发送本机数据。</p>
              </div>
            </div>
          ) : null}

          {!checking && error ? (
            <div className="update-error">
              <ErrorNotice
                error={error}
                onRetry={onRetry}
                retryLabel="重新检查"
                retrying={checking}
              />
              <div className="update-actions">
                <button type="button" className="btn btn--ghost-ink" onClick={onClose}>
                  关闭
                </button>
                <button type="button" className="btn btn--ink" onClick={onOpenAllReleases}>
                  打开 GitHub Releases
                  <ArrowUpRight size={15} aria-hidden="true" />
                </button>
              </div>
            </div>
          ) : null}

          {!checking && !error && info ? (
            <>
              <div
                className={cx(
                  "update-status",
                  info.updateAvailable && "update-status--available",
                )}
              >
                <span className="update-status__icon" aria-hidden="true">
                  {info.updateAvailable ? <DownloadCloud size={21} /> : <BadgeCheck size={21} />}
                </span>
                <div>
                  <span className="micro-label">
                    {info.updateAvailable ? "UPDATE AVAILABLE" : "UP TO DATE"}
                  </span>
                  <h3>
                    {info.updateAvailable
                      ? `发现新版本 v${info.latestVersion}`
                      : "已经是最新版本"}
                  </h3>
                  <p>
                    当前 v{info.currentVersion}
                    {info.updateAvailable ? ` · 最新 v${info.latestVersion}` : " · 无需更新"}
                  </p>
                </div>
              </div>

              <dl className="update-meta">
                <div>
                  <dt>CURRENT</dt>
                  <dd>v{info.currentVersion}</dd>
                </div>
                <div>
                  <dt>LATEST</dt>
                  <dd>v{info.latestVersion}</dd>
                </div>
                <div>
                  <dt>PUBLISHED</dt>
                  <dd>{published ?? "未知"}</dd>
                </div>
              </dl>

              {info.releaseNotes ? (
                <section className="update-notes" aria-label="版本说明">
                  <h4>
                    RELEASE NOTES
                    {releaseTitle ? <span>{releaseTitle}</span> : null}
                  </h4>
                  <div className="update-notes__body">{info.releaseNotes}</div>
                </section>
              ) : null}

              <div className="update-actions">
                <button type="button" className="text-button" onClick={onRetry}>
                  <RefreshCw size={14} aria-hidden="true" />
                  重新检查
                </button>
                <span className="update-actions__spacer" />
                <button type="button" className="btn btn--ghost-ink" onClick={onOpenAllReleases}>
                  查看全部版本
                </button>
                {info.updateAvailable ? (
                  <button
                    type="button"
                    className="btn btn--primary"
                    onClick={() => onOpenRelease(info.downloadUrl)}
                  >
                    <DownloadCloud size={16} aria-hidden="true" />
                    下载更新
                    <ArrowUpRight size={15} aria-hidden="true" />
                  </button>
                ) : null}
              </div>

              <p className="update-modal__privacy">
                <ShieldCheck size={14} aria-hidden="true" />
                检查更新只读取 GitHub Releases 的公开信息；Keyport 不会上传密钥、主机或任何本机数据。
              </p>
            </>
          ) : null}

          {!checking && !error && !info ? (
            <div className="update-loading">
              <DownloadCloud size={24} aria-hidden="true" />
              <div>
                <h3>还没有检查过更新</h3>
                <p>当前版本 v{APP_VERSION}。点击下方按钮查询 GitHub Releases。</p>
              </div>
            </div>
          ) : null}

          {!checking && !error && !info ? (
            <div className="update-actions">
              <button type="button" className="btn btn--ghost-ink" onClick={onClose}>
                稍后
              </button>
              <button type="button" className="btn btn--primary" onClick={onRetry}>
                <RefreshCw size={15} aria-hidden="true" />
                检查更新
              </button>
            </div>
          ) : null}
        </div>
      </div>
    </div>
  );
}
