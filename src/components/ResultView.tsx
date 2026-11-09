import {
  AlertTriangle,
  ArrowRight,
  Check,
  Clock3,
  FolderOpen,
  KeyRound,
  LockKeyhole,
  RotateCcw,
  Server,
  ShieldCheck,
  Terminal,
  UserRound,
} from "lucide-react";

import type { EnvironmentReport, FinishResult } from "../types";
import { cx, formattedTime, nowLabel, shortFingerprint } from "../lib/utils";
import { CopyButton, Spinner } from "./Ui";

export function ResultView({
  result,
  environment,
  onRestart,
  onOpenFolder,
  onCopyError,
  onRetryTest,
  retryingTest,
}: {
  result: FinishResult;
  environment: EnvironmentReport | null;
  onRestart: () => void;
  onOpenFolder: () => void;
  onCopyError: (error: unknown) => void;
  onRetryTest: () => void;
  retryingTest: boolean;
}) {
  const ok = result.test.ok;
  const terminalOutput = [result.test.stderr, result.test.stdout].filter(Boolean).join("\n").trim();

  return (
    <section className="result" aria-labelledby="result-title">
      <div className="result__hero">
        <div className="result__heading">
          <span className="eyebrow">ARRIVAL RECORD / 04</span>
          <h1 id="result-title" className={cx("result-title", !ok && "result-title--partial")}>
            {ok ? (
              <>
                Docked<span className="result-title__accent">.</span>
              </>
            ) : (
              <>
                Almost ashore<span className="result-title__accent">.</span>
              </>
            )}
          </h1>
          <p>
            {ok
              ? "新钥匙已经送达，SSH 试航通过。以后从终端直接登船即可。"
              : "公钥已经写入服务器，但最后的 SSH 试航没有通过。连接档案已经保存，可重试测试或检查服务器配置。"}
          </p>
          <div className="result__quickline">
            <span className={cx("status-chip", ok ? "status-chip--ready" : "status-chip--danger")}>
              {ok ? <Check size={12} aria-hidden="true" /> : <AlertTriangle size={12} aria-hidden="true" />}
              {ok ? "CONNECTION VERIFIED" : "TESTS NEED ATTENTION"}
            </span>
            <span className="result__hostline">
              {environment?.platformLabel ?? "LOCAL"} → {result.user}@{result.host}
              {result.port === 22 ? "" : `:${result.port}`}
            </span>
          </div>
        </div>

        <div className={cx("arrival-stamp", !ok && "arrival-stamp--hold")} aria-label={ok ? "连接测试通过" : "连接测试未通过"}>
          <span>KEYPORT / SSH</span>
          <strong>{ok ? "CLEARED" : "HOLD"}</strong>
          <span>{nowLabel()}</span>
          <span className="arrival-stamp__dot" aria-hidden="true" />
        </div>
      </div>

      <div className="command-ticket">
        <div className="command-ticket__head">
          <div>
            <span className="eyebrow eyebrow--ink">YOUR BOARDING CALL</span>
            <h2>{ok ? "以后这样连接" : "连接命令（待确认）"}</h2>
          </div>
          <span className="ticket-code">ALIAS / {result.alias.toUpperCase()}</span>
        </div>
        <div className="command-line">
          <span className="command-line__prompt" aria-hidden="true">
            $
          </span>
          <code>{result.connectCommand}</code>
          <CopyButton
            value={result.connectCommand}
            label="复制命令"
            copiedLabel="已复制"
            className="command-line__copy"
            onError={onCopyError}
          />
        </div>
        <p className="command-ticket__note">
          <LockKeyhole size={14} aria-hidden="true" />
          配置已写入 <code>{result.configPath}</code>。密钥由 ssh-agent 托管，服务器密码已从内存清除。
        </p>
        <div className="command-ticket__perforation" aria-hidden="true" />
        <div className="command-ticket__foot">
          <span>{result.agent.ready ? "SSH AGENT READY" : "SSH AGENT NOT READY"}</span>
          <span>{result.hostKeyLearned ? "HOST KEY PINNED" : "HOST KEY VERIFIED"}</span>
          <span>{result.fingerprint.slice(0, 16)}…</span>
        </div>
      </div>

      <div className="result-grid">
        <section className={cx("result-panel", "test-panel", !ok && "test-panel--fail")} aria-label="连接测试结果">
          <div className="result-panel__head">
            <span className="micro-label">
              <Terminal size={12} aria-hidden="true" /> LIVE TEST
            </span>
            <span className={cx("test-badge", ok ? "test-badge--ok" : "test-badge--fail")}>
              {ok ? <Check size={12} aria-hidden="true" /> : <AlertTriangle size={12} aria-hidden="true" />}
              {ok ? "PASSED" : "FAILED"}
            </span>
          </div>
          <div className="test-console" role="status">
            <div className="test-console__bar">
              <span />
              <span />
              <span />
              <em>ssh · batch mode</em>
            </div>
            <div className="test-console__body">
              <p className={ok ? "console-ok" : "console-fail"}>
                {ok ? "✓" : "!"} {result.test.message}
              </p>
              <p className="console-muted">
                exit {result.test.exitCode ?? "n/a"} · {formattedTime(result.test.durationMs)} · verified with pinned key
              </p>
              {terminalOutput ? (
                <details className="console-details">
                  <summary>测试输出</summary>
                  <pre>{terminalOutput}</pre>
                </details>
              ) : null}
            </div>
          </div>
          {!ok ? (
            <div className="test-panel__recovery">
              <p>
                常见原因：服务器禁用了公钥登录、authorized_keys 权限不正确，或 sshd 尚未重新加载。
              </p>
              <button type="button" className="btn btn--outline-paper" onClick={onRetryTest} disabled={retryingTest}>
                {retryingTest ? <Spinner size={15} /> : <RotateCcw size={15} aria-hidden="true" />}
                {retryingTest ? "正在重试…" : "重试连接测试"}
              </button>
            </div>
          ) : (
            <div className="test-panel__next">
              <span className="micro-label">
                <Clock3 size={12} aria-hidden="true" /> NEXT
              </span>
              <p>打开任意终端，粘贴上方命令。不需要再次输入密码，也不用手动指定密钥。</p>
            </div>
          )}
        </section>

        <section className="result-panel record-panel" aria-label="连接档案">
          <div className="result-panel__head">
            <span className="micro-label">
              <KeyRound size={12} aria-hidden="true" /> CONNECTION RECORD
            </span>
            <span className="mono-tag">ED25519</span>
          </div>
          <dl className="record-list">
            <div>
              <dt>
                <Server size={14} aria-hidden="true" /> HOST
              </dt>
              <dd>
                {result.user}@{result.host}
                {result.port === 22 ? "" : `:${result.port}`}
              </dd>
            </div>
            <div>
              <dt>
                <UserRound size={14} aria-hidden="true" /> IDENTITY
              </dt>
              <dd>{result.keyPath}</dd>
            </div>
            <div>
              <dt>
                <ShieldCheck size={14} aria-hidden="true" /> FINGERPRINT
              </dt>
              <dd className="record-fingerprint">
                <span>{shortFingerprint(result.fingerprint)}</span>
                <CopyButton
                  value={result.fingerprint}
                  variant="icon"
                  label="复制指纹"
                  copiedLabel="已复制指纹"
                  onError={onCopyError}
                />
              </dd>
            </div>
            <div>
              <dt>
                <Clock3 size={14} aria-hidden="true" /> TOTAL TIME
              </dt>
              <dd>{formattedTime(result.elapsedMs)}</dd>
            </div>
          </dl>
          <div className="record-panel__actions">
            <button type="button" className="text-button" onClick={onOpenFolder}>
              <FolderOpen size={14} aria-hidden="true" />
              打开 .ssh 目录
            </button>
            <button type="button" className="text-button" onClick={onRestart}>
              <RotateCcw size={14} aria-hidden="true" />
              配置另一台服务器
              <ArrowRight size={13} aria-hidden="true" />
            </button>
          </div>
        </section>
      </div>

      <div className="result__footer">
        <div>
          <span className="micro-label">WHAT KEYPORT DID</span>
          <p>
            签发密钥 · 配置 ssh-agent 自动启动 · 更新 SSH config · 写入远端 authorized_keys
            {result.hostKeyLearned ? " · 固定主机指纹" : ""}
          </p>
        </div>
        <span className="result__privacy">
          <LockKeyhole size={13} aria-hidden="true" />
          Password discarded · No telemetry
        </span>
      </div>
    </section>
  );
}
