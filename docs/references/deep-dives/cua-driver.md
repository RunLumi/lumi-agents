# Deep Dive: Cua Driver

Reviewed: 2026-09-18  
Repository: https://github.com/trycua/cua  
Reviewed commit: `05f29785b508a4441ec3aa06c556a8e8b26c1d71`  
Observed repository license: MIT at repository root and Cua Driver workspace, with optional components carrying other licenses.

## Why this reference matters

Cua Driver is the strongest immediate implementation reference for Lumi's native computer-use layer because it treats desktop automation as a cross-platform capability system with explicit permission modes, resource manifests, structured refusals, session lifecycle, and empirical OS/app/action matrices.

The biggest lesson is that **refusal is a valid, testable result**. A driver should not pretend an action succeeded when the operating system cannot prove safe delivery.

## What Lumi should learn

### 1. Separate permission mode from capability policy

Cua distinguishes a mode such as standard/bounded/unrestricted from the policy ceiling that defines what is actually allowed.

A mode cannot widen managed/user policy.

**Apply to Lumi:** separate:

- policy ceiling;
- autonomy mode;
- workflow manifest;
- temporary grants.

Do not collapse them into one boolean like `autoApprove`.

### 2. Bounded automation should use manifests

Cua bounded mode can constrain tools, applications, browser profiles/origins, and file roots.

**Apply to Lumi:** unattended Workflow mode should run under a reviewed capability manifest.

Example dimensions:

- allowed apps;
- allowed domains/origins;
- file roots;
- connector scopes;
- allowed business actions;
- maximum risk class;
- background/foreground execution;
- time/budget window.

### 3. Existing authenticated browser profiles are a separate trust boundary

Cua treats attachment to an already logged-in browser profile as explicit, scoped authorization. It binds grants to concrete process/profile/origin context and rechecks live origin before mutation.

**Apply to Lumi:** browser session reuse is not a convenience toggle. It is a protected capability.

### 4. Exact refusals beat fake success

Cua's action ledger distinguishes:

- Delivered;
- Refused;
- Gap/unproven.

A refusal is only accepted when the exact refusal code and no-side-effect oracles pass.

**Apply to Lumi ActionResult:**

```text
DELIVERED
REFUSED
NO_EFFECT
AMBIGUOUS
ERROR
CANCELLED
```

Never translate "OS API returned success" into "business action succeeded" without evidence.

### 5. Evals must test side effects around the target, not just target state

Cua background-action tests check:

- target state;
- focus preservation;
- z-order;
- cursor preservation where observable;
- no leaked input.

**Apply to Lumi:** desktop evals need collateral-effect oracles, not only "button toggled".

### 6. Matrix dimensions are part of the product contract

Cua tracks OS, window system, toolkit/harness, addressing mode, foreground/background delivery, scope, oracle, status, and observed behavior.

**Apply to Lumi:** our certification matrix should explicitly include:

- OS/version;
- app/version/toolkit;
- action;
- semantic vs coordinate addressing;
- foreground/background;
- executor route;
- expected effect/refusal;
- evidence oracle.

### 7. Unknown or unsupported capability should fail closed

Cua risk maps unknown tools to fail-closed behavior and keeps unsupported delivery as explicit refusal or gap.

**Apply to Lumi:** capability discovery and executor routing should never infer support from absence of evidence.

### 8. Protected resources should be checked again immediately before mutation

Cua's browser profile/origin model revalidates live state at action time.

**Apply to Lumi:** permission at task start is not enough. Recheck mutable security-sensitive context before material side effects:

- active origin;
- target account;
- selected customer;
- payment destination;
- file identity;
- process/window identity.

### 9. Privacy-friendly action history can be useful without recording content

Cua's computer-history design keeps encrypted local event metadata while explicitly excluding raw arguments/results, typed text, clipboard data, screenshots, accessibility trees, URLs, titles, and paths in the baseline profile.

**Apply to Lumi:** local operational history can be useful while remaining content-minimized.

Separate:

- action accountability metadata;
- optional evidence content.

Do not default to "record everything".

### 10. Driver sessions need lifecycle and generation semantics

Cua tests session ownership, idle cleanup, exact-once cleanup, and invalidation of handles across runtime generations.

**Apply to Lumi:** executor/session handles should carry generation/lease identity and become invalid after restart/rebind.

## Concrete architecture changes for Lumi

1. Add permission ceiling + autonomy mode + manifest as separate concepts.
2. Add exact ActionResult outcome vocabulary.
3. Add collateral-effect desktop oracles.
4. Add capability certification matrix.
5. Protect existing authenticated browser-profile attachment as explicit capability.
6. Revalidate mutable protected context at mutation time.
7. Add privacy-minimized local history profile.
8. Add runtime-generation/session handle semantics.

## What not to copy blindly

- Cua's low-level permission modes should not replace Lumi's business-action policy.
- Cua's generic desktop risk taxonomy is necessary but insufficient for financial/legal/customer-facing effects.
- We should not adopt optional Cua components with incompatible/unclear licenses just to improve vision.
- Cua's broad platform matrix is a model for evidence discipline; Lumi should first certify only the app/OS combinations required by paid workflows.
