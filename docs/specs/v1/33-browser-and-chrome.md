# 33. Browser and Chrome environments

Status: **Browser read path implemented; Chrome session path planned**
Date: 2026-09-21

## Implemented

- Project-scoped public HTTPS browser reading uses the existing worker protocol.
- Origins are normalized and persisted per Project.
- The worker runs headless, isolated, JavaScript-disabled, without profiles,
  cookies, downloads or mutating requests.
- DNS resolution is pinned per request and private/local addresses are refused.
- Browser observations are labeled untrusted and cannot grant capabilities.
- Worker bytes are checked against the build’s reviewed source before launch.

## Planned

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
