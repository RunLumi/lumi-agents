# Lumi Playwright Browser Worker

Unprivileged browser automation worker (spec 06). Speaks line-delimited
JSON over stdio; `lumi-browser` owns process lifecycle and correlation.
Local policy and the orchestrator authorize the operation before dispatch.

## Boundary (spec 06 §6.3)

The worker holds **no** policy, **no** secret store, **no** approval
authority, and **no** tenant credentials. It receives an already-authorized,
closed set of operations and returns normalized observations/failures.
Page content is untrusted data (§6.11): returned verbatim as observation
payloads, never interpreted as instructions — the operation set is closed,
so page text structurally cannot become an operation.

## Protocol

```text
request:  {"id":"r1","op":"navigate","url":"https://…","session":{"headless":true}}
response: {"id":"r1","ok":true,"result":{"url":"…","status":200,"title":"…"}}
          {"id":"r1","ok":false,"error":{"category":"BROWSER_SELECTOR","message":"…","native_code":"TimeoutError"}}
```

Operations (§6.4): `navigate`, `read`, `click`, `fill`, `select`, `check`,
`upload`, `download`, `wait_for`, `screenshot`, `submit`, `stop_trace`, `close`.

Locator preference (§6.5): `test_id` → `role`(+`name`) → `label` → `text` →
`css` (last resort).

Failure categories map onto the canonical taxonomy (§6.14). A submit whose
outcome is not observed within its timeout returns an **AMBIGUOUS_STATE**
result — the runtime verifies external state before any retry (spec 02 §2.7).

## Install (local dev)

```bash
cd workers/playwright
npm install          # pinned playwright 1.49.1
npx playwright install chromium
```

## E2E (local, not part of CI)

CI runs the Rust-side fake-worker suite, a Rust round-trip check of the shared
wire fixtures, and tests of the actual JavaScript handler with an injected
browser adapter. The latter tests do not launch Chromium or prove live-system
behavior. Run them without installing browser packages:

```sh
node --experimental-vm-modules --test tests/worker_protocol.test.js
```

With real browsers installed, the same protocol can be exercised
end-to-end:

```bash
node worker.js   # then feed requests on stdin, e.g.:
{"id":"1","op":"navigate","url":"file:///…/fixtures/portal.html"}
{"id":"2","op":"click","target":{"strategy":"test_id","test_id":"submit"}}
```

Tracing (§6.12) is off by default. It requires the first request to carry
`session.trace: "on_demand"` and an explicit `trace_path` on navigation. Session
changes after the first request, legacy nested `params`, unsupported profiles
and unknown fields are refused before browser mutation. Downloads take a direct
`trigger` selector; submits take `expect: {"selector": ...}`.

Rust defaults to a headless isolated session with capture off. Actual browser
installation, authenticated sessions and representative customer runs remain
separate validation steps; none is established by the handler tests.
