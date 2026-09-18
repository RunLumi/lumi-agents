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
