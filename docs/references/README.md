# Reference Repositories

This directory is Lumi Agents' **learning radar**.

Reference repositories are not dependencies by default. They are places we watch for architectural patterns, product ideas, failure modes, security lessons, packaging techniques, and evidence that our assumptions are changing.

The goal is to learn continuously without cargo-culting.

## Rules

1. **Reference != dependency.** Never copy code or add a dependency because it appears here.
2. **Extract principles, not shapes.** Understand why a design works before adopting it.
3. **Record what not to copy.** Every serious review should include anti-learnings.
4. **Prefer primary evidence.** Read architecture docs, contracts, tests, release notes, and code, not only README marketing.
5. **Track exact review points.** Record the commit/release reviewed so future agents can diff meaningfully.
6. **Architecture changes require an ADR.** A reference can trigger investigation, not silently rewrite Lumi's design.
7. **License before reuse.** Any code reuse/dependency decision still follows THIRD_PARTY_NOTICES.md and docs/adr/0003-licensing.md.

## Review cadence

### Tier A — weekly skim, monthly deep review

Repositories that directly influence core architecture or execution:

- pingdotgg/t3code
- trycua/cua
- iFurySt/open-codex-computer-use
- microsoft/playwright
- openai/codex
- anthropics/claude-code

### Tier B — monthly skim, quarterly deep review

Adjacent agent/runtime ecosystems:

- browser-use/browser-use
- OpenHands/OpenHands
- aaif-goose/goose
- modelcontextprotocol/typescript-sdk
- e2b-dev/E2B

## Review template

For each deep review, answer:

- What changed since last review?
- What user/problem are they optimizing for?
- Which architectural boundary changed?
- What evidence suggests the change works?
- What can Lumi reuse as a principle?
- What should Lumi explicitly not copy?
- Does this change any load-bearing assumption in docs/roadmap.md?
- Does it justify an experiment, issue, or ADR?
- What is the cheapest test before adopting it?

Update repos.yaml with the reviewed commit/date and short learning summary.

## T3 Code review — 2026-09-18

Repository: https://github.com/pingdotgg/t3code

Reviewed commit: `9ea9c3d5d2c444133e3ddff40eecf38737951589`

License observed at reviewed commit: MIT.

### What Lumi should learn

#### 1. The environment owns the work

T3 Code keeps provider processes, terminals, Git, project files, credentials, and machine state on the environment/server that owns the workspace. Remote clients control it over authenticated RPC instead of pretending the remote client owns those resources.

**Apply to Lumi:** formalize a `DeviceEnvironment` / execution-environment identity early. A phone/web control surface may supervise work, but filesystem, app sessions, local credentials, and native automation remain owned by the machine executing the workflow.

This strengthens our hybrid local/cloud model.

#### 2. Independently version clients and environments

T3 Code uses typed RPC contracts and capability negotiation so web, desktop, mobile, and servers can upgrade independently.

**Apply to Lumi:** protocol versions should advertise capabilities, not rely on synchronized app releases. Missing capabilities must degrade explicitly rather than guess.

This will matter when Lumi has desktop, web, mobile approval surfaces, managed devices, and older runtime versions.

#### 3. Record intent before doing side effects

T3 Code's orchestration architecture records intent/state in an event log before reactors perform external work. Command acknowledgements mean intent committed, not that the side effect finished.

**Apply to Lumi:** keep a simpler version of this principle:
- persist durable task/action intent;
- assign idempotency state;
- perform side effect;
- record result;
- verify postcondition.

Do **not** adopt full event sourcing everywhere merely because T3 does. We need the durability property, not necessarily the same machinery.

#### 4. Provider differences belong at the adapter boundary

T3 normalizes provider commands/events and keeps account/session/capability quirks inside adapters. Different accounts of the same provider are separate instances with isolated mutable state.

**Apply to Lumi:** strengthen our provider-neutral model layer with:
- provider driver vs provider instance separation;
- per-instance credentials/config/model catalog;
- canonical runtime events;
- capability negotiation;
- provider-specific usage normalization at the adapter edge.

This is directly relevant to Issue #8.

#### 5. Remote control should not put the cloud relay in the hot path

T3 Connect uses its relay for identity, environment discovery, credentials, tunnels, and notifications. Normal app/WebSocket traffic goes directly to the environment after connection.

**Apply to Lumi:** long term, prefer:
- cloud control plane for discovery, policy, fleet, scheduling and signaling;
- direct or environment-terminated encrypted execution traffic when practical;
- short-lived scoped credentials;
- remote clients never receiving the employee machine's long-lived secrets.

This can improve latency, privacy, cost, and outage isolation.

#### 6. Native/unsafe components should be process-isolated

T3 intentionally keeps risky native modules out of the Electron main process, moving them into child processes/workers with deadlines so a crash or stall does not take down the app.

**Apply to Lumi:** preserve the Rust policy core as the small trusted kernel. Run Playwright, Cua, plugin/MCP bridges, and eventually risky native helpers in supervised processes where practical.

This is stronger than simply calling them "lower trust" in documentation.

#### 7. Checkpoints are a product primitive

T3 separates turn completion from checkpoint settlement and uses hidden Git refs to snapshot coding work without polluting user branches.

**Apply to Lumi:** Work mode needs explicit checkpoint/restore semantics. For Git work, hidden refs are attractive. For general files, use a different snapshot mechanism. Do not equate checkpoint with "copy the whole folder".

#### 8. Tests should wait for states, not time

T3 uses durable receipts/drainable-worker concepts rather than sleeps and polling for async correctness.

**Apply to Lumi:** deterministic test harnesses should expose observable lifecycle states and drains. Time-based sleeps should be a smell.

#### 9. Multi-surface support is an architecture decision

T3 treats web, desktop, and mobile as first-class surfaces sharing runtime/domain contracts.

**Apply to Lumi:** do not duplicate task/approval state logic inside the future Tauri UI. Keep shared contracts so future mobile/web supervisors can control the same execution environment.

### What Lumi should NOT copy blindly

- T3 primarily orchestrates existing coding-agent harnesses. Lumi must still own business action semantics, policy, verification, workflow packs, and cost-center economics.
- T3's event-sourced architecture is mature but can be heavy. We should adopt durable intent + idempotency before committing to event sourcing every domain object.
- T3 uses Electron because its product history and UI stack justify it. Lumi already chose a Rust core and should still prefer Tauri for our employee desktop shell unless evidence changes.
- T3's provider model includes local provider CLIs/subscriptions. Lumi should support that pattern where valuable, but direct API/local-model adapters remain important for general Work mode and controlled Workflow mode.
- Remote control convenience must not weaken Lumi's local policy authority.

### Concrete follow-ups for Lumi

1. Add `ExecutionEnvironment` identity/capability concepts to Action Protocol planning.
2. Include provider **driver vs instance** separation in Issue #8.
3. Add canonical provider runtime events/usage normalization.
4. Add durable-intent-before-side-effect requirement to the orchestrator implementation.
5. Design remote control so control-plane relay is not automatically the data hot path.
6. Make risky executor adapters supervised child processes where feasible.
7. Add checkpoint/restore as an explicit Work-mode roadmap primitive.
