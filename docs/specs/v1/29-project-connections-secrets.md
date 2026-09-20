# 29 — Project Connections, Credentials & Secret Isolation v1

Status: Normative security/product contract; not an implementation claim.

Specializes [17 — Security](17-security-privacy-secrets.md) for
[28 — Project extensions](28-project-skills-plugins.md) and
[13 — Automations](13-scheduler-background-triggers.md).

## 29.1 Decision

**Credentials belong to a project logically; secret values do not belong in its
working directory.** Store portable connection requirements in the project and
keep secret material behind the trusted runtime's protected credential broker.

A plaintext `credentials.json` plus `.gitignore` is not an acceptable default:
Git exclusion does not restrict filesystem reads, shell access, plugins, backups,
archive exports, screenshots, or already-tracked files. A copied project must not
become an authenticated copy of a customer's account.

Do not implement a second secret store when `lumi-secrets` can be extended.
Existing reference validation/keyring plumbing is substrate, not proof that the
project/caller/destination authorization requirements below are implemented.

## 29.2 Three distinct records

| Record | Location | Contains |
| --- | --- | --- |
| Connection requirement | `.lumi/connections.json`, shareable | Logical slot, provider/auth kind, requested scopes, intended plugin/connector |
| Connection binding | Trusted runtime DB; optional ignored `.lumi/local/connections.json` projection | Opaque connection reference and non-secret readiness metadata |
| Credential material | OS protected store or approved enterprise vault | Access/refresh token, API key, client secret or other secret value |

The optional local projection is convenience metadata, never a bearer capability
or an authorization database. A user-editable file cannot change which connection
the broker authorizes. Treat account identifiers/binding metadata as private even
when they are not authentication secrets.

Example portable requirement, not a grant or actual account binding:

```json
{
  "version": 1,
  "requirements": [{
    "slot": "ads-read-session",
    "kind": "browser_session",
    "purpose": "Read approved Ads account metrics",
    "external_writes": "deny"
  }]
}
```

Example optional local projection:

```json
{
  "version": 1,
  "bindings": {
    "ads-read-session": {"connection_ref": "conn_<opaque-id>"}
  }
}
```

Broker lookup MUST still authenticate the caller and authorize the registered
project/environment/account. Possessing or guessing a reference is insufficient.

## 29.3 Protected storage

Default adapters: macOS Keychain, Windows user-scoped protected credential
storage, or an approved enterprise secret manager. Windows machine-wide DPAPI
scope is not the default for user secrets. Use supported OS APIs and audited
libraries, never custom encryption or a key beside the encrypted credential file.

No plaintext long-lived fallback in the repository, home config, environment
file, plugin package, model prompt, audit log, trace, crash report or fixture.
Backend unavailable/locked -> `BLOCKED_CREDENTIAL_STORE`; user reconnects/unlocks
or chooses an approved backend. An explicitly authorized ephemeral-memory session
may be offered but cannot be advertised as surviving restart or enabling reliable
unattended work. Do not silently downgrade to JSON storage.

Backup/export of credentials is a separate privileged workflow and out of V1
scope. Project export omits credentials/local bindings by default. UI may show
provider/account labels and readiness, never reveal a token to the model.

## 29.4 Broker authorization context

Each binding and resolution is constrained by:

```text
tenant + owner/principal + registered project + execution environment
+ connector/plugin identity and approved version
+ external account/tenant + auth audience/scopes
+ action/operation and destination + expiry/revocation generation
```

The host establishes caller identity from authenticated IPC/session context, not
fields asserted by plugin JSON. The broker rechecks effective permission at use,
including the automation lease when unattended. A global personal/provider account
may be reused only through an explicit project binding allowed by policy; it is
not globally available merely because its backend is installed.

Validate intended account immediately before consequential use. Switching UI
project or browser account never retargets a live task. New bindings, broader
scopes, destination changes, and replacement accounts require review.

## 29.5 Use, not reveal

Prefer brokered operations: the trusted connector attaches authentication to the
approved request after policy validates its normalized business effect. Skills
and models see typed tools and minimized results, not authentication headers.

Where an approved executable genuinely requires a token, issue only the minimum
scoped/short-lived material to its isolated process, not the whole vault. Prefer
private IPC or an approved secret handle. Environment injection is an explicit
compatibility exception in a reviewed sandbox; never parent/global environment
inheritance, command-line arguments, files in the project, or logged subprocess
configuration. Clear material on process exit and exclude it from traces/dumps.

Code receiving a token can misuse that token. Consequently, untrusted executable
plugins MUST NOT receive broadly privileged raw tokens just because declared
capabilities say `read`. Enforce operation/account/destination restrictions in a
broker/proxy or use genuinely restricted provider credentials. If neither can
provide the required boundary, block that unattended integration.

A separate child process or `chmod 600` alone does not isolate hostile code running
as the same OS user. Encryption at rest does not solve runtime misuse. Certify
actual sandbox/process/network boundaries per platform and disclose unsupported
cases instead of promising protection against arbitrary same-user code or a
compromised host.

## 29.6 OAuth, API keys, MCP and browser sessions

OAuth setup belongs to trusted user-interactive UI. Use the provider's supported
flow; validate issuer, redirect, state, resource/audience and PKCE where applicable.
Refresh is broker-owned with serialized rotation and generation checks so parallel
runs cannot overwrite refreshed tokens or resurrect revoked credentials.

MCP authentication and downstream service authentication are distinct. Do not
forward arbitrary provider access tokens to a remote MCP server or accept an
unvalidated remote endpoint supplied by model/plugin output. Validate discovery
URLs/redirects against network/SSRF policy; request only reviewed scopes. Remote
endpoint/tool changes invalidate affected consent as required.

API keys enter through secure connection setup, never a chat request to paste
secrets into SKILL.md, a terminal command, or automation YAML.

Browser connection = approved session/profile handle owned by the environment,
not exported cookies or a browser-profile path in project configuration. Human
login/2FA remains interactive. A broadly authorized browser session can perform
writes; an Ads read-only label alone does not prevent them. Enforce semantic
operations and browser control grants, deny credential extraction/full CDP
shortcuts, and block when the read-only boundary cannot be maintained.

## 29.7 Project lifecycle

Opening/cloning/copying a repository imports requirements, not credentials, grants,
automation activation, or browser sessions. A checked-in/copied Project marker is
not proof of secret ownership. Register the project against trusted local identity
and root information outside the checkout before any binding can resolve.

A legitimate moved folder can retain identity after explicit relink and validation.
A fork/new device requires new connection binding. An isolated worktree created by
the runtime MAY use the parent Project's explicitly allowed connection through the
broker; no token/local credential directory is copied into the worktree.

Removing a project disconnects its grants and reports affected automations. Deleting
shared provider credentials or revoking them upstream is a separate confirmed
operation with impact review. Never delete another project's shared connection
because a plugin/project was removed.

## 29.8 Git hygiene during initialization

On explicit `Initialize Lumi Project` or the first authorized project extension/
automation setup, ensure a root `.gitignore` exists, including for a non-Git
folder. Merely browsing/opening a folder MUST NOT mutate it or run `git init`.

Add an idempotent managed block; preserve existing content, line endings, and
unrelated user changes. Use Project-safe compare-and-swap writes and reject
symlink/junction escapes. A read-only/dirty/conflicting ignore file yields a clear
setup error and no hidden rewrite.

```gitignore
# BEGIN Lumi local-only state
/.lumi/local/
/.lumi/runtime/
/.lumi/cache/
/.lumi/credentials.json
/.lumi/secrets.json
/.lumi/secrets/
/credentials.json
# END Lumi local-only state
```

The credential paths cover accidental/legacy files; they are NOT permission to
create plaintext credentials. Keep `.lumi/skills/`, `.lumi/plugins/`,
`.lumi/automations.yaml`, `.lumi/connections.json` and `extensions.lock.json`
trackable. Safe example files use names such as `credentials.example.json` and
contain placeholders only.

For Git repositories, verify both tracked state and effective ignore rules:
`git ls-files` identifies tracked candidates; `git check-ignore --no-index -v`
checks effective patterns including nested negations. Neither alone proves safety.
Use a hardened Git invocation without invoking repository hooks or arbitrary
filters. Never automatically untrack user files or rewrite Git history.

Tracked/staged secret-bearing files -> block publication/activation involving
those secrets and offer a reviewed remediation. When material was committed,
shared, or exposed, rotate/revoke it first; adding `.gitignore` is not remediation
of already leaked credentials. Support fresh archives and non-Git projects without
pretending that ignore rules apply outside Git.

## 29.9 Defense in depth independent of Git

Protected credential/local-state paths remain excluded from model file tools,
search/indexing/embeddings, watcher payloads, artifact packaging, synchronization,
Git staging, and project export even if ignore rules are removed or negated.
Trusted settings/broker operations use separate audited access.

Shell/MCP/plugin isolation must enforce the same boundary outside ordinary file
APIs; it cannot rely on asking the model not to read a file. Built-in publication
checks refuse secret-bearing staging/export and flag accidental plaintext.
Scanning/redaction are supplementary defenses, not a proof that every possible
secret or encoded copy can be detected.

Do not print discovered values in diagnostics. Show path, classification, and
recovery action. If a secret has entered context/logs, treat it as an incident,
stop further propagation and request rotation; deletion cannot undo exposure.

## 29.10 Runtime lifecycle and automation behavior

Connections expose `NOT_CONNECTED`, `READY`, `EXPIRED`, `LOCKED`, `REVOKED`, or
`NEEDS_REVIEW` with account/scope, last verification, and dependent automations.

At admission and before protected actions, verify current binding, plugin snapshot,
lease and revocation generation. Readiness of a previous run is not current
availability. Sleep, locked keychain, missing login, and unavailable device are
separate blockers. Do not weaken OS store access rules to make schedules run.

Disconnect invalidates local grants immediately, stops new credential issuance,
and blocks dependent automation actions at the next safe gate. Already-issued
external tokens/actions may remain valid/in flight; report this limit and attempt
provider revocation where supported rather than claiming retroactive cancellation.

Run records contain references/generation/purpose only, never token values. Deduplicate
repeated connection alerts across dependent automations. Preserve each occurrence's
blocked record and required closeout evidence without spamming identical login
notifications or looping model calls. Resuming requires a readiness check; no
automatic scope upgrade or unbounded catch-up.

## 29.11 Migration from credentials.json

Detection does not grant permission to ingest credentials into model context.
A trusted host-only migration tool can, after explicit user approval:

1. Identify candidate path/keys without displaying values; check tracked status.
2. Confirm destination project/account, required scopes, and protected backend.
3. Store through broker and verify a safe permitted request.
4. Replace references with non-secret connection metadata.
5. Request explicit removal/quarantine of the old file and necessary rotation.

Never auto-delete originals, claim secure erasure on SSD/backups, or claim `.gitignore`
removed credentials from history. Until safe migration/rotation completes, affected
unattended jobs remain blocked. A reference to another project's connection fails
regardless of a copied local file or project ID.

## 29.12 Acceptance gates

Test with synthetic canary secrets; never real customer values in CI:

- A can use its approved connection; B cannot resolve, enumerate, swap, or inherit it.
- A copied project ID, guessed ref, edited binding file or forged plugin caller does
  not bypass broker authorization.
- Removing/negating `.gitignore` does not expose secrets through read/search/shell/
  plugin/export paths; pre-existing tracked files trigger remediation.
- Initialization is idempotent, preserves unrelated ignore rules, handles non-Git
  and read-only folders, and refuses symlink/negation/stale-write pitfalls.
- Vault locked/unavailable never falls back to plaintext; restart works only with
  the approved backend and current binding.
- Local/browser credentials never appear in prompts, logs, notifications, fixtures,
  exported project archives, or worktree copies.
- Concurrent refresh and revoke cannot resurrect credentials; actual caller,
  project, account, audience, operation and destination are checked.
- Plugin disable/update, connection removal and automation Resume cannot revive
  revoked authority or silently choose another account.
- Broad-token untrusted plugins and unsupported required sandboxes fail closed.
- Legacy migration requires consent and never exposes values to the model.

Implement broker scoping and safe setup first, then project connections UI and
scheduled dependency checks. A custom project-local encrypted vault, credential
sync/export, and universal OAuth provider support are not required for V1.
