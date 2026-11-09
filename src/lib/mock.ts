import type {
  AgentStatus,
  ConnectionInput,
  EnvironmentReport,
  FinishInput,
  FinishResult,
  HostProbe,
  InstallResult,
  KeyType,
  PrepareResult,
  TestResult,
  UpdateInfo,
} from "../types";
import { KeyportError } from "./apiError";
import { OPENSSH_AUTHORIZED_KEYS_PATH, keyFileName } from "./remotePath";
import { APP_VERSION, RELEASES_PAGE_URL } from "./version";

const MOCK_FINGERPRINT = "SHA256:q3vT0V0D4C3F8aH2mJ5nK7pQ9rS1tU4wX6yZ8bC0dE2";

const delay = (ms: number) => new Promise<void>((resolve) => window.setTimeout(resolve, ms));

function pathInfo() {
  const isMac = /Mac/i.test(navigator.userAgent);
  const home = isMac ? "/Users/you" : "C:\\Users\\you";
  const sshDir = isMac ? `${home}/.ssh` : `${home}\\.ssh`;
  return {
    home,
    sshDir,
    configPath: isMac ? `${sshDir}/config` : `${sshDir}\\config`,
    knownHostsPath: isMac ? `${sshDir}/known_hosts` : `${sshDir}\\known_hosts`,
    sep: isMac ? "/" : "\\",
  };
}

function agentStatus(): AgentStatus {
  return {
    ready: true,
    running: true,
    autostart: true,
    socket: /Mac/i.test(navigator.userAgent) ? "/Users/you/.ssh/keyport-agent.sock" : null,
    manager: /Mac/i.test(navigator.userAgent) ? "launchd · LaunchAgent" : "Windows Service",
    summary: "ssh-agent 运行中，已设为登录自动启动。",
    detail: null,
  };
}

let mockEnv: EnvironmentReport = {
  platform: /Mac/i.test(navigator.userAgent) ? "macos" : "windows",
  arch: "x86_64",
  platformLabel: /Mac/i.test(navigator.userAgent) ? "macOS" : "Windows",
  ...(() => {
    const paths = pathInfo();
    return {
      home: paths.home,
      sshDir: paths.sshDir,
      configPath: paths.configPath,
      knownHostsPath: paths.knownHostsPath,
    };
  })(),
  openssh: {
    available: true,
    version: "OpenSSH_for_Windows_9.5p1, LibreSSL 3.8.2",
    missing: [],
    installHint: "",
    autoInstallSupported: true,
  },
  agent: agentStatus(),
};

const preparedNames = new Set<string>();
let mockHostKnown = false;

function prepareResult(
  name: string,
  host: string,
  user: string,
  port: number,
  reused: boolean,
  keyType: KeyType,
  remoteAuthorizedKeysPath: string,
): PrepareResult {
  const paths = pathInfo();
  const keyName = keyFileName(name, keyType);
  const keyPath = `${paths.sshDir}${paths.sep}${keyName}`;
  const configBlock = `Host ${name}
    HostName ${host}
    User ${user}
${port === 22 ? "" : `    Port ${port}\n`}    IdentityFile ~/.ssh/${keyName}
    IdentitiesOnly yes
    BatchMode yes
    ServerAliveInterval 30
`;
  const publicKey =
    keyType === "rsa"
      ? `ssh-rsa AAAAB3NzaC1yc2EAAAADAQABAAABAQExampleKeyportDemoRsaFingerprint keyport:${name}`
      : `ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIExampleKeyportDemoFingerprint keyport:${name}`;
  return {
    alias: name,
    host,
    user,
    port,
    keyPath,
    publicKey,
    fingerprint: MOCK_FINGERPRINT,
    algorithm: keyType === "rsa" ? "ssh-rsa" : "ssh-ed25519",
    keyType,
    remoteAuthorizedKeysPath,
    configPath: paths.configPath,
    knownHostsPath: paths.knownHostsPath,
    reusedKey: reused,
    configBlock,
    connectCommand: `ssh ${name}`,
    agent: agentStatus(),
    stages: [
      {
        id: "key",
        status: reused ? "reused" : "created",
        detail: `${keyPath} · ${MOCK_FINGERPRINT}`,
      },
      { id: "agent", status: "ready", detail: agentStatus().summary },
      { id: "config", status: "updated", detail: paths.configPath },
    ],
  };
}

export const mockApi = {
  async diagnose(): Promise<EnvironmentReport> {
    await delay(450);
    return structuredClone(mockEnv);
  },

  async installOpenSsh(): Promise<InstallResult> {
    await delay(1600);
    mockEnv = {
      ...mockEnv,
      openssh: {
        available: true,
        version: "OpenSSH_for_Windows_9.5p1, LibreSSL 3.8.2",
        missing: [],
        installHint: "",
        autoInstallSupported: true,
      },
    };
    return {
      installed: true,
      log: "OpenSSH 安装命令已完成。",
      environment: structuredClone(mockEnv),
    };
  },

  async prepareConnection(input: ConnectionInput): Promise<PrepareResult> {
    await delay(1150);
    const preparedName = `${input.keyType}:${input.name}`;
    const existed = preparedNames.has(preparedName);
    if (existed && !input.reuseExistingKey) {
      throw new KeyportError(
        "KEY_EXISTS",
        "同名连接密钥已经存在，Keyport 不会覆盖它。",
        "选择“复用现有密钥”继续，或换一个连接名称重新生成。",
        `${pathInfo().sshDir}${pathInfo().sep}${keyFileName(input.name, input.keyType)}`,
      );
    }
    preparedNames.add(preparedName);
    return prepareResult(
      input.name,
      input.host,
      input.user,
      input.port,
      existed,
      input.keyType,
      input.remoteAuthorizedKeysPath || OPENSSH_AUTHORIZED_KEYS_PATH,
    );
  },

  async probeHost(input: { host: string; port: number }): Promise<HostProbe> {
    await delay(950);
    return {
      host: input.host,
      port: input.port,
      hostToken: input.port === 22 ? input.host : `[${input.host}]:${input.port}`,
      fingerprint: MOCK_FINGERPRINT,
      algorithm: "ssh-ed25519",
      status: mockHostKnown ? "trusted" : "unknown",
      recordedFingerprints: mockHostKnown ? [MOCK_FINGERPRINT] : [],
      knownHostsPath: pathInfo().knownHostsPath,
    };
  },

  async finishSetup(input: FinishInput): Promise<FinishResult> {
    await delay(2200);
    if (input.password.toLowerCase() === "denied") {
      throw new KeyportError(
        "AUTH_FAILED",
        "服务器密码认证失败。",
        "请确认登录用户和密码；服务器也可能关闭了密码登录。密码不会被保存，可直接重试。",
      );
    }
    mockHostKnown = true;
    const startedAt = performance.now();
    const test: TestResult = {
      ok: true,
      message: "使用新密钥成功登录服务器，连接测试通过。",
      stdout: "",
      stderr: "",
      exitCode: 0,
      durationMs: performance.now() - startedAt,
    };
    return {
      copied: true,
      hostKeyLearned: true,
      agent: agentStatus(),
      test,
      alias: input.connection.name,
      host: input.connection.host,
      user: input.connection.user,
      port: input.connection.port,
      connectCommand: `ssh ${input.connection.name}`,
      keyPath: `${pathInfo().sshDir}${pathInfo().sep}${keyFileName(
        input.connection.name,
        input.connection.keyType,
      )}`,
      keyType: input.connection.keyType,
      remoteAuthorizedKeysPath: input.connection.remoteAuthorizedKeysPath,
      configPath: pathInfo().configPath,
      knownHostsPath: pathInfo().knownHostsPath,
      fingerprint: MOCK_FINGERPRINT,
      elapsedMs: 2200,
    };
  },

  async testConnection(_input: { name: string }): Promise<TestResult> {
    await delay(1000);
    return {
      ok: true,
      message: "使用新密钥成功登录服务器，连接测试通过。",
      stdout: "",
      stderr: "",
      exitCode: 0,
      durationMs: 1000,
    };
  },

  async openSshFolder(): Promise<void> {
    await delay(100);
  },

  async checkUpdate(): Promise<UpdateInfo> {
    await delay(750);
    const latestVersion = "0.2.0";
    return {
      currentVersion: APP_VERSION,
      latestVersion,
      updateAvailable: latestVersion !== APP_VERSION,
      releaseName: `Keyport SSH v${latestVersion}`,
      releaseUrl: `${RELEASES_PAGE_URL}/tag/v${latestVersion}`,
      downloadUrl: `${RELEASES_PAGE_URL}/tag/v${latestVersion}`,
      releaseNotes:
        "预览数据：这是 mock 后端返回的更新说明。\n\n- 新增远端 authorized_keys 路径预设\n- 支持 RSA 3072 密钥\n- 提升提示文字可读性",
      publishedAt: new Date().toISOString(),
    };
  },

  async openReleasePage(_url: string): Promise<void> {
    await delay(150);
  },
};
