# Audit Event v0

Status: design contract for implementation.

## Event envelope

```yaml
event_id: uuid
timestamp: rfc3339
task_id: string
workflow_id: string
run_id: string
action_id: string
environment_id: string
runtime_generation: string
principal: string
risk_class: string
resource: string
target_class: string

policy:
  decision: ALLOW|DENY|REQUIRE_APPROVAL
  rule_id: string
  policy_version: string

approval:
  id: optional
  action_digest: optional

executor:
  tier: connector|browser|native|app|vision|shell|files|artifact
  adapter: string
  route: optional

result:
  outcome: DELIVERED|REFUSED|NO_EFFECT|AMBIGUOUS|ERROR|CANCELLED
  failure_class: optional

verification:
  status: passed|failed|ambiguous|not_required
  verifier_id: optional

evidence:
  kinds: []
  refs: []
```

## Requirements

- structured evidence preferred over screenshots;
- secrets redacted;
- event ordering tamper-evident where practical;
- retention configurable;
- evidence references tenant-scoped;
- approval/action digest link preserved;
- failure taxonomy attached;
- executor outcome and verification are separate fields;
- a `DELIVERED` transport outcome does not override a failed required verifier.

## Privacy-minimized baseline

The default action-history profile should avoid persisting raw content unless evidence policy explicitly requires it.

Do not store by default:

- plaintext secrets;
- typed text;
- clipboard content;
- raw tool arguments/results;
- screenshots/video/audio;
- accessibility trees;
- URL/window titles;
- file/profile paths;
- arbitrary free-form model reasoning.

Prefer fixed enums, opaque identifiers, counts, hashes, capability names, effect classes, and evidence references.

There is no "debug mode" that silently disables privacy policy in customer production.

## Durable intent events

Consequential actions should make it possible to distinguish:

1. intent persisted;
2. policy decision;
3. approval requested/resolved;
4. dispatch started;
5. executor result;
6. verification result;
7. finalized action outcome.

This ordering is needed to recover safely after crash/restart.

## Privacy

Audit is for accountability, reliability, and debugging, not continuous employee surveillance.
