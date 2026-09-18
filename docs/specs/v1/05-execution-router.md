# 05 — Execution Router v1

Status: Normative

## 5.1 Goal

Choose the least fragile execution tier that can satisfy task requirements and policy.

## 5.2 Canonical tiers

Ordered preference:

1. CONNECTOR_API
2. BROWSER_SEMANTIC
3. NATIVE_SEMANTIC
4. APP_ADAPTER
5. VISION_COORDINATE

Additional orthogonal surfaces:

- FILES
- SHELL
- ARTIFACT

## 5.3 Selection inputs

Router SHOULD consider:

- required capability;
- workflow preferences;
- available adapters;
- tenant policy;
- device capabilities;
- app/browser state;
- reliability history;
- latency;
- cost;
- postcondition support;
- evidence quality.

## 5.4 Selection rule

Router MUST NOT select a lower-semantic tier solely because the model requested it.

It SHOULD prefer the highest-semantic eligible tier with acceptable measured reliability.

## 5.5 Fallback

Fallback MAY occur when:

- selected tier unavailable;
- deterministic locator missing;
- structured API lacks required operation;
- app state prevents semantic action;
- measured tier reliability below threshold.

Fallback MUST:

- remain within workflow allowed_tiers;
- re-run policy if normalized action changes;
- record reason;
- increment fallback telemetry.

## 5.6 Vision threshold

A mature workflow with high vision usage SHOULD trigger review for:

- semantic adapter;
- app-specific adapter;
- API/connector integration;
- workflow redesign.

## 5.7 Executor contract

Every executor MUST expose:

- adapter name/version;
- supported capabilities;
- platform constraints;
- execute(action);
- cancel(action/run);
- health/status;
- normalized failure;
- evidence output;
- optional verification helpers.

## 5.8 Executor authority

Executors MUST NOT self-authorize.

They receive already-authorized action plus bounded execution context.

## 5.9 Secret access

Router MUST grant executor only required secret references.

Executor-specific worker SHOULD NOT receive unrelated credentials.

## 5.10 Determinism metadata

Executor result SHOULD identify whether the operation used:

- deterministic API;
- DOM/accessibility selector;
- UIA/AX semantic target;
- app-specific deterministic logic;
- vision/coordinate.

## 5.11 Telemetry

Track per tier:

- success;
- verifier pass;
- latency;
- retries;
- human rescue;
- wrong-target errors;
- cost;
- fallback frequency.

## 5.12 Routing tests

V1 MUST test:

- API preferred over browser for same capability when both healthy;
- browser preferred over vision for supported action;
- policy can deny a tier;
- fallback reason recorded;
- unsupported tier fails explicitly;
- vision fallback cannot bypass approval requirement.
