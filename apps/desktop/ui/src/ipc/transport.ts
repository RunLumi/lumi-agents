/* Transport selection (goal §8): the packaged app uses the Tauri IPC;
   `?devmock` loads the mock transport implementing the same interface.
   The mock is never wired in production paths (no query param, no
   fixture). Views never call invoke() directly — they import from
   ipc/commands.ts, which routes through this module. */

export interface Transport {
  invoke<T>(command: string, args?: Record<string, unknown>): Promise<T>;
}

const tauriTransport: Transport = {
  async invoke<T>(command: string, args?: Record<string, unknown>): Promise<T> {
    const tauri = window.__TAURI__;
    if (!tauri?.core) throw new Error("runtime unavailable");
    return tauri.core.invoke<T>(command, args);
  },
};

const nullTransport: Transport = {
  invoke<T = unknown>(): Promise<T> {
    return Promise.reject(new Error("runtime unavailable"));
  },
};

let mock: Transport | null = null;

/** Registers the devmock transport (goal §8). Called only by the
    ?devmock dev harness; the packaged app never reaches fixture data. */
export function setMockTransport(t: Transport | null): void {
  mock = t;
}

export function getTransport(): Transport {
  if (mock) return mock;
  if (typeof window.__TAURI__ !== "undefined" && window.__TAURI__.core) {
    return tauriTransport;
  }
  return nullTransport;
}

declare global {
  interface Window {
    __TAURI__?: {
      core: { invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> };
    };
    __lumiMock?: Transport;
  }
}
