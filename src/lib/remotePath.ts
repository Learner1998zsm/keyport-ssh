import type { FormState, KeyType, RemotePathPreset } from "../types";

export const OPENSSH_AUTHORIZED_KEYS_PATH = "~/.ssh/authorized_keys";
export const DROPBEAR_AUTHORIZED_KEYS_PATH = "/etc/dropbear/authorized_keys";

export interface KeyTypeOption {
  value: KeyType;
  label: string;
  tag: string;
  description: string;
}

export const KEY_TYPE_OPTIONS: KeyTypeOption[] = [
  {
    value: "ed25519",
    label: "Ed25519",
    tag: "推荐",
    description: "现代服务器首选，密钥短、握手快，安全性更好。",
  },
  {
    value: "rsa",
    label: "RSA 3072",
    tag: "兼容",
    description: "兼容不支持 Ed25519 的旧版 Dropbear 或老设备。",
  },
];

export interface RemotePathOption {
  value: RemotePathPreset;
  label: string;
  path: string;
  description: string;
}

export const REMOTE_PATH_OPTIONS: RemotePathOption[] = [
  {
    value: "openssh",
    label: "OpenSSH 默认",
    path: OPENSSH_AUTHORIZED_KEYS_PATH,
    description: "适用于大多数 Linux、macOS 与 BSD 服务器。",
  },
  {
    value: "dropbear",
    label: "Dropbear / iStoreOS",
    path: DROPBEAR_AUTHORIZED_KEYS_PATH,
    description: "root 登录的 Dropbear；自动处理父目录与 600 权限。",
  },
  {
    value: "custom",
    label: "自定义绝对路径",
    path: "手动填写",
    description: "用于非标准路径或特殊服务端配置。",
  },
];

export const REMOTE_PATH_NOTES: Record<RemotePathPreset, string> = {
  openssh:
    "Keyport 会确保 ~/.ssh 目录为 700、authorized_keys 文件为 600；密码只使用一次。",
  dropbear:
    "需要 root 或对 /etc/dropbear 有写权限。Keyport 会按需创建父目录并把文件设为 600，但不会修改 /etc 下任何目录的权限。",
  custom:
    "支持 / 绝对路径或 ~/ 用户目录路径。父目录不存在时会尝试创建；只有 ~/.ssh 会被调整为 700，/etc 下的目录权限不会被改动。",
};

export function keyFileName(alias: string, keyType: KeyType): string {
  return `${keyType === "rsa" ? "id_rsa" : "id_ed25519"}_${alias || "home"}`;
}

export function remotePathForForm(
  form: Pick<FormState, "remotePathPreset" | "customRemotePath">,
): string {
  switch (form.remotePathPreset) {
    case "dropbear":
      return DROPBEAR_AUTHORIZED_KEYS_PATH;
    case "custom":
      return form.customRemotePath.trim();
    default:
      return OPENSSH_AUTHORIZED_KEYS_PATH;
  }
}

const PATH_SEGMENT_PATTERN = /^[A-Za-z0-9._@+-]+$/;

/**
 * Mirrors the Rust-side validator so users get immediate feedback. The backend
 * always validates again and is the authority for the remote command.
 */
export function validateRemoteAuthorizedKeysPath(raw: string): string | null {
  const value = raw.trim();
  if (!value) {
    return "填写远端 authorized_keys 的完整路径。";
  }
  if (value.length > 512) {
    return "路径不能超过 512 个字符。";
  }
  if (!/^[\x20-\x7E]+$/.test(value)) {
    return "路径只能使用 ASCII 字符，不能包含换行或控制字符。";
  }

  let rest: string;
  if (value.startsWith("~/")) {
    rest = value.slice(2);
  } else if (value.startsWith("/")) {
    rest = value.slice(1);
  } else {
    return "路径必须以 / 开头，或以 ~/ 表示登录用户主目录。";
  }

  if (!rest) {
    return "还需要填写具体文件名。";
  }

  const segments = rest.split("/");
  if (segments.some((segment) => segment.length === 0)) {
    return "路径不能包含空路径段，也不能以 / 结尾。";
  }
  if (segments.some((segment) => segment === "." || segment === "..")) {
    return "路径不能包含 . 或 .. 路径段。";
  }
  if (segments.some((segment) => !PATH_SEGMENT_PATTERN.test(segment))) {
    return "路径只能包含字母、数字、点、下划线、短横线、加号、@ 和 /，不能有空格或 shell 特殊字符。";
  }

  return null;
}
