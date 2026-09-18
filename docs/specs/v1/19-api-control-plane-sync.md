# 19 — API, Control Plane & Sync v1

Status: Normative

## 19.1 Goal

Define optional cloud/enterprise coordination without moving action authority away from trusted local runtime.

## 19.2 Local authority

Control plane MAY request work and distribute policy.

It MUST NOT directly bypass local policy/executor gate.

## 19.3 Control-plane responsibilities

May include:

- tenant/device registry;
- organization policy distribution;
- workflow pack catalog;
- provider configuration metadata;
- schedules/event metadata;
- fleet status;
- revocation;
- task handoff;
- aggregated observability;
- release/update ring metadata.

## 19.4 Device sync

Device sync message SHOULD include:

- device_id;
- runtime/app version;
- trust state;
- policy version;
- supported capabilities;
- last_seen;
- update ring.

Avoid sending unnecessary local content.

## 19.5 Policy distribution

Organization policy SHOULD be signed/versioned.

Local runtime MUST reject invalid/unsupported policy version.

Cloud unavailability MUST NOT expand permissions.

## 19.6 Task dispatch

Remote dispatch MUST include:

- tenant/principal;
- task/workflow ref;
- budget;
- capability expectations;
- privacy constraint;
- lease/expiry;
- dedup/idempotency token.

## 19.7 Unattended lease

Managed unattended execution SHOULD require revocable lease.

Expired/revoked lease MUST stop starting new privileged actions.

## 19.8 Sync conflict

For mutable metadata, sync MUST define conflict policy.

Security/policy state SHOULD prefer safer state on conflict.

## 19.9 Offline

Local task MAY continue offline only if:

- workflow/policy permits;
- no required cloud lease validation has expired;
- provider/local executor available;
- audit buffering policy permits.

## 19.10 Audit upload

Audit/evidence upload follows tenant egress and retention.

Local audit MAY buffer offline.

## 19.11 API versioning

Control-plane APIs MUST follow spec 20.

## 19.12 Authentication

Device/control plane communication SHOULD use strong device identity and short-lived credentials.

## 19.13 Revocation

Control plane MUST support revoking:

- device;
- workflow pack;
- organization policy;
- unattended lease;
- connector registration where applicable.

## 19.14 Tests

V1 MUST test:

- forged policy rejected;
- revoked lease;
- offline safe behavior;
- sync conflict;
- replayed task dispatch deduped;
- tenant mismatch rejected;
- cloud cannot execute around local policy.


## 19.15 Execution environment ownership

Remote clients and control plane supervise work, but the ExecutionEnvironment owns local machine state.

Environment-owned resources include:

- filesystem/workspace state;
- authenticated browser/application sessions;
- local secret references;
- native OS permissions;
- local provider/harness processes;
- local executor state.

Remote clients MUST NOT substitute their own machine state, credentials, or filesystem for the execution environment.

## 19.16 Relay/control-plane hot-path rule

The hosted control plane MAY provide:

- identity;
- environment discovery;
- policy distribution;
- fleet metadata;
- schedules/events;
- revocation;
- connection signaling;
- notification fan-out.

When practical and policy permits, normal task/control traffic SHOULD terminate directly at the selected ExecutionEnvironment rather than require the hosted relay to proxy all application traffic.

This is a SHOULD, not an absolute requirement. Relayed traffic MAY be used when network topology, enterprise policy, or product constraints require it.

The architecture MUST preserve the same local policy authority either way.

## 19.17 Remote credential principle

Cloud identity, relay/bootstrap credentials, and environment-local sessions are separate trust boundaries.

A relay/control plane SHOULD use short-lived, scoped credentials for environment connection bootstrap.

It SHOULD NOT require possession of the employee environment's long-lived local secrets.

Remote-client authentication MUST NOT automatically imply every RPC/action capability.

## 19.18 Environment capability descriptor

Device/environment sync SHOULD advertise typed capabilities in addition to versions.

Clients MUST use capability negotiation for features that can vary independently, such as:

- task resume/fork;
- approval protocol version;
- browser/native execution;
- artifact types;
- workflow pack schema;
- provider/harness support;
- remote supervision controls.

## 19.19 Additional tests

V1 MUST additionally test:

- remote client cannot substitute its own local path/credential for environment-owned state;
- relay/bootstrap token cannot be used as a broader environment session;
- authenticated transport without required action scope is denied;
- capability absent after downgrade disables/rejects feature even if client cache previously saw it;
- control-plane outage does not expand local authority.
