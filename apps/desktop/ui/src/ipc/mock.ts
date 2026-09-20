/* Devmock transport (goal §8): implements the same command surface as
   commands.ts with fixture data. Loaded ONLY via ?devmock; the packaged
   app never contains a query parameter, so the fixture is unreachable
   in production. */
import { setMockTransport, type Transport } from "./transport.ts";

export const MOCK_PROJECT_ID = "devmock-printup";

const ago = (mins: number) => new Date(Date.now() - mins * 60000).toISOString();

export function createMockTransport(): Transport {
  return {
    async invoke<T>(command: string): Promise<T> {
      await new Promise((r) => setTimeout(r, 40));
      switch (command) {
        case "project_list_recent":
          return [
            {
              project_id: MOCK_PROJECT_ID,
              display_name: "PrintUp",
              primary_root: "~/dev/printup",
              environment_id: "env-devmock",
              detected_source: "Node",
              capabilities: ["project_open_folder", "files_read", "files_write", "git_read"],
              is_repository: true,
              health: "available",
              last_opened_at: ago(2),
            },
          ] as T;
        case "get_operations_snapshot":
          return {
            connection: "connected",
            execution: "unavailable",
            queue: [],
            progress: null,
            pending_approvals: null,
            exceptions: null,
            evidence: null,
            economics: null,
            permissions: null,
          } as T;
        default:
          throw new Error(`devmock: unimplemented command ${command}`);
      }
    },
  };
}

export function installMockTransport(): void {
  setMockTransport(createMockTransport());
}

