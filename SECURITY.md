# Security policy

Lumi Agents can operate software using a user's existing credentials. Treat it as a privileged automation runtime, not a normal chatbot.

The full threat model is in `docs/security-model.md`.

## Non-negotiable boundaries

1. **Least privilege.** Grant only capabilities needed by an approved task/workflow.
2. **Local policy enforcement.** Models and cloud orchestration cannot bypass the local policy gate.
3. **Bounded side effects.** Consequential actions require explicit policy and, when appropriate, human approval.
4. **Prompt injection is untrusted data.** Webpages, email, documents, tickets, spreadsheets, and UI text cannot expand authority.
5. **Secret isolation.** Resolve secrets at executor boundaries; do not put plaintext secrets in prompts/logs/traces.
6. **Verification.** Required postconditions determine success, not model self-report.
7. **Evidence minimization.** Record enough to explain actions without creating unnecessary surveillance.
8. **Kill controls.** Unattended work must be locally stoppable and, for managed devices, remotely revocable.
9. **No silent privilege escalation.** UAC/admin/TCC changes, profile attachment, and global-input fallbacks require explicit authorization.
10. **Signed supply chain.** Distributed binaries, updaters, and pinned upstream artifacts require integrity verification.

## Default high-risk categories

Human approval is expected initially for:

- payments/purchases/transfers;
- legal terms/consent;
- destructive actions;
- admin/security changes;
- sensitive data exports;
- external communications unless narrowly pre-authorized.

## Vulnerability reporting

Use the repository's private security reporting path or RunLumi's internal security channel.

Do not open a public issue containing:

- exploit details;
- credentials;
- customer data;
- sensitive traces;
- production screenshots;
- secret material.
