# Research: global automations, project extensions and credentials

Reviewed: 2026-09-20. Supporting research, not a claim that Lumi implements the
referenced products. Normative decisions: Specs 27, 28 and 29.

## Requested basis and repository baseline

The user's supplied Scheduled tasks screenshot shows a global searchable list,
Active/Paused/Completed filters, creation and unread state. It does not establish
backend execution or authorization semantics. Use its interaction pattern, not
its personal task content, in Lumi fixtures.

Reviewed Lumi at commit `3c43ea49497c6086d7067b2b42ae5df90079e1e6`: AGENTS.md,
Specs 13/14/17/22/24, the v1 index and reference registry; inspected the secret
broker interface in `crates/lumi-secrets/src/broker.rs`. Existing contracts already
prefer opaque secret references and OS protected storage. The broker interface
validates references and delegates to a backend; that alone does not prove
caller/project/account isolation. No runtime code or live credential test is
claimed by this documentation change.

The requested project-local `skills/` and `plugins/` direction is retained. The
proposal to keep plaintext credentials in the project with only `.gitignore`
is changed deliberately for the security reasons below.

## Primary-source findings and Lumi decisions

### Scheduled work: OpenAI

Source: [Scheduled tasks](https://developers.openai.com/codex/app/automations),
currently redirecting to official ChatGPT Learn documentation.

The documentation describes centralized management of active, paused and
completed scheduled tasks and recent runs. Desktop tasks may use project folders
or worktrees; local work needs the relevant computer/app available. Web tasks do
not directly own a folder on a user's computer. Manual prompt testing and review
of early runs are recommended.

Adopt a global management view plus project filtering over one record. Preserve
Lumi's explicit environment boundary. Do not infer cloud execution, automatic
wake-from-sleep, or broader authority from a global list.

### Skills: Agent Skills and Codex

Sources: [Agent Skills specification](https://agentskills.io/specification) and
[OpenAI Build skills](https://developers.openai.com/codex/skills).

Agent Skills defines a directory with SKILL.md frontmatter and optional scripts,
references and assets. Codex documents metadata-first progressive disclosure,
repository/user/admin scopes and current `.agents/skills` discovery. The existing
Ads sample uses `.codex/skills`; this is a compatibility requirement for Lumi,
not evidence that it is the current universal Codex directory convention.

Adopt portable method bundles and bounded metadata discovery. Keep installation,
visibility, instruction activation and executable permission separate. Foreign
`allowed-tools` metadata never grants Lumi capabilities.

### Project installation versus physical package storage: Claude Code

Source: [Plugins reference](https://code.claude.com/docs/en/plugins-reference),
sections on installation scopes and plugin caching/file resolution.

Claude Code distinguishes user, project, local and managed installation scopes.
Project declarations can be version-controlled while marketplace package versions
are kept in a local cache. Plugin components can include skills, hooks and MCP
servers. This supports separating a portable project declaration from immutable
package bytes and local runtime state.

Adopt that separation and explicit version identity. Do not copy all component
formats or assume foreign hooks are safe to execute. Lumi V1 does not need a
marketplace or arbitrary plugin runtime compatibility to support project installs.

### OpenClaw: scope is not a sandbox

Sources: [Skills](https://docs.openclaw.ai/tools/skills) and
[Plugins](https://docs.openclaw.ai/tools/plugin).

OpenClaw distinguishes skill location from visibility and explicitly notes that
skill allowlists are not host-shell authorization. It warns that third-party
skills should be treated as untrusted code. Its native plugin format can load
runtime modules in process, while compatible bundles are a distinct category.

Adopt explicit inventories, provenance and review. Do not copy in-process loading
into Lumi's privileged policy/secret core. A project-local path or restricted
skill list cannot establish isolation against unrestricted host execution.

### Git exclusion is not secret protection

Sources: [gitignore](https://git-scm.com/docs/gitignore) and
[git-check-ignore](https://git-scm.com/docs/git-check-ignore).

Git ignore rules apply to intentionally untracked paths; already-tracked files
are unaffected. Effective patterns may be overridden by more local rules.

Therefore require hygiene checks for both tracked state and ignore behavior,
but make the security boundary independent of Git. Do not claim adding an ignore
rule removes a secret from existing commits, archives, or a process's read access.

### Protected credential storage and its limits

Sources: [OpenAI authentication](https://developers.openai.com/codex/auth),
[Apple Keychain data protection](https://support.apple.com/guide/security/keychain-data-protection-secb0694df1a/web),
[Microsoft CryptProtectData](https://learn.microsoft.com/en-us/windows/win32/api/dpapi/nf-dpapi-cryptprotectdata),
and [OWASP Secrets Management](https://cheatsheetseries.owasp.org/cheatsheets/Secrets_Management_Cheat_Sheet.html).

Codex documents file/keyring/auto storage and warns that its auth cache contains
access tokens. This is not a recommendation to put auth caches in projects.
Apple documents protected Keychain storage. Microsoft documents DPAPI's usual
logon/computer relationship and broader machine-scope behavior. OWASP emphasizes
controlled access, auditing and secret lifecycle management.

Lumi decision: prefer the existing broker plus OS/approved-vault backends, project-
scoped grants, and reference-only project metadata. Unlike an automatic file
fallback, a locked/unavailable store should block or offer explicitly ephemeral
operation. At-rest protection still requires process isolation and per-use
account/destination authorization; it cannot make a privileged malicious process
safe. No custom cryptography or home-grown project vault for V1.

### MCP authentication and credential misuse

Source: [MCP security best practices](https://modelcontextprotocol.io/docs/2025-11-25/tutorials/security/security_best_practices).

The guidance prohibits token passthrough and discusses confused-deputy, SSRF,
local MCP compromise, and OAuth URL risks. A connection to a remote MCP server is
not permission to hand it arbitrary downstream account tokens.

Adopt audience/account/destination checks and least privilege. Prefer brokered
requests; raw secrets given to a process can be misused. MCP tool descriptions
and a declared read-only flag are not enough to enforce Ads read-only behavior.

## Decisions specific to Lumi, not competitor facts

- Global Automations is a view over project-bound records, not a global scheduler.
- Completed means bounded scheduling is exhausted, not merely one successful run.
- Per-user read markers never resolve approvals or incidents.
- Shared browser/resource arbitration must span different automations/projects.
- Skills and install declarations are portable; grants and secrets are not.
- Cloning/relinking/importing requires trusted project/connection registration.
- Package/source changes are reviewed for scheduled runs; no live hot-swap.
- `.gitignore` is initialized during authorized setup, not silent folder browsing.

## Validation boundary

This work specifies behavior and acceptance tests. It does not install plugins,
connect accounts, move real secrets, enable schedules, or establish live Ads safety.
Runtime integration, platform sandbox tests, and user-owned credential canaries
remain implementation work. Public CI must use synthetic secrets/accounts.
