# 06 — Browser Executor v1

Status: Normative

## 6.1 Goal

Provide reliable structured web automation without making the browser worker a privileged authority.

## 6.2 Default engine

Playwright is the v1 default browser-semantic engine.

## 6.3 Process boundary

Browser worker SHOULD run outside the privileged policy core.

It MUST NOT own:

- organization policy;
- master secret store;
- approval authority;
- tenant-wide credentials.

## 6.4 Browser operations

V1 browser contract SHOULD support:

- navigate;
- inspect/read;
- locate;
- click;
- fill;
- select;
- check/uncheck;
- upload;
- download;
- wait for state;
- screenshot/selective snapshot;
- trace in controlled modes;
- tab/window management.

## 6.5 Locator strategy

Preference:

1. stable test/data IDs when first-party app;
2. accessibility role/name;
3. label/text with stable context;
4. CSS/XPath only when needed;
5. vision fallback outside browser worker contract.

Selectors SHOULD be resilient to layout shifts.

## 6.6 Authentication

Authenticated profile use MUST be explicit.

Supported modes MAY include:

- isolated managed browser profile;
- user-selected existing profile;
- connector/API instead of profile automation.

Attaching to an existing profile MUST be policy-controlled.

## 6.7 Downloads

Downloads MUST:

- land in controlled workspace;
- record filename, MIME, size, hash;
- pass safe-file handling policy before further execution.

## 6.8 Uploads

Upload action MUST identify exact local artifact/file ref.

Arbitrary filesystem browse MUST NOT be implicit.

## 6.9 External side effects

Clicking a UI element that sends/publishes/submits MUST still correspond to a normalized action already authorized.

## 6.10 Cross-origin and popups

Unexpected origin changes SHOULD be treated as state change requiring validation.

Sensitive workflows MAY allowlist origins.

## 6.11 Browser prompt injection

Page content MUST remain untrusted.

Browser worker MUST NOT interpret page instructions as policy or tool authority.

## 6.12 Tracing

Test/staging SHOULD retain Playwright trace for failures.

Production trace retention MUST follow evidence/privacy policy.

Continuous full-session capture is not default.

## 6.13 Verification

Browser actions SHOULD verify post-state through:

- DOM state;
- network/API state;
- URL/origin;
- durable remote record where possible.

Visual confirmation alone is weaker.

## 6.14 Failure categories

Map browser failures to canonical taxonomy:

- BROWSER_SELECTOR
- BROWSER_STATE
- AUTH_SESSION
- NETWORK
- AMBIGUOUS_STATE
- etc.

## 6.15 Tests

V1 browser executor MUST have fixtures for:

- dynamic content;
- delayed rendering;
- popup/tab;
- download/upload;
- stale locator;
- login expiry;
- submit timeout with ambiguous result;
- malicious prompt-injection text;
- cancel mid-run.
