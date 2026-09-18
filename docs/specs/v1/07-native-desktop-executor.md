# 07 — Native Desktop Executor v1

Status: Normative

## 7.1 Goal

Operate macOS and Windows applications through semantic accessibility/UI automation first, while keeping upstream engines replaceable.

## 7.2 V1 upstream

Cua Driver is the initial upstream native executor behind a Lumi-owned DesktopDriver interface.

Workflow definitions MUST NOT call Cua tool names directly.

## 7.3 Supported v1 platforms

First-class:

- macOS
- Windows

Linux desktop MAY remain experimental/out of v1 product certification.

## 7.4 Semantic targets

Preferred target description:

- application;
- window;
- role/control type;
- accessible name;
- optional stable property/value.

Coordinates MUST NOT be durable workflow identity.

## 7.5 Platform semantics

macOS path SHOULD prioritize Accessibility / AX semantics.

Windows path SHOULD prioritize UI Automation.

Vision/coordinates are fallback when semantic target unavailable.

## 7.6 Permissions

Runtime MUST detect required OS permissions.

It MUST NOT silently attempt privilege escalation or permission granting.

Permission onboarding belongs to desktop UX.

## 7.7 Session boundaries

Executor MUST detect unsupported states such as:

- locked session;
- disconnected desktop;
- UAC secure desktop;
- unavailable app window;
- permission revoked.

It MUST fail explicitly.

## 7.8 Input synthesis

Global keyboard/mouse input is high-impact.

It MUST be bounded by current authorized action and target context.

## 7.9 Screenshot capture

Screenshots SHOULD be selective.

Capture MUST obey privacy/evidence policy.

Sensitive regions SHOULD support masking/redaction where feasible.

## 7.10 Upstream pinning

Bundled upstream binary MUST be:

- exact version;
- checksum verified;
- license recorded;
- update owner recorded.

Runtime MUST NOT download "latest" blindly.

## 7.11 Direct driver strategy

Direct Lumi macOS/Windows drivers MAY replace or supplement upstream when measured value justifies:

- reliability;
- security hooks;
- latency;
- evidence;
- background execution;
- upstream independence.

## 7.12 Fork policy

Fork upstream only under criteria in AGENTS.md/ADR.

A fork MUST have:

- explicit reason;
- patch queue;
- upstream sync strategy;
- exit plan.

## 7.13 Verification

Native action SHOULD verify semantic application state after execution.

For save/submit operations, verify durable outcome where possible.

## 7.14 Tests

V1 MUST test:

- permission missing/revoked;
- app/window not found;
- stale semantic target;
- modal dialog;
- wrong-window prevention;
- cancel;
- ambiguous save;
- vision fallback approval;
- macOS and Windows deterministic fixtures.


## 7.15 Canonical outcomes

Native executor MUST return one of:

- DELIVERED;
- REFUSED;
- NO_EFFECT;
- AMBIGUOUS;
- ERROR;
- CANCELLED.

An OS/API call returning success is not sufficient evidence of DELIVERED.

DELIVERED requires the executor's declared effect oracle to observe the intended executor-level state change.

A known unsupported route SHOULD return a stable REFUSED code before mutation instead of falling through to a riskier path.

## 7.16 Certification matrix

Native support claims MUST be backed by an explicit certification matrix containing at least:

- OS and version;
- app and version/toolkit;
- runtime/driver version;
- action;
- semantic vs coordinate addressing;
- foreground/background mode;
- executor route;
- required OS permissions;
- expected outcome;
- effect oracle;
- observed outcome.

Do not infer support for an untested app/toolkit/version simply because the underlying OS API exists.

## 7.17 Collateral-effect oracles

Where observable, background/native evals SHOULD verify:

- active/focused application remains correct;
- z-order does not change unexpectedly;
- physical pointer/cursor remains preserved when promised;
- input does not leak to another app/window;
- wrong account/window/resource is not mutated;
- REFUSED causes no process launch, focus change, or global input;
- clipboard/files/network do not change outside the normalized action.

## 7.18 Runtime generation and session handles

Desktop executor sessions/handles MUST be bound to an execution environment runtime generation.

After driver/runtime restart, rebind, or permission-host change:

- stale handles MUST fail closed;
- cached target identity MUST be re-resolved;
- protected grants MUST be revalidated where necessary.

## 7.19 Existing authenticated sessions

Attaching to an existing authenticated browser/application profile is a protected capability.

When this path is supported, authorization SHOULD bind to concrete context such as:

- profile/account;
- process/runtime generation;
- application;
- allowed origins/workspaces.

Before a material mutation, revalidate mutable target context such as current origin, active account/tenant, selected customer, file identity, or target window.

## 7.20 No silent global-input fallback

The executor MUST NOT silently fall back from semantic/app-scoped/background delivery to unrestricted global keyboard/mouse input.

Such escalation requires an explicitly allowed execution preference plus fresh policy evaluation.

## 7.21 Additional tests

V1 MUST additionally test:

- exact REFUSED with no collateral side effects;
- OS API success but missing effect -> NO_EFFECT;
- unknown effect after interruption -> AMBIGUOUS;
- background delivery preserves focus/cursor where promised;
- input does not leak to another app/window;
- stale generation handle rejected;
- authenticated-profile target changes before mutation -> block/re-authorize;
- global-input fallback denied unless explicitly authorized.
