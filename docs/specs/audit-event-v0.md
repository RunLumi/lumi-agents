# Audit Event v0

Status: design contract for implementation.

## Event envelope

```yaml
event_id: uuid
timestamp: rfc3339
workflow_id: string
run_id: string
action_id: string
principal: string
risk_class: string
resource: string
target: string

policy:
  decision: ALLOW|DENY|REQUIRE_APPROVAL
  rule_id: string

approval:
  id: optional
  action_digest: optional

executor:
  tier: connector|browser|native|app|vision|shell|files|artifact
  adapter: string

result:
  status: success|failed|ambiguous|cancelled

verification:
  status: passed|failed|not_required

evidence:
  refs: []
```

## Requirements

- structured evidence preferred over screenshots;
- secrets redacted;
- event ordering tamper-evident where practical;
- retention configurable;
- evidence references tenant-scoped;
- approval/action digest link preserved;
- failure taxonomy attached.

## Privacy

Audit is for accountability and debugging, not continuous employee surveillance.
