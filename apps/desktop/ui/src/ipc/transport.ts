/* Transport selection (goal §8): the packaged app uses the Tauri IPC;
   `?devmock` loads the mock transport implementing the same interface.
   The mock is never wired in production paths (no query param, no
   fixture). Views never call invoke() directly — they import from
   ipc/commands.ts, which routes through this module. */
import { invoke } from "@tauri-apps/api/core";

export interface Transport {
  invoke<T>(command: string, args?: Record<string, unknown>): Promise<T>;
}

const tauriTransport: Transport = {
  async invoke<T>(command: string, args?: Record<string, unknown>): Promise<T> {
    return invoke<T>(command, args);
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
  return tauriTransport;
}
