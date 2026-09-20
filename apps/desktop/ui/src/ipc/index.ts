/* IPC entry point: picks the transport and re-exports the typed command
   surface. Views import from here only (goal §8). */
export * from "./types.ts";
export * from "./commands.ts";
export { getTransport, setMockTransport } from "./transport.ts";
export { createMockTransport } from "./mock.ts";
