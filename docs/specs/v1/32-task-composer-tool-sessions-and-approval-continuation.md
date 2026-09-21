# 32. Task composer, tool sessions and approval continuation

Status: **Normative product contract; not a claim of implementation**
Date: **2026-09-21**
Related: [06](06-browser-executor.md), [07](07-native-desktop-executor.md), [13](13-scheduler-background-triggers.md), [23](23-user-experience-handoff.md), [26](26-project-workspace-folder-as-project.md), [27](27-global-and-project-automations.md), [29](29-project-connections-secrets.md), [31](31-delegated-work-product-contract.md).

## 32.1 Outcome and boundaries

A user opens a Project, describes a job in ordinary language, selects any required execution environment, and starts or schedules work. The agent receives actual callable tools, observes their results, and returns an inspectable deliverable or a precise exception. Mentions are discoverable shortcuts, not the only way to grant scoped access.

This spec specializes the existing task, executor, approval and automation contracts. It introduces no second planner, scheduler, permission authority or secret store. A UI mock, declared capability, transport response or model answer alone is not proof that this journey works.

## 32.2 Composer contract

The task composer MUST be a multiline field with a visible label, preserved paragraphs and a bounded auto-growing height. Enter inserts a newline. Cmd+Enter on macOS or Ctrl+Enter on Windows submits once. IME composition, key repetition, double clicks and pending submissions MUST NOT create duplicate work. A submission failure preserves the user's text and a successfully created Task ID.

Run task is the primary action. Save task draft and Schedule are secondary actions with distinct results. Saving never implies execution; scheduling never implies unattended authorization. A project-scoped local draft is restored only to its owning project, and a draft storage error is visible. Secret values do not belong in drafts. Attachments and selected tool-session references are shown separately from prose.

The Tools button and @ picker MUST read the same backend capability snapshot. The picker supports keyboard navigation, Escape, focus restoration, descriptive unavailable states and accessible labels. Mention completion does not activate inside an email address or code token. An unavailable explicit mention is a blocker, not permission to substitute a different executor.

Switching Projects while loading, submitting or editing MUST NOT mix goals, sessions, files, tasks, permissions or results. Older asynchronous responses cannot overwrite a newer Project selection. The UI MUST remain usable at the declared minimum window size in English and Vietnamese.

## 32.3 Runtime capability and session resolution

Every advertised capability identifies its execution environment, readiness, permission/setup blocker and supported operations. Readiness is observed, not inferred from a package name. Use these user-facing distinctions:

| Selection | Meaning |
|---|---|
| Files | Explicit Project workspace paths, not the home directory |
| Browser | Lumi-owned isolated browser session; public reading and interactive sessions have separate capability sets |
| Chrome | Explicitly attached existing Chrome window/tab/profile with separate authorization |
| Computer | An exact authorized native application/window on the local device |
| Commands | Explicit local shell capability with its real isolation limits disclosed |

A Task request carries intent and opaque session/attachment references. The host resolves the references against the current principal, Project and ExecutionEnvironment. Frontend booleans, model output, webpage content and repository files cannot attest readiness or grant authority.

A session reference MUST bind Project, environment, executor instance, runtime generation, target identity and expiry. Stale references, cross-project references, lost ownership, a changed browser profile/window/account or a restarted process fail closed. Reconnecting does not restore expired authority. Task-to-Automation conversion strips ephemeral sessions and demands fresh unattended admission.

Natural-language requests MAY propose using additional tools, but the host must obtain the missing scoped authorization before exposing or invoking them. Tool selection does not authorize every business effect reachable through that tool.

## 32.4 Model conversation and tool results

The configured host system instructions MUST reach the model. Every executed tool call has a corresponding result with its original provider call ID. Assistant tool calls and results remain in chronological order, including multiple calls in one turn, errors and approval continuation. Do not fabricate a shared `last` call ID or append all historical results after later assistant turns.

Return bounded useful observations: page title/URL/text/semantic refs, application state, file content or verified artifact metadata. Preserve untrusted provenance. Raw credentials, full screenshots, binary payloads and arbitrary action arguments are not default observation text. Large results need explicit truncation or references, never silent omission represented as complete.

A refusal, truncated model response, provider error, cancelled run, missing required observation or failed verifier MUST NOT be recorded as successful completion. The model's final prose is not independent effect verification. A durable final result carries the answer or failure, Task/Run identity and evidence/artifact references.

## 32.5 Approval and continuation

Before exposing a consequential proposal for review, persist its normalized intent. Show the exact target/account/destination, operation, risk, parameters, expected effect, reversibility and verification requirement. Distinguish user intent from untrusted page-provided descriptions.

Only the authenticated trusted UI may submit Approve once or Reject against the pending request ID and exact material digest. The backend binds approval to principal, tenant, Task, Run, workflow when present, resource/target, parameters and expiry. Approval cannot widen a policy ceiling, satisfy missing OS permissions, enable a different executor or authorize a changed action.

The waiting run does not execute further actions or claim completion. On approval, revalidate mutable target/session state immediately before the effect, resume the exact proposed action once, then return its observed result under the original model call ID. Material changes require a fresh proposal and approval. A rejected, stale, mismatched, replayed or expired decision produces no effect.

The UI reads the approval inbox without waiting behind the execution mutex. Emergency stop, revocation, cancellation, deadline expiry and app shutdown interrupt approval waiting. A restart without a supported persisted continuation marks the approval/run interrupted for review; it MUST NOT automatically approve, silently rerun or consume an old approval for a new proposal.

## 32.6 Browser use

Browser sessions follow Spec 06. A public read-only mode MUST NOT advertise authenticated or interactive capabilities. An interactive mode declares allowed origins, sessions/accounts, downloads, uploads, script behavior and network limits separately. A normal GET is not universally side-effect-free; navigation remains scoped and audited.

Use DOM/accessibility locators or current semantic references before coordinates. Bind references to the exact session, tab, frame/document and snapshot generation. Ambiguous or stale targets trigger re-observation, not a guessed first match. JavaScript evaluation, arbitrary CDP commands, local-file URLs and secret extraction are not generic agent escape hatches.

Clicks, fills and navigation can have business effects. Normalize the effect and obtain the required scoped approval; do not label every click READ. Post-state verification reads the actual application or artifact after execution. Downloads are confined to approved workspace paths, reject unsafe filenames/path escapes and do not overwrite silently. Uploads bind the exact approved file version, destination and action digest.

Attaching personal Chrome is a separate user decision. Never copy a personal profile, kill/restart the user's browser, edit profile preferences, enable remote debugging or connect to a random loopback endpoint as a hidden setup step. Explain broad debugging authority and validate endpoint/process ownership. The selected account/window/tab and visible takeover state remain inspectable. Host controls are not a claim of an OS network sandbox.

## 32.7 Native computer use

Native workflows remain expressed through Lumi-owned contracts in Spec 07. The initial Cua adapter is replaceable. An installed binary is accepted only through a reviewed version/platform/checksum pin and supported host-permission identity. Unknown protocol versions, changed binary bytes and unsupported OS/session states fail closed.

The host selects one exact application/window and receives a fresh semantic snapshot before mutation. Snapshot-bound tokens stay private to the driver adapter. Revalidate target identity and expected pre-state immediately before acting; do not reuse a token after a new snapshot invalidates it. Return actual post-state or an explicit ambiguous/refused outcome, never a constant fabricated observation identifier.

Accessibility and screen-recording grants, screen lock, disconnected sessions and secure desktops are observed, not hardcoded. Screenshots and provider image egress are opt-in and bounded. No continuous screen recording is implied. Background semantic delivery MUST NOT silently become global pointer input, foreground activation or broader desktop capture. An unsupported safe path yields a precise blocker and recovery action.

Stop and take over ends remaining agent authority before human interaction. Do not promise resumable pause when the implementation only supports cancellation. Already-delivered effects remain visible and are not undone by stopping.

## 32.8 Automations and credentials

Global Automations and Project Automations are views of the same authoritative definitions and occurrence history, per Spec 27. Global navigation works without a Project already open. It aggregates only authorized registered Projects, identifies each owning Project/device, and never creates a global super-project or independent execution queue.

Creation, editing, Run now, pause, deletion, schedule previews and history use shared commands and validation. The scheduler creates the same project-bound Tasks as foreground work. Schedules use IANA timezones and explicit DST/missed-run/overlap rules. Persist occurrence admission before execution; reconcile uncertainty rather than replay possible effects.

Unattended authority is bounded by device, workflow/definition revision, capability/resource scope, action/model budgets, deadline and expiry. Enforce the deadline in the loop and executor, not merely in a display field. Revocation applies at the next execution gate and interrupts waiting approvals. Pause prevents future admissions; it does not reverse effects already delivered.

Local-only scheduling requires the local application/device and necessary sessions to be available. No cloud or OS-daemon execution is implied. Missing provider credentials, unavailable sessions, stopped runtime and expired authorization are explicit states that do not starve unrelated eligible automations.

Provider credential persistence is opt-in and uses the existing OS-backed secret broker. Store a consistent provider/endpoint/model/key tuple or integrity-bound equivalent so an endpoint change cannot redirect a saved key. Restore never grants new tool/automation authority. Forget reports revocation failures instead of claiming removal. Debug, prompt, audit and project records contain opaque references, not actual API keys. Reject endpoint credentials and unsupported plaintext remote endpoints.

## 32.9 Attachments and durable outputs

A file attachment binds the user's selected Project-relative file and its checked content identity. Validate containment, symlinks/junctions, size, type and current version at use time. A folder attachment does not imply recursive whole-disk access. A changed attachment is revalidated or refused explicitly. Cross-project references and model-invented paths cannot widen scope.

Preserve final answers, failures, evidence and created artifact references under the actual Task/Run. The user can reopen the result after navigating or restarting. Never display a successful output from another Project or an older run as the current result. Saving a draft document and publishing/sending it remain distinct effects.

## 32.10 Acceptance and evidence matrix

The implementation and review MUST attach revision-bound evidence for these journeys:

| Area | Required proof |
|---|---|
| Composer | Multiline/paste/IME/shortcut; failed-create and failed-run retry; no duplicate task; project switch race; keyboard/minimum-size EN/VI |
| Capability resolution | Ready/missing/denied/stale states; exact explicit mentions; no unavailable-tool shell fallback; cross-project session refusal |
| Model loop | Actual configured instructions; multi-call chronological results with original IDs; approval continuation IDs; refusal/truncation/cancel not success |
| Approvals | Exact digest/target shown; accept once; reject/expiry/replay/wrong Project or digest; stop while waiting; interrupted restart |
| Browser | Real browser against controlled pages; navigate/observe/interact/verify; stale/ambiguous locator; unapproved origin; lost session; bounded upload/download; prompt injection treated as data |
| Chrome | Real selected-profile/tab attachment, account/window identity and cleanup; no hidden debugging/profile change; no claim from a mocked CDP response |
| Computer | Real supported macOS and Windows fixture apps; permission denial, lock/disconnect, changed window/generation, no focus steal, effect readback and stop |
| Automations | Same record through global/project views; durable restart; timezone/DST; missed/duplicate/overlap; expired lease; blocked-provider visibility; no post-revocation effect |
| Credentials | Opt-in broker roundtrip and deletion; wrong/corrupt/missing key; no secret in metadata/debug/audit; OS backend tested separately from in-memory fixtures |
| Artifacts | Real output exists with matching source/Task/Run; safe filename/path; reopen after restart; no model-only success |

Separate evidence levels: source review; unit/contract tests; fixture-backed UI; real browser; real native OS; live provider end-to-end; signed distribution; sustained customer canary. Passing one level does not assert another. The release decision remains governed by Specs 21/22 and the repository's readiness record.

## 32.11 Reviewed upstream boundaries

Reference reviewed on 2026-09-21: Cua repository commit `9bbfa7dd3e27ca7f1861ede70aaca390174493f9`, driver skill version `0.28.2`.

- [MCP protocol and skills](https://github.com/trycua/cua/blob/9bbfa7dd3e27ca7f1861ede70aaca390174493f9/libs/cua-driver/docs/mcp-protocol-and-skills.md): use a negotiated persistent stdio connection; protocol discovery does not grant permissions.
- [Native skill contract](https://github.com/trycua/cua/blob/9bbfa7dd3e27ca7f1861ede70aaca390174493f9/libs/cua-driver/rust/Skills/cua-driver/SKILL.md): fresh snapshot, exact target, delivery/verification distinction and no silent foreground escalation.
- [Browser contract](https://github.com/trycua/cua/blob/9bbfa7dd3e27ca7f1861ede70aaca390174493f9/libs/cua-driver/rust/Skills/cua-driver/BROWSER.md): explicit existing-profile preparation and exact session/tab/ref bindings.

Transfer these boundary principles and verify actual schemas. Do not copy upstream's unrestricted mode, global capture defaults or implicit screenshots into Lumi's authorization model. These references do not attest that a pinned binary was installed or a real OS workflow passed.
