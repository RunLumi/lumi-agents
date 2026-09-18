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


## 5.13 Trust-boundary fallback

Fallback across a stronger trust boundary MUST require fresh policy evaluation and, when policy requires it, approval.

Examples:

- semantic target -> raw global pointer;
- background delivery -> foreground activation;
- isolated browser context -> existing authenticated profile;
- sandboxed shell -> host shell;
- bounded network egress -> arbitrary network;
- app-scoped action -> system-wide automation.

The router MUST prefer a precise REFUSED outcome over an unauthorized or surprising fallback.

## 5.14 Canonical executor outcomes

Every executor MUST normalize execution to:

- DELIVERED;
- REFUSED;
- NO_EFFECT;
- AMBIGUOUS;
- ERROR;
- CANCELLED.

An adapter-native "success" MUST NOT become DELIVERED unless the required executor-level effect is observed according to the executor contract.

REFUSED MAY be the expected correct outcome for an unsupported or policy-bounded path.

## 5.15 Supervised executor processes

Crash-prone or lower-trust executors SHOULD run behind supervised process boundaries where practical.

Examples:

- browser worker;
- Cua/native driver;
- plugin/MCP bridge;
- external agent harness;
- optional native helper.

Supervision SHOULD provide:

- deadline;
- cancellation;
- bounded/versioned IPC;
- structured errors;
- health;
- restart policy;
- runtime-generation identity.

Executor crash MUST NOT corrupt policy authority or durable task state.

## 5.16 Additional routing tests

V1 MUST additionally test:

- background route unavailable -> no silent foreground activation;
- semantic route unavailable -> no silent global-pointer escalation;
- isolated browser denied -> no silent existing-profile attachment;
- exact REFUSED has no forbidden side effect;
- adapter-native success without effect oracle is not DELIVERED;
- executor restart invalidates stale generation-bound handles.
