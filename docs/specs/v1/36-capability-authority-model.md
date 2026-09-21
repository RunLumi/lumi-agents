# 36. Capability and authority model

Status: **Backend catalog and structured selection implemented; Chrome/computer adapters planned**
Date: 2026-09-21

## Source of truth

The backend `CapabilityDescriptor` catalog defines each capability’s identifier,
authority class, interactive/unattended eligibility and implementation readiness.
The capability snapshot, structured composer selections, scheduler validation,
policy grants and runtime tool construction derive from that catalog.

Natural language describes a goal. Structured tool selection expresses user intent.
Policy authorizes the normalized action. The runtime executes it. Verification
determines the outcome. Prose, pasted content, quoted text, files, webpages, model
output and tool observations cannot grant a capability.

## Current catalog

| Capability | State | Scope |
|---|---|---|
| files | implemented | Project workspace |
| shell | implemented, interactive only | explicit project-local command |
| browser | implemented | approved public HTTPS read origins |
| chrome | unavailable | authenticated selected session not wired |
| computer | unavailable | native adapter not wired into engagement |

Readiness is observed by the host. The UI cannot promote an unavailable or stale
capability. Automation creation revalidates selected tools and authority before
admission.

## Acceptance

Test ready/setup/unavailable/stale states, explicit selections, prose-injection
attempts, cross-Project references, changed browser scope, expired authority and
unattended restrictions at the backend boundary.
