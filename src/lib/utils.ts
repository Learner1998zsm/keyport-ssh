export function cx(...values: Array<string | false | null | undefined>): string {
  return values.filter(Boolean).join(" ");
}

export function shortFingerprint(fingerprint: string): string {
  if (fingerprint.length <= 26) {
    return fingerprint;
  }
  return `${fingerprint.slice(0, 19)}…${fingerprint.slice(-10)}`;
}

export function formattedTime(ms: number): string {
  if (ms < 1000) {
    return `${Math.max(1, Math.round(ms))} ms`;
  }
  return `${(ms / 1000).toFixed(1)} s`;
}

export function nowLabel(): string {
  return new Intl.DateTimeFormat("zh-CN", {
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
  }).format(new Date());
}
