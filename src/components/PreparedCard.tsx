import {
  ArrowLeft,
  ArrowRight,
  Check,
  Fingerprint,
  FolderOpen,
  KeyRound,
  LockKeyhole,
  ShieldCheck,
  Terminal,
} from "lucide-react";

import type { PrepareResult } from "../types";
import { CopyButton } from "./Ui";

const STAGE_COPY: Record<string, { title: string; note: string }> = {
  key: { title: "密钥已签发", note: "Ed25519 密钥对已保存在本机" },
  agent: { title: "ssh-agent 已托管", note: "密钥已加载，登录时自动启动" },
  config: { title: "SSH 配置已写入", note: "受管理的 Host 配置块已就位" },
};

export function PreparedCard({
  prepare,
  onOpenPassword,
  onReset,
  onOpenFolder,
  onCopyError,
}: {
  prepare: PrepareResult;
  onOpenPassword: () => void;
  onReset: () => void;
  onOpenFolder: () => void;
  onCopyError: (error: unknown) => void;
}) {
  return (
    <section className="paper-card prepared" aria-labelledby="prepared-title">
      <div className="manifest__strip">
        <span className="micro-label">KP / READY FOR DELIVERY</span>
        <span className="manifest__serial">STEP&nbsp;02</span>
      </div>

      <div className="manifest__body">
        <header className="prepared__head">
          <div>
            <span className="eyebrow eyebrow--ink">ALL LINES SECURED</span>
            <h2 id="prepared-title">密钥已在港。</h2>
            <p className="prepared__lede">
              本机准备完成。接下来输入一次服务器密码，把公钥写进远端
              <code> authorized_keys</code>，然后我们会替你试航。
            </p>
          </div>
          <div className="clearance-seal" aria-hidden="true">
            <Check size={19} strokeWidth={2.4} />
            <span>CLEARED</span>
          </div>
        </header>

        <div className="prepared-route">
          <div className="prepared-route__local">
            <span className="micro-label">FROM</span>
            <strong>LOCALHOST</strong>
            <span>{prepare.keyPath}</span>
          </div>
          <div className="prepared-route__arrow" aria-hidden="true">
            <span />
            <KeyRound size={14} />
            <span />
          </div>
          <div className="prepared-route__remote">
            <span className="micro-label">TO</span>
            <strong>{prepare.user}@{prepare.host}</strong>
            <span>Port {prepare.port}</span>
          </div>
        </div>

        <ol className="stage-list" aria-label="已完成步骤">
          {prepare.stages.map((stage, index) => {
            const copy = STAGE_COPY[stage.id] ?? { title: stage.id, note: stage.detail };
            return (
              <li key={stage.id}>
                <span className="stage-list__index">{String(index + 1).padStart(2, "0")}</span>
                <span className="stage-list__check" aria-hidden="true">
                  <Check size={14} strokeWidth={2.6} />
                </span>
                <span className="stage-list__copy">
                  <strong>{copy.title}</strong>
                  <span>{copy.note}</span>
                </span>
                <span className="stage-list__tag">{stage.status.toUpperCase()}</span>
              </li>
            );
          })}
        </ol>

        <div className="prepared-evidence">
          <div className="fingerprint-block">
            <span className="micro-label">
              <Fingerprint size={12} aria-hidden="true" /> KEY FINGERPRINT
            </span>
            <code>{prepare.fingerprint}</code>
            <div className="prepared-evidence__actions">
              <CopyButton
                value={prepare.fingerprint}
                label="复制指纹"
                copiedLabel="指纹已复制"
                onError={onCopyError}
              />
              <CopyButton
                value={prepare.connectCommand}
                label="复制连接命令"
                copiedLabel="命令已复制"
                onError={onCopyError}
              />
            </div>
          </div>

          <div className="config-file-block">
            <span className="micro-label">
              <Terminal size={12} aria-hidden="true" /> CONFIG FILE
            </span>
            <code>{prepare.configPath}</code>
            <button type="button" className="text-button" onClick={onOpenFolder}>
              <FolderOpen size={14} aria-hidden="true" />
              打开 .ssh 目录
            </button>
          </div>
        </div>

        <div className="prepared__action">
          <div className="prepared__security">
            <LockKeyhole size={15} aria-hidden="true" />
            <span>
              密码只用于这次加密交接。提交后输入框立即清空，<br className="note-break" />
              不写入磁盘，也不会出现在技术日志里。
            </span>
          </div>
          <button type="button" className="btn btn--primary btn--wide" onClick={onOpenPassword}>
            <ShieldCheck size={17} aria-hidden="true" />
            <span>输入密码并安装公钥</span>
            <ArrowRight size={16} aria-hidden="true" />
          </button>
        </div>

        <div className="prepared__back">
          <button type="button" className="text-button" onClick={onReset}>
            <ArrowLeft size={14} aria-hidden="true" />
            返回并修改（不会删除已生成的文件）
          </button>
        </div>
      </div>
    </section>
  );
}
