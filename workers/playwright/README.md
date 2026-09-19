# Lumi Playwright Browser Worker

Unprivileged browser automation worker (spec 06). Speaks line-delimited
JSON over stdio; the Rust policy core (`lumi-browser` crate) owns the
process lifecycle, authorization, and correlation.

## Boundary (spec 06 §6.3)

The worker holds **no** policy, **no** secret store, **no** approval
authority, and **no** tenant credentials. It receives an already-authorized,
closed set of operations and returns normalized observations/failures.
Page content is untrusted data (§6.11): returned verbatim as observation
payloads, never interpreted as instructions — the operation set is closed,
so page text structurally cannot become an operation.

## Protocol

```text
request:  {"id":"r1","op":"navigate","params":{"url":"https://…"}}
response: {"id":"r1","ok":true,"result":{"url":"…","status":200,"title":"…"}}
          {"id":"r1","ok":false,"error":{"category":"BROWSER_SELECTOR","message":"…","native_code":"TimeoutError"}}
```

Operations (§6.4): `navigate`, `read`, `click`, `fill`, `select`, `check`,
`upload`, `download`, `wait`, `screenshot`, `submit`, `stop_trace`, `close`.

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

CI runs the Rust-side contract suite against a deterministic fake worker.
With real browsers installed, the same protocol can be exercised
end-to-end:

```bash
node worker.js   # then feed requests on stdin, e.g.:
{"id":"1","op":"navigate","url":"file:///…/fixtures/portal.html"}
{"id":"2","op":"click","target":{"strategy":"test_id","test_id":"submit"}}
```

Tracing (§6.12) is off by default and only starts when a request carries
`trace: true` — staging/test convenience, never default evidence.
