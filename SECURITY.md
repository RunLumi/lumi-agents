# Security policy

Lumi Agents can operate software using a user's existing credentials. Treat the runtime as a privileged automation component, not as a normal chatbot.

## Non-negotiable boundaries

1. **Least privilege.** Grant only the OS, browser, application, filesystem, and network capabilities required by an approved workflow.
2. **Local policy enforcement.** A cloud model cannot directly bypass local policy. Every side-effecting action passes through the local policy gate.
3. **Approval for material effects.** Sending messages, submitting forms, deleting data, moving money, changing permissions, publishing, and other externally visible or destructive actions require an approval policy appropriate to the workflow.
4. **Prompt injection is untrusted data.** Text from webpages, email, documents, tickets, and app UIs never becomes authority to change policy, reveal secrets, or expand scope.
5. **Secret isolation.** Credentials belong in OS key stores or approved secret managers. Do not place raw secrets in model prompts, logs, screenshots, traces, or workflow definitions.
6. **Evidence and replay.** Record structured action metadata and redacted evidence sufficient to explain what happened without collecting unnecessary user data.
7. **Kill switch.** Unattended execution must be locally stoppable and remotely revocable.
8. **No silent privilege escalation.** UAC/admin elevation, macOS permission changes, browser profile attachment, and global input fallbacks require explicit user/admin authorization.

## Threats we design for

Prompt injection, credential leakage, destructive actions, cross-tenant leakage, compromised upstreams or update channels, hallucinated/stale UI state, privilege/session-boundary mistakes, sensitive screenshots/trajectories, and a compromised orchestration service attempting to widen local permissions.

## Vulnerability reporting

Use the repository's private security reporting path or RunLumi's internal security channel. Do not open a public issue containing exploit details, credentials, customer data, or sensitive traces.
