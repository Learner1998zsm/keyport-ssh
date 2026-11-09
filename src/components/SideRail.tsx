import { Check, FolderOpen, Lock, ShieldAlert } from "lucide-react";

import type { EnvironmentReport, JourneyScreen } from "../types";
import { cx } from "../lib/utils";
import { BrandLockup } from "./Logo";
import { Spinner, StatusDot } from "./Ui";

const STEPS = [
  { number: "01", title: "签发", caption: "KEYPAIR", detail: "Ed25519" },
  { number: "02", title: "托管", caption: "SSH AGENT", detail: "Auto-start" },
  { number: "03", title: "配置", caption: "SSH CONFIG", detail: "Managed block" },
  { number: "04", title: "送达", caption: "DELIVERY", detail: "Public key" },
] as const;

type StepState = "done" | "active" | "pending";

function stepState(index: number, screen: JourneyScreen): StepState {
  if (screen === "result") {
    return "done";
  }
  if (screen === "prepared") {
    return index < 3 ? "done" : "active";
  }
  return index === 0 ? "active" : "pending";
}

export function SideRail({
  environment,
  screen,
  demo,
  loading,
  onOpenFolder,
}: {
  environment: EnvironmentReport | null;
  screen: JourneyScreen;
  demo: boolean;
  loading: boolean;
  onOpenFolder: () => void;
}) {
  const agentTone = environment?.agent.ready
    ? "ready"
    : environment && !environment.openssh.available
      ? "warning"
      : "idle";

  return (
    <aside className="rail">
      <BrandLockup />

      <div className="rail__lede">
        <span className="rail__rule" />
        <p>从本机签发一把钥匙，<br />把它稳稳送上你的服务器。</p>
      </div>

      <ol className="journey" aria-label="配置进度">
        {STEPS.map((step, index) => {
          const state = stepState(index, screen);
          return (
            <li
              key={step.number}
              className={cx("journey__item", `journey__item--${state}`)}
              aria-current={state === "active" ? "step" : undefined}
            >
              <span className="journey__marker" aria-hidden="true">
                {state === "done" ? <Check size={13} strokeWidth={2.6} /> : step.number}
              </span>
              <span className="journey__copy">
                <span className="journey__caption">{step.caption}</span>
                <span className="journey__title">{step.title}</span>
                <span className="journey__detail">{step.detail}</span>
              </span>
            </li>
          );
        })}
      </ol>

      <div className="rail__spacer" />

      <section className="agent-card" aria-label="SSH agent 状态">
        <div className="agent-card__head">
          <span className="micro-label">SSH AGENT</span>
          {loading ? <Spinner size={14} /> : <StatusDot tone={agentTone as "ready" | "idle" | "warning"} />}
        </div>
        {environment ? (
          <>
            <div className="agent-card__state">
              {environment.agent.ready ? "已托管" : environment.openssh.available ? "待启动" : "环境未就绪"}
            </div>
            <p>{environment.agent.summary}</p>
            <div className="agent-card__meta">
              <span>{environment.agent.manager}</span>
              <span>{environment.agent.autostart ? "AUTOSTART ON" : "AUTOSTART OFF"}</span>
            </div>
          </>
        ) : (
          <>
            <div className="agent-card__state">检测中</div>
            <p>正在检查本机 OpenSSH 与 ssh-agent。</p>
            <div className="agent-card__meta">
              <span>LOCAL CHECK</span>
              <span>WAIT</span>
            </div>
          </>
        )}
        {environment && !environment.openssh.available ? (
          <div className="agent-card__alert">
            <ShieldAlert size={14} aria-hidden="true" />
            需要安装 OpenSSH
          </div>
        ) : null}
        <button type="button" className="agent-card__folder" onClick={onOpenFolder}>
          <FolderOpen size={14} aria-hidden="true" />
          打开 .ssh 目录
        </button>
      </section>

      <footer className="rail__foot">
        <span>
          <Lock size={12} aria-hidden="true" />
          LOCAL ONLY
        </span>
        <span>v0.1.0</span>
        {demo ? <span className="demo-flag">DEMO</span> : null}
      </footer>
    </aside>
  );
}
