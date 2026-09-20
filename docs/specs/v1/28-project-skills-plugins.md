# 28 — Project Skills & Plugins v1

Status: Normative target contract; installation/activation must be implemented
and tested before being described as available.

Extends [14 — Extensions](14-mcp-connectors-extension-sdk.md),
[24 — Skills](24-skills-subagents.md), and
[26 — Projects](26-project-workspace-folder-as-project.md).
Credentials follow [29](29-project-connections-secrets.md); scheduled consumers
follow [13](13-scheduler-background-triggers.md) and [27](27-global-and-project-automations.md).

## 28.1 Outcome and vocabulary

Install reusable expertise and integrations for one Project without silently
changing other projects, enabling code, or sharing credentials.

| Concept | Responsibility | Not equivalent to |
| --- | --- | --- |
| Skill | Instructions and optional scripts/references/assets for a reusable method | Tool permission or an authenticated integration |
| Plugin | Versioned installable bundle of Skills, tools/connectors, hooks or templates | A trusted process or new policy authority |
| Connector instance | Configured integration endpoint/account within an environment | The reusable plugin package |
| Connection | Project-scoped binding to an authenticated identity | A token file inside the package |
| Automation | When/how a bounded task is created | Installing or authorizing its dependencies |

A Skill can exist without a Plugin. A Plugin can contain several Skills and
connectors. Installing a Plugin MUST NOT install a second copy of each bundled
Skill into `.lumi/skills/`. Preserve the package namespace and version ownership.

## 28.2 Project filesystem contract

```text
project/
  .gitignore
  .lumi/
    automations.yaml                    # shareable desired schedules
    extensions.lock.json                # exact package/content resolutions
    connections.json                    # shareable connection requirements, no secrets
    skills/
      daily-review/
        SKILL.md                        # metadata + method
        scripts/                        # optional, execution separately gated
        references/
        assets/
    plugins/
      publisher/
        package-name/
          install.json                  # non-secret install declaration
          package/                      # optional vendored/local source
            plugin.json
            skills/
            scripts/
    local/                              # ignored; protected local metadata only
      connections.json                  # optional opaque bindings, never token values
    runtime/                            # ignored generated local state
    cache/                              # ignored disposable caches
```

`.lumi/skills/` and `.lumi/plugins/` are first-class project locations. They are
not credential stores. Do not ignore the whole `.lumi/` tree: prompts, skills,
install declarations, locks and automation intent should remain shareable.

Small first-party/local packages MAY be vendored in `package/` after review.
Downloaded third-party payloads SHOULD use a verified content-addressed cache
outside the checkout, referenced by the project declaration and lock. Installation
scope is Project even when immutable bytes are deduplicated on the device.
Do not copy dependency trees, executable caches, or secrets into version control.
No shared mutable package/session state across projects.

All component paths resolve relative to a validated package root; cache access is
an explicit read-only package resource, not permission to read the parent folder.
Reject absolute escapes, traversal, symlink/junction escapes, special files,
case/Unicode path collisions, oversized archives and decompression bombs.

## 28.3 Discovery, scope, and compatibility

Opening a Project performs bounded metadata discovery, never installation code,
network startup, OAuth, dependency installation, or automatic schedule activation.
New material appears as `Discovered / Needs review`.

V1 MUST understand Agent Skills-style `SKILL.md`: name, description, Markdown body,
and optional scripts/references/assets. Use metadata-first progressive disclosure;
load selected instructions/resources only when needed and within context budgets.
An `allowed-tools` or similar foreign field is a request/hint, not a Lumi grant.

Approved compatibility discovery MUST include the existing `.codex/skills/`
layout used by Ads Agents and `.agents/skills/` for current Codex-style projects.
`.claude/skills/` MAY be supported through the same resolver. Scan only authorized
roots; do not follow ancestor discovery outside Project scope or import home auth.

Leave existing source trees intact. Do not mass-move legacy skills into `.lumi/`
or copy methodology into prompts to simulate compatibility. Record the exact
source, canonical path, hash, and parser/compatibility version.

User/organization catalogs MAY make reusable packages available, but installing
or connecting in Project A does not enable them or their credentials in B.
Project activation selects from these catalogs under the organization ceiling.
A global Plugins & Skills menu, when provided, manages availability and shows
`Used in N authorized projects`; installation still names a target project.

## 28.4 Identity and resolution

Stable identity includes publisher/package, kind, version/content digest and
scope. Display names and filesystem names alone are insufficient.

Reference forms are semantic examples:

```text
project:daily-review
plugin:publisher/package-name:weekly-review
```

Short names or `$skill-name` references resolve only when unambiguous within the
project's reviewed inventory. Duplicate names from native/compatibility/plugin
roots MUST be surfaced, not silently overridden by path order. Explicit aliases
may be reviewed and saved. No merging of unrelated same-name Skill bodies.

Installed state, activation, connection readiness and trust are separate fields:

- present: discovered / installed / removed;
- activation: disabled / enabled;
- readiness: ready / needs-review / needs-connection / incompatible / blocked.

A checkmark saying Installed MUST NOT imply Enabled, Connected, or Safe.

## 28.5 Minimal manifest and lock contract

The native package `plugin.json` normalizes Spec 14 metadata: ID, version, origin,
license, minimum protocol/runtime, platforms, component paths, capabilities,
side effects, network destinations, filesystem requirements and connection slots.
A manifest declares requested access; effective access still comes from policy.

A project `install.json` identifies the package source and desired version or local
source. `extensions.lock.json` resolves exact immutable bytes and dependencies:

```json
{
  "version": 1,
  "plugins": [{
    "id": "example/reporting",
    "version": "1.0.0",
    "source": {"kind": "project", "path": ".lumi/plugins/example/reporting/package"},
    "integrity": "sha256:<computed-by-installer>",
    "manifest_digest": "sha256:<computed-by-installer>",
    "components": ["skill:daily-review"]
  }]
}
```

This is a format illustration, not an executable fixture: placeholders MUST be
rejected on installation. Remote entries additionally require a reviewed origin,
immutable revision/archive digest and resolved dependency identities. Lockfile
integrity detects change; it does not prove benevolence or confer trust.

Keep credentials, local absolute paths, live grants, browser profiles, local
connection bindings and private account identifiers out of portable locks.
Unknown security-relevant fields or unsupported mandatory components fail closed.
Do not claim universal plugin compatibility from recognizing a JSON filename.

## 28.6 Install flow

```text
Choose Project and source
-> fetch/stage under bounded download policy, without running code
-> inspect package/license/dependencies, verify pinned bytes
-> show components, requested access, endpoints, hooks and connection needs
-> install exact bytes + declaration/lock atomically
-> explicitly enable reviewed components
-> connect required accounts separately
-> test with the intended task/automation authority
```

Install and enable are distinct authorized mutations. A combined confirmation
can cover both only if the effects are individually visible. Repository config,
model output, webpage text and plugin output cannot self-approve installation.

No on-install lifecycle scripts, `npx ...@latest`, unpinned Git branches, arbitrary
remote shell pipes, or opportunistic package-manager execution. A genuinely
required setup command is a separate reviewed sandboxed action with pinned
dependencies, allowed destinations, bounded cost/time and no inherited secrets.

Respect the deployment's dependency release-age, advisory and license policy.
A passed scanner or signature is evidence of origin/checks, not proof of safety.
Interrupted install leaves the prior active revision intact, never half-enabled.

## 28.7 Execution and trust

Run third-party executable components outside the privileged Rust policy/secret
core. A child process alone is not a sandbox: enforce filesystem, process,
network and credential isolation with supported platform controls.

If the required isolation cannot be enforced, refuse that component. A separately
reviewed trusted-host mode MAY exist, but MUST disclose its broader trust and
cannot be silently selected for an unattended or untrusted package.

All tool effects, including script and hook effects, use normal ActionProposal,
policy, approval, audit and verifier contracts. Hooks can narrow/deny/log; they
cannot approve actions or convert model prose into authority.

MCP tool annotations are descriptive input, not security proof. Newly discovered
or changed tools/endpoints/side-effect mappings require review. A remote server
can change without a local package update; detect material capability/schema drift
on reconnect and before use rather than relying only on package version.

Use the credential broker from Spec 29. Plugins MUST NOT enumerate the vault,
inherit the host environment, import browser cookies, or receive other projects'
connections. An allowed network domain is not authority for every API/account on it.

## 28.8 Skills, scripts and run snapshots

Instruction-only skills do not need a running plugin process. Their text remains
lower-trust procedural context, never organization/user authorization.
Script execution is separately admitted even if the Skill is already enabled.

Each task/run captures an immutable dependency snapshot: selected Skill bodies,
package and script digests, tool schema/version, alias resolution, project config
and allowed connection identities. Active runs MUST NOT hot-swap package code or
silently load edited instructions under the old approval.

For scheduled work, changed instruction/script/plugin hashes require a reviewed
new snapshot before the next execution by default. A narrower managed update
policy may authorize specific changes; no update can broaden capabilities,
credential scope or data egress without fresh authorization. Self-authored edits
are not approved simply because the same agent produced them.

## 28.9 Update, rollback, disable and remove

Update stages a new immutable revision, shows source/component/permission diff,
then switches future admissions after review. Retain revisions referenced by
active runs/audit retention. Rollback selects known bytes but rechecks current
policy, advisories and revocations; old approval is not automatically revived.

Disable blocks new invocations and stops future dependent actions at safe gates.
Show affected tasks/automations; never silently substitute another similarly named
Skill. Removal offers preservation of user-authored source and generated data.
Do not delete another project's cache references or a user's modified package.

Uninstall does not equal upstream credential revocation. Offer disconnect/revoke
separately, with dependent-project impact under Spec 29. Preserve run history and
provenance. Garbage-collect only unreferenced verified cache revisions.

## 28.10 Project management UX

Project -> `Skills & Plugins` contains Skills, Plugins and Connections views.
Use a simple list with source/version, scope, enabled state, capability summary,
connection readiness and dependent automations. Connections deep-links to Spec 29.

Actions: Add skill, Install plugin, Inspect source, Enable/Disable, Test, Review
update, Roll back, Remove. New skill scaffolding writes a minimal SKILL.md rather
than inventing a plugin bundle. Do not present a marketplace as a V1 prerequisite.

An install review states whether the item is instruction-only, runs local code,
starts a remote MCP connection, adds hooks, or proposes automation templates.
A template remains disabled until separately reviewed as an Automation.

## 28.11 Ads Agents acceptance

Discover its existing `prompts/` and `.codex/skills/` without duplication. Resolve
`$daily-ads-ops`, `$ads-health-monitor`, `$ads-portfolio-review`, and the mandatory
session-lifecycle method through the project inventory. Keep cadence and mutable
thresholds in their original configuration layer.

Browser access is an explicitly bound existing session, not an invented API plugin
or a copied cookie file. Missing credentials/profile -> actionable blocked result.
Scheduled read-only is enforced by the executor/policy boundary, not only Skill
text. Hostile plugin code must not evade it using shell HTTP, browser CDP, an
unreviewed tool, or a reused account token. Unsupported enforceable paths block.

An automation pins method/source/config versions; updating an installed Skill
shows which automations need review. Session report and worklog obligations are
verified by runtime closeout; prompts alone cannot guarantee them after a crash.
Public regression fixtures use synthetic accounts and credentials only.

## 28.12 Required proof and limits

Required tests: metadata discovery executes no code; malformed manifest/archive
and path escape rejected; repeated install/import idempotent; same-name collision
not shadowed; A/B project activation isolated; compatibility paths resolve exact
sources; unknown mandatory components denied; denied install cannot run setup;
update cannot widen permissions; active snapshots survive update/rollback;
revocation blocks new actions; MCP tool drift requires review; scripts cannot read
vault/host environment or bypass read-only policy; disable/uninstall preserves
other projects and historical evidence; project clone carries requirements but
no grants or sessions; dependency checks gate scheduled execution.

Implement through existing extension/project/state/policy/secret contracts. First
prove one instruction-only Skill and one narrowly sandboxed or brokered integration.
Defer a public marketplace, arbitrary in-process plugins, cross-ecosystem runtime
emulation, and a generic dependency installer. This specification does not promise
security against a compromised operating system or an intentionally unrestricted
same-user host process; the certified isolation envelope must be explicit.
