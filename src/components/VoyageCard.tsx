import { KeyRound, LockKeyhole, Radio } from "lucide-react";

import type { FormState, PrepareResult } from "../types";
import { cx, shortFingerprint } from "../lib/utils";

export function VoyageCard({
  form,
  prepare,
}: {
  form: FormState;
  prepare: PrepareResult | null;
}) {
  const alias = (prepare?.alias ?? form.name.trim()) || "home";
  const host = (prepare?.host ?? form.host.trim()) || "target.host";
  const user = (prepare?.user ?? form.user.trim()) || "user";
  const port = prepare?.port ?? (Number(form.port) || 22);
  const keyName = `id_ed25519_${alias}`;
  const target = `${user}@${host}${port === 22 ? "" : `:${port}`}`;
  const fingerprint = prepare?.fingerprint;
  const ready = prepare !== null;

  const previewLines = [
    { key: "Host", value: alias },
    { key: "HostName", value: host },
    { key: "User", value: user },
    ...(port !== 22 ? [{ key: "Port", value: String(port) }] : []),
    { key: "IdentityFile", value: `~/.ssh/${keyName}` },
    { key: "IdentitiesOnly", value: "yes" },
  ];

  return (
    <aside
      className={cx("voyage-card", ready && "voyage-card--ready")}
      aria-label="SSH 通行证预览"
    >
      <div className="voyage-card__notch voyage-card__notch--left" aria-hidden="true" />
      <div className="voyage-card__notch voyage-card__notch--right" aria-hidden="true" />
      <div className="voyage-card__strip">
        <span className="micro-label">VOYAGE RECORD</span>
        <span className="voyage-card__serial">KP / SSH / {port}</span>
      </div>

      <div className="voyage-card__body">
        <div className="voyage-card__head">
          <div>
            <span className="eyebrow">PASSENGER KEY</span>
            <h3>{alias}</h3>
          </div>
          <span className={cx("status-chip", ready ? "status-chip--ready" : "status-chip--draft")}>
            <Radio size={12} aria-hidden="true" />
            {ready ? "READY" : "DRAFT"}
          </span>
        </div>

        <div className="route">
          <div className="route__endpoint">
            <span className="route__port">LOCAL</span>
            <strong>this device</strong>
          </div>
          <div className="route__line" aria-hidden="true">
            <span />
            <span className="route__key">
              <KeyRound size={14} />
            </span>
            <span />
          </div>
          <div className="route__endpoint route__endpoint--right">
            <span className="route__port">DESTINATION</span>
            <strong>{host}</strong>
          </div>
        </div>

        <dl className="voyage-fields">
          <div>
            <dt>DESTINATION</dt>
            <dd>{target}</dd>
          </div>
          <div>
            <dt>KEY</dt>
            <dd>~/.ssh/{keyName}</dd>
          </div>
          <div>
            <dt>CIPHER</dt>
            <dd>Ed25519</dd>
          </div>
          <div>
            <dt>AGENT</dt>
            <dd>{ready ? "Pinned · loaded" : "Pending"}</dd>
          </div>
        </dl>

        <div className="config-preview">
          <div className="config-preview__head">
            <span>~/.ssh/config</span>
            <span className={ready ? "config-preview__dot config-preview__dot--on" : "config-preview__dot"} />
          </div>
          <pre aria-label="SSH 配置预览">
            {previewLines.map((line) => (
              <span className="config-preview__line" key={line.key}>
                <span className="config-preview__key">{line.key}</span>
                <span className="config-preview__value"> {line.value}</span>
                {"\n"}
              </span>
            ))}
          </pre>
        </div>

        <div className="fingerprint-row">
          <div>
            <span className="micro-label">HOST KEY FINGERPRINT</span>
            <span className={cx("fingerprint-value", !fingerprint && "fingerprint-value--pending")}>
              {fingerprint ? shortFingerprint(fingerprint) : "探测主机后生成"}
            </span>
          </div>
          <LockKeyhole size={17} aria-hidden="true" className="fingerprint-row__icon" />
        </div>

        <div className="voyage-card__foot">
          <div className="barcode" aria-hidden="true" />
          <div className="voyage-card__foot-copy">
            <span>ISSUED LOCALLY</span>
            <span>NO TELEMETRY</span>
          </div>
        </div>
      </div>

      {ready ? (
        <div className="voyage-stamp" aria-hidden="true">
          <span>KEYPORT</span>
          <strong>ON<br />BOARD</strong>
          <span>ED25519</span>
        </div>
      ) : null}
    </aside>
  );
}
