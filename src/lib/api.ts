import { invoke } from "@tauri-apps/api/core";
import { writeText } from "@tauri-apps/plugin-clipboard-manager";

import type {
  ConnectionInput,
  EnvironmentReport,
  FinishInput,
  FinishResult,
  HostProbe,
  InstallResult,
  PrepareResult,
  TestResult,
} from "../types";
import { normalizeError } from "./apiError";
import { mockApi } from "./mock";

export const isTauriRuntime =
  typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

// The mock backend is strictly a browser-preview tool. A production Tauri build
// without the injected IPC bridge must fail visibly instead of pretending that
// key or server operations succeeded.
export const isDemo =
  !isTauriRuntime &&
  (import.meta.env.DEV || import.meta.env.VITE_KEYPORT_DEMO === "1");

async function realCommand<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(command, args);
  } catch (error) {
    throw normalizeError(error);
  }
}

export const api = {
  async diagnose(): Promise<EnvironmentReport> {
    if (isDemo) {
      return mockApi.diagnose();
    }
    return realCommand<EnvironmentReport>("diagnose");
  },

  async installOpenSsh(): Promise<InstallResult> {
    if (isDemo) {
      return mockApi.installOpenSsh();
    }
    return realCommand<InstallResult>("install_openssh");
  },

  async prepareConnection(input: ConnectionInput): Promise<PrepareResult> {
    if (isDemo) {
      return mockApi.prepareConnection(input);
    }
    return realCommand<PrepareResult>("prepare_connection", { input });
  },

  async probeHost(host: string, port: number): Promise<HostProbe> {
    if (isDemo) {
      return mockApi.probeHost({ host, port });
    }
    return realCommand<HostProbe>("probe_host", { input: { host, port } });
  },

  async finishSetup(input: FinishInput): Promise<FinishResult> {
    if (isDemo) {
      return mockApi.finishSetup(input);
    }
    return realCommand<FinishResult>("finish_setup", { input });
  },

  async testConnection(name: string): Promise<TestResult> {
    if (isDemo) {
      return mockApi.testConnection({ name });
    }
    return realCommand<TestResult>("test_connection", { input: { name } });
  },

  async openSshFolder(): Promise<void> {
    if (isDemo) {
      return mockApi.openSshFolder();
    }
    return realCommand<void>("open_ssh_folder");
  },
};

export async function copyText(value: string): Promise<void> {
  if (!isDemo) {
    await writeText(value);
    return;
  }
  try {
    await navigator.clipboard.writeText(value);
  } catch {
    const textarea = document.createElement("textarea");
    textarea.value = value;
    textarea.style.position = "fixed";
    textarea.style.opacity = "0";
    document.body.appendChild(textarea);
    textarea.select();
    document.execCommand("copy");
    textarea.remove();
  }
}

export { KeyportError, normalizeError } from "./apiError";
