# 03 — Action & Observation Protocol v1

Status: Normative  
Supersedes: action-protocol-v0.md

## 3.1 Goal

Normalize intent and observed state so policy, execution, verification, and audit are independent of provider and executor implementation.

## 3.2 ActionProposal

Required conceptual schema:

```yaml
schema_name: lumi.action
schema_version: "1.0"

action_id: string
task_id: string
run_id: string
workflow_id: optional string
step_id: optional string

principal:
  principal_id: string
  kind: USER|WORKFLOW|SCHEDULE|SERVICE|ADMIN|SYSTEM

capability: string

resource:
  type: string
  id: string
  sensitivity: optional string

target:
  canonical: string
  display: optional string

operation: string
arguments: object

expected_effect:
  summary: string
  external_visibility: boolean
  reversible: boolean

risk_class: READ|LOCAL_WRITE|EXTERNAL_WRITE|COMMUNICATION|DATA_EXPORT|CREDENTIAL|FINANCIAL|LEGAL_CONSENT|DESTRUCTIVE|ADMIN

execution_preferences:
  allowed_tiers: []
  preferred_tier: optional

evidence_requirements: []
postconditions: []

idempotency:
  key: optional string
  semantics: NONE|CLIENT_KEY|REMOTE_KEY|VERIFY_BEFORE_RETRY

timeout_ms: integer
```

## 3.3 Action immutability

Once approved, material fields MUST NOT change.

Material fields include:

- capability;
- resource;
- target;
- operation;
- destination;
- financial/material value;
- consequential arguments.

Any material change requires new policy evaluation and approval where applicable.

## 3.4 Raw gestures

Raw executor gestures such as click coordinates MAY exist only inside executor-specific plans.

They MUST NOT be the unit of organization-level policy.

## 3.5 Observation

Observation schema:

```yaml
schema_name: lumi.observation
schema_version: "1.0"

observation_id: string
task_id: string
run_id: string
source:
  tier: connector|browser|native|app|vision|shell|files|artifact
  adapter: string
  resource: optional string

timestamp: rfc3339
kind: STATE|CONTENT|RESULT|ERROR|EVIDENCE
payload: object

confidence:
  kind: DETERMINISTIC|STRUCTURED|MODEL_INFERRED|VISUAL_HEURISTIC
  score: optional number

sensitivity: optional string
provenance: optional object
```

## 3.6 Observation trust

Observation confidence MUST NOT grant authority.

Model-inferred and visual observations SHOULD be verified through structured state before consequential actions when feasible.

## 3.7 Execution result

Executor result MUST distinguish:

- SUCCESS
- FAILED
- AMBIGUOUS
- CANCELLED

SUCCESS means executor operation returned successfully, not necessarily that workflow postconditions passed.

## 3.8 Error envelope

Errors MUST include canonical failure category from spec 18 and MAY include adapter-native diagnostics.

Provider/executor-native raw errors MUST NOT be the only persisted failure representation.

## 3.9 Protocol extensibility

Unknown optional fields MAY be ignored.

Unknown required schema versions MUST be rejected unless a migration exists.

Enums SHOULD reserve UNKNOWN for telemetry only; policy decisions MUST fail closed on unknown risk/capability values.

## 3.10 Redaction

Action/Observation payloads MUST support field-level redaction before leaving local device or entering long-term audit.

## 3.11 Correlation

Every observation/result MUST be traceable to task/run and, when applicable, action/step IDs.

## 3.12 Contract tests

V1 MUST include fixtures proving:

- same business action can map to browser/native/connector executors;
- policy sees identical normalized action regardless of tier;
- adapter-native failure maps to canonical failure;
- approved action mutation is detected;
- redaction removes marked secret fields.
