# 33. Browser and Chrome environments

Status: **Browser read path implemented; explicit Chrome session attach/read path partial**
Date: 2026-09-21

## Implemented

- Project-scoped public HTTPS browser reading uses the existing worker protocol.
- Origins are normalized and persisted per Project.
- The worker runs headless, isolated, JavaScript-disabled, without profiles,
  cookies, downloads or mutating requests.
- DNS resolution is pinned per request and private/local addresses are refused.
- Browser observations are labeled untrusted and cannot grant capabilities.
- Worker bytes are checked against the build’s reviewed source before launch.

## Chrome session path implemented in this slice

The desktop Settings surface can attach one user-selected Chrome PID/window
through the reviewed Cua Driver `existing-profile` grant. The resulting binding
is project-scoped, carries an explicit approved-origin ceiling, is held only in
runtime memory, and can be revoked. Tasks can read the selected tab or navigate
within that origin ceiling. Page content remains untrusted.

## Remaining work

Authenticated Chrome requires an explicit selected session/profile/tab, visible
ownership, origin/account scope, expiry, revocation and a reviewed extension or
CDP/native-host boundary. Existing browser profiles must never be attached by
guessing a loopback endpoint or copying the profile.

Interactive browser actions, uploads, downloads and browser-side mutations need
separate capability and approval contracts. The read adapter must not be widened
implicitly to provide them.

## Acceptance

Real Chromium read canaries must cover approved origins, redirects, DNS rebinding,
private addresses, prompt-injection content, stale links, and bounded observations.
Chrome acceptance requires a real selected session and cleanup proof; mocks do not
establish authenticated Chrome support.
