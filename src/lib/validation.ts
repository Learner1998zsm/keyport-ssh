import type { ConnectionInput, FieldErrors, FormState } from "../types";
import { remotePathForForm, validateRemoteAuthorizedKeysPath } from "./remotePath";

const NAME_PATTERN = /^[A-Za-z0-9][A-Za-z0-9._-]{0,63}$/;
const USER_PATTERN = /^[A-Za-z0-9._@+-][A-Za-z0-9._@+-]{0,63}$/;
const HOST_PATTERN = /^[A-Za-z0-9][A-Za-z0-9._:-]*$/;

export function validateForm(form: FormState): FieldErrors {
  const errors: FieldErrors = {};
  const name = form.name.trim();
  if (!name) {
    errors.name = "给这条连接起一个名字。";
  } else if (!NAME_PATTERN.test(name)) {
    errors.name = "使用 1–64 位字母、数字、点、下划线或短横线，并以字母或数字开头。";
  }

  const host = form.host.trim().replace(/^\[|\]$/g, "");
  if (!host) {
    errors.host = "填写目标服务器的 IP 或主机名。";
  } else if (host.length > 253) {
    errors.host = "服务器地址过长。";
  } else if (host.startsWith("-") || !HOST_PATTERN.test(host) || !/[A-Za-z0-9]/.test(host)) {
    errors.host = "只填写 IP 或主机名，不要包含用户名、斜杠或空格。";
  }

  const user = form.user.trim();
  if (!user) {
    errors.user = "填写登录用户。";
  } else if (!USER_PATTERN.test(user)) {
    errors.user = "用户名不能包含空格或特殊字符。";
  }

  const port = Number(form.port);
  if (!/^\d{1,5}$/.test(form.port.trim()) || !Number.isInteger(port) || port < 1 || port > 65535) {
    errors.port = "端口必须在 1–65535 之间。";
  }

  if (form.keyType !== "ed25519" && form.keyType !== "rsa") {
    errors.keyType = "请选择密钥类型。";
  }

  if (form.remotePathPreset === "custom") {
    const remotePathError = validateRemoteAuthorizedKeysPath(form.customRemotePath);
    if (remotePathError) {
      errors.customRemotePath = remotePathError;
    }
  }

  return errors;
}

export function hasErrors(errors: FieldErrors): boolean {
  return Object.keys(errors).length > 0;
}

export function toConnectionInput(form: FormState, reuseExistingKey: boolean): ConnectionInput {
  return {
    name: form.name.trim(),
    host: form.host.trim(),
    user: form.user.trim(),
    port: Number(form.port),
    reuseExistingKey,
    keyType: form.keyType,
    remoteAuthorizedKeysPath: remotePathForForm(form),
  };
}

export function suggestedAlias(name: string): string {
  return name.trim() || "home";
}
