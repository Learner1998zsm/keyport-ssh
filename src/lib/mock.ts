import type {
  AgentStatus,
  EnvironmentReport,
  FinishInput,
  FinishResult,
  HostProbe,
  InstallResult,
  PrepareResult,
  TestResult,
} from "../types";
import { KeyportError } from "./apiError";

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

function prepareResult(name: string, host: string, user: string, port: number, reused: boolean): PrepareResult {
  const paths = pathInfo();
  const keyPath = `${paths.sshDir}${paths.sep}id_ed25519_${name}`;
  const configBlock = `Host ${name}
    HostName ${host}
    User ${user}
${port === 22 ? "" : `    Port ${port}\n`}    IdentityFile ~/.ssh/id_ed25519_${name}
    IdentitiesOnly yes
    BatchMode yes
    ServerAliveInterval 30
`;
  return {
    alias: name,
    host,
    user,
    port,
    keyPath,
    publicKey: `ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIExampleKeyportDemoFingerprint keyport:${name}`,
    fingerprint: MOCK_FINGERPRINT,
    algorithm: "ssh-ed25519",
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

  async prepareConnection(input: {
    name: string;
    host: string;
    user: string;
    port: number;
    reuseExistingKey: boolean;
  }): Promise<PrepareResult> {
    await delay(1150);
    const existed = preparedNames.has(input.name);
    if (existed && !input.reuseExistingKey) {
      throw new KeyportError(
        "KEY_EXISTS",
        "同名连接密钥已经存在，Keyport 不会覆盖它。",
        "选择“复用现有密钥”继续，或换一个连接名称重新生成。",
        `${pathInfo().sshDir}${pathInfo().sep}id_ed25519_${input.name}`,
      );
    }
    preparedNames.add(input.name);
    return prepareResult(
      input.name,
      input.host,
      input.user,
      input.port,
      existed,
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
      keyPath: `${pathInfo().sshDir}${pathInfo().sep}id_ed25519_${input.connection.name}`,
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
};
