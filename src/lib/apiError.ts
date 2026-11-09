export class KeyportError extends Error {
  readonly code: string;
  readonly hint: string | null;
  readonly detail: string | null;

  constructor(code: string, message: string, hint?: string | null, detail?: string | null) {
    super(message);
    this.name = "KeyportError";
    this.code = code;
    this.hint = hint ?? null;
    this.detail = detail ?? null;
  }
}

export function normalizeError(value: unknown): KeyportError {
  if (value instanceof KeyportError) {
    return value;
  }
  if (value && typeof value === "object") {
    const candidate = value as Record<string, unknown>;
    if (typeof candidate.code === "string" && typeof candidate.message === "string") {
      return new KeyportError(
        candidate.code,
        candidate.message,
        typeof candidate.hint === "string" ? candidate.hint : null,
        typeof candidate.detail === "string" ? candidate.detail : null,
      );
    }
    if (typeof candidate.error === "string") {
      return normalizeError(candidate.error);
    }
    if (typeof candidate.message === "string") {
      return new KeyportError("INTERNAL", candidate.message);
    }
  }
  if (typeof value === "string") {
    try {
      return normalizeError(JSON.parse(value));
    } catch {
      return new KeyportError("INTERNAL", value);
    }
  }
  return new KeyportError("INTERNAL", "发生了未知错误，请重试。");
}
