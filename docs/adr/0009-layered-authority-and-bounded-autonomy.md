# ADR 0009: Layered authority and bounded autonomy

- Status: Accepted
- Date: 2026-09-18

## Decision

Separate authority ceiling from autonomy mode.

Effective authority is the intersection of:

```text
platform hard invariants
  ∩ organization managed policy
  ∩ user policy
  ∩ workflow/pack manifest
  ∩ task temporary grant
```

Lower layers may narrow, never widen, higher ceilings.

Initial autonomy modes:

- supervised;
- bounded unattended;
- explicitly acknowledged unrestricted development/testing.

Workflow mode unattended execution uses a reviewed capability manifest.

Existing authenticated browser/app sessions are protected resources and require explicit policy/grant. Mutable protected context is revalidated immediately before material mutation.

## Why

- Cua separates permission mode from policy ceilings and uses bounded manifests.
- Claude Code demonstrates the importance of organization-managed settings that constrain project/user customization.
- Codex separates sandbox and approval policy and supports narrower temporary permission escalation.
- Lumi needs business-effect policy above all lower-level engine permission systems.

## Consequences

- No generic `autoApprove=true` authority switch.
- Temporary grants are narrow, expiring, and action/resource scoped.
- Hooks, skills, agents, plugins, MCP servers, and external harnesses cannot widen authority.
- Safe refusal is preferred to silent escalation across trust boundaries.
- Financial, legal, destructive, admin/security, sensitive-export, and broad communication actions remain conservative by default.
