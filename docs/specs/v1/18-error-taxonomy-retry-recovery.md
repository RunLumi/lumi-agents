# 18 — Error Taxonomy, Retry & Recovery v1

Status: Normative

## 18.1 Goal

Use stable error categories so recovery targets the failing layer and does not create duplicate/unsafe side effects.

## 18.2 Canonical categories

Required v1 categories:

- MODEL_REASONING
- MODEL_FORMAT
- MODEL_CONTEXT_LIMIT
- MODEL_REFUSAL
- PROVIDER_RATE_LIMIT
- PROVIDER_UNAVAILABLE
- POLICY_DENY_EXPECTED
- POLICY_BUG
- APPROVAL_TIMEOUT
- APPROVAL_INVALID
- CONNECTOR_FAILURE
- BROWSER_SELECTOR
- BROWSER_STATE
- NATIVE_ELEMENT
- NATIVE_SESSION
- VISION_GROUNDING
- FILESYSTEM
- SHELL_EXECUTION
- OS_PERMISSION
- AUTH_SESSION
- UPSTREAM_DRIVER
- NETWORK
- POSTCONDITION
- AMBIGUOUS_STATE
- BUDGET_EXCEEDED
- CRASH
- USER_CANCEL
- VERSION_INCOMPATIBLE
- DATA_POLICY
- SECURITY_VIOLATION

Adapters MAY attach native codes.

## 18.3 Retryability

Each error MUST map to:

- RETRY_IMMEDIATE
- RETRY_BACKOFF
- REFRESH_AUTH
- REOBSERVE
- REQUIRE_APPROVAL
- REQUIRE_USER
- TERMINAL
- AMBIGUOUS_NO_RETRY

## 18.4 Retry safety

Retry MUST NOT happen automatically after ambiguous consequential side effect.

Runtime MUST verify external state first.

## 18.5 Backoff

Rate-limit/provider/network retries SHOULD use bounded exponential/jittered backoff.

## 18.6 Alternate provider

Provider fallback MAY be recovery only if privacy/capability policy permits.

## 18.7 Alternate executor

Execution fallback MAY be recovery only if:

- workflow allows tier;
- policy permits;
- normalized business action unchanged or re-evaluated.

## 18.8 Re-observation

Stale browser/native state SHOULD re-observe before retry.

## 18.9 Compensation

Workflow MAY define compensating action for reversible side effects.

Compensation is a new side effect and MUST pass policy.

## 18.10 Human exception

After retry budget exhausted, runtime SHOULD route to human with:

- what failed;
- what was attempted;
- current state;
- evidence;
- safe next options.

## 18.11 Crash recovery

On restart:

- load checkpoint;
- identify last attempted consequential action;
- determine whether result persisted;
- verify external postcondition;
- resume or enter ambiguity.

## 18.12 Failure causality

Audit SHOULD preserve:

- root category;
- adapter/provider native error;
- retry history;
- fallback history;
- final disposition.

## 18.13 Tests

V1 MUST test:

- rate-limit recovery;
- stale UI re-observe;
- auth refresh;
- provider fallback constrained by privacy;
- ambiguous submit no double-submit;
- retry budget exhaustion;
- compensating action policy;
- crash resume.
