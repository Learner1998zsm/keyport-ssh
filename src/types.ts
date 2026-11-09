export type KeyType = "ed25519" | "rsa";

export type RemotePathPreset = "openssh" | "dropbear" | "custom";

export interface OpenSshInfo {
  available: boolean;
  version: string | null;
  missing: string[];
  installHint: string;
  autoInstallSupported: boolean;
}

export interface AgentStatus {
  ready: boolean;
  running: boolean;
  autostart: boolean;
  socket: string | null;
  manager: string;
  summary: string;
  detail: string | null;
}

export interface EnvironmentReport {
  platform: string;
  arch: string;
  platformLabel: string;
  home: string;
  sshDir: string;
  configPath: string;
  knownHostsPath: string;
  openssh: OpenSshInfo;
  agent: AgentStatus;
}

export interface SetupStage {
  id: "key" | "agent" | "config" | string;
  status: string;
  detail: string;
}

export interface PrepareResult {
  alias: string;
  host: string;
  user: string;
  port: number;
  keyPath: string;
  publicKey: string;
  fingerprint: string;
  algorithm: string;
  keyType: KeyType;
  remoteAuthorizedKeysPath: string;
  configPath: string;
  knownHostsPath: string;
  reusedKey: boolean;
  configBlock: string;
  connectCommand: string;
  agent: AgentStatus;
  stages: SetupStage[];
}

export type HostTrustStatus = "trusted" | "unknown" | "changed";

export interface HostProbe {
  host: string;
  port: number;
  hostToken: string;
  fingerprint: string;
  algorithm: string;
  status: HostTrustStatus;
  recordedFingerprints: string[];
  knownHostsPath: string;
}

export interface TestResult {
  ok: boolean;
  message: string;
  stdout: string;
  stderr: string;
  exitCode: number | null;
  durationMs: number;
}

export interface FinishResult {
  copied: boolean;
  hostKeyLearned: boolean;
  agent: AgentStatus;
  test: TestResult;
  alias: string;
  host: string;
  user: string;
  port: number;
  connectCommand: string;
  keyPath: string;
  keyType: KeyType;
  remoteAuthorizedKeysPath: string;
  configPath: string;
  knownHostsPath: string;
  fingerprint: string;
  elapsedMs: number;
}

export interface InstallResult {
  installed: boolean;
  log: string;
  environment: EnvironmentReport;
}

export interface ConnectionInput {
  name: string;
  host: string;
  user: string;
  port: number;
  reuseExistingKey: boolean;
  keyType: KeyType;
  remoteAuthorizedKeysPath: string;
}

export interface FinishInput {
  connection: ConnectionInput;
  password: string;
  expectedFingerprint: string;
  trustHost: boolean;
}

export interface FormState {
  name: string;
  host: string;
  user: string;
  port: string;
  keyType: KeyType;
  remotePathPreset: RemotePathPreset;
  customRemotePath: string;
}

export interface UpdateInfo {
  currentVersion: string;
  latestVersion: string;
  updateAvailable: boolean;
  releaseName: string | null;
  releaseUrl: string;
  downloadUrl: string;
  releaseNotes: string | null;
  publishedAt: string | null;
}

export type FormField = keyof FormState;

export type FieldErrors = Partial<Record<FormField, string>>;

export interface CommandErrorShape {
  code: string;
  message: string;
  hint?: string | null;
  detail?: string | null;
}

export type JourneyScreen = "form" | "prepared" | "result";

export type ToastTone = "neutral" | "success" | "danger";

export interface ToastMessage {
  id: number;
  text: string;
  tone: ToastTone;
}
