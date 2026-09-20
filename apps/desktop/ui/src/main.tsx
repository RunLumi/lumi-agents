import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import App from "./App";
import { installMockTransport } from "./ipc/mock";
import "./styles/index.css";

/* Dev-only harness (?devmock): routes IPC to the in-memory fixture so
   the UI can be developed and visually audited in a plain browser. The
   packaged app never carries this query parameter. */
if (new URLSearchParams(window.location.search).has("devmock")) {
  installMockTransport();
}

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
