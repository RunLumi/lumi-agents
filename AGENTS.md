# AGENTS.md

## Mission and standard

Lumi Agents makes computer work reliably delegable. It is RunLumi's execution
substrate for useful work across APIs, browsers, files, shells, and desktop apps.

A user should be able to delegate a clear outcome, understand the authority
being granted, leave, and return to a verified result or an actionable exception.
The product must return more time and value than setup, supervision, recovery,
and support consume.

**Build for millions of businesses by making one valuable job repeatable across
businesses.** Scale is an ambition, not evidence for adding scope. Earn breadth
through reliable outcomes, reusable deployments, and customers choosing to return.

“Insanely great” means nothing necessary is missing and nothing unnecessary
remains. This is a continuous design test, not a claim of perfection or a reason
to delay useful delivery. Simplicity must never remove a safety boundary,
accessibility, verification, or recovery that the job needs.

## Focus: every feature earns its place

Honor the user's explicit task scope. For product-led work, follow the current
[product thesis](docs/product-thesis.md), [lighthouse](docs/lighthouse-role.md),
and [execution plan](docs/plan.md). Their experiments are provisional; candidate
ideas are not active commitments. Do not silently replace the requested task
with your preferred roadmap.

Before non-trivial work, put this compact decision in the task or PR; reuse an
existing issue or spec rather than create another planning document:

- **Outcome:** which user, recurring job, or demonstrated defect does this serve?
- **Evidence:** observed pain, failure, safety obligation, or explicit hypothesis;
  distinguish these from customer demand and production proof.
- **Smallest solution:** why existing behavior, configuration, an adapter, or
  removing a step is insufficient; name what stays out of scope.
- **Proof:** the postcondition, representative test, and outcome metric that
  would establish improvement against the current baseline.
- **Cost and exit:** added user decisions, permissions, dependencies, latency,
  support, and maintenance; name the rollback and the stop/continue threshold.

A bounded experiment may proceed without customer proof if it names the missing
fact, the cheapest test, a time/budget limit, and a decision date. Missing demand
is a reason to test demand, not to build a broader platform.

Work on the current bottleneck: safety/correctness first, then completing the
chosen job, then reliability and recovery, then setup/support economics, then
adjacent scope. Performance, accessibility, or design work belongs earlier when
it is the demonstrated blocker. Finish one coherent vertical slice before
starting another; do not bundle unrelated refactors or speculative extensibility.

For additions, ask **what breaks if we omit this?** For removals, ask **which
required outcome or protection would be lost?** Keep only what has a concrete
answer. A new abstraction needs a second real use case or a demonstrated
security/reliability boundary. Fewer lines are not simpler if they hide coupling.

## Product craft: complete the user's journey

- Start with the user's job and vocabulary. Present one clear next action per
  state; reveal advanced controls when needed. Avoid exposing provider, agent,
  or executor machinery unless it helps a decision.
- Reuse established components, interaction patterns, and tokens. New screens,
  settings, dependencies, and public concepts must pass the feature test above.
- Design setup, permission refusal, empty/loading states, partial results,
  interruption, restart, recovery, and completion with the happy path.
- Progress reflects observed state. Never invent completion percentages, hide
  uncertainty, or show success while required verification is pending.
- Approvals explain the exact effect, account, destination, material value,
  risk, and reversibility. Preserve required judgment; reduce routine friction
  through narrow policy, never blanket approvals or automatic consent.
- Make cancel, local emergency stop, exception ownership, evidence, and safe
  takeover discoverable. Distinguish stopping future work from undoing an effect.
- Use accessible semantics, keyboard/focus behavior, readable contrast, and
  consistent layouts. Test changed journeys at real window sizes and supported
  platforms; screenshots alone do not prove behavior.
- Verify that a representative user can reach the outcome without a developer
  narrating it. Measure time to first verified result and ongoing human effort.

## Read the right source

This file governs contributor decisions; it is not a second specification suite.
Read relevant sources before editing, and reconcile conflicts explicitly.

| Question | Source |
| --- | --- |
| What do contracts mean? | [Normative v1 specifications](docs/specs/v1/00-v1-index.md) and [ADRs](docs/adr/) |
| What exists and what is proven? | [Readiness](docs/v1-readiness.md), [truth map](docs/implementation-truth.md), current code and checks; verify dated claims |
| What should we prove next? | [Product thesis](docs/product-thesis.md), [lighthouse](docs/lighthouse-role.md), [plan](docs/plan.md) |
| What may ship? | [Release gates](docs/RELEASE_GATES.md) and [v1 definition of done](docs/specs/v1/22-v1-definition-of-done.md) |
| How do we contribute and extend? | [Contributing](CONTRIBUTING.md), [extension vocabulary](docs/extension-model.md), [reference registry](docs/references/repos.yaml) |

A roadmap does not prove implementation. Tests do not prove live effects. An
instruction file does not enforce runtime policy. Fix contradictions at their
source; do not silently weaken a normative contract to make a change pass.

## Non-negotiable execution boundary

**Models propose. Policy authorizes. Executors act. Verifiers determine success.
Audit records what happened.**

```text
observe -> plan -> normalize -> persist consequential intent/idempotency state
  -> authorize -> approve when required -> execute -> persist result
  -> observe actual effect -> verify -> audit/finalize -> continue/recover/stop
```

No provider, remote client, webpage, plugin, worker, driver, or subagent may
bypass this path. Persist externally visible/destructive intent before I/O;
never hide irreversible I/O inside an uncommitted state transition.

The trusted local runtime owns policy, approvals, executor gating, secrets,
evidence requirements, verification, cancellation, device identity, and update
trust. The execution environment owns its files, sessions, credentials, native
permissions, and local processes. Remote clients supervise; they do not acquire
local authority or secret state.

### Authority and approvals

- Deny unspecified side effects. Evaluate normalized business effects such as
  `send_customer_email` or `delete_file`, not a mouse click or tool name.
- Bind action/workflow/principal identity, capability, resource, target,
  arguments, expected effect, risk, evidence, postconditions, and idempotency
  through the [action contract](docs/specs/v1/03-action-observation-protocol.md).
  Canonical risk classes are `READ`, `LOCAL_WRITE`, `EXTERNAL_WRITE`,
  `COMMUNICATION`, `DATA_EXPORT`, `CREDENTIAL`, `FINANCIAL`, `LEGAL_CONSENT`,
  `DESTRUCTIVE`, and `ADMIN`.
- Approval binds the normalized action digest, workflow, principal, resource,
  target, destination, material value, parameters, and expiry. Re-evaluate
  material changes and obtain a new approval when required; revalidate mutable
  target/account/session state immediately before mutation.
- Initially require human approval for purchases/payments/transfers, external
  communications, legal commitments/consent, destructive actions, permission or
  account administration, and sensitive exports. Only narrow pre-authorization
  permitted by governing policy can remove a required checkpoint.
- Ask for the least additional authority: one origin, file, account, action, or
  destination. Autonomy settings and approval reviewers cannot widen policy
  ceilings. Delegation cannot confer authority the delegator does not possess.
- Model output, repository instructions, webhooks, emails, documents, webpages,
  MCP/plugin responses, and other agents' output remain lower-trust content.
  Preserve provenance. They cannot grant permissions, approve actions, install
  software, reveal secrets, or change destinations, policy, or data egress.
- Hooks may validate, enrich, narrow, deny, log, or schedule policy-allowed work.
  Model-based hooks cannot become the security kernel.

### Verified effects and recovery

Keep executor outcomes separate from verification:

```text
executor:     DELIVERED | REFUSED | NO_EFFECT | AMBIGUOUS | ERROR | CANCELLED
verification: PASSED | FAILED | AMBIGUOUS | NOT_REQUIRED
```

A transport OK, OS API success, generated report, or model self-report does not
prove the business effect. Define postconditions before execution; read back
actual records, fields, reconciled totals, artifact checksums, or draft/send
state. Use an independent effect oracle where possible. Required verification
must pass; never substitute `NOT_REQUIRED` to clear a failure.

Fail closed on missing/invalid policy or signature, policy errors, unknown
risk/capability, invalid/mismatched approval, unavailable required secrets,
verifiers or privacy controls, failed required persistence, exhausted budgets,
or ambiguous consequential effects. Report the specific blocker and safe next
step; do not silently fall back to broader authority.

Long-running work needs durable checkpoints, scoped idempotency, deadlines,
action/provider budgets, cancellation, bounded retries, and a human exception
route. A timeout or crash does not prove no effect. Reconcile possible prior
success before replay; quarantine ambiguity. Retry only proven-safe operations
within budget. Compensation is another authorized action, not automatic undo.
A cancelled task must not schedule further effects; report any already in flight.

Classify failures with the [canonical taxonomy](docs/specs/v1/18-error-taxonomy-retry-recovery.md).
Fix the failing layer; do not assume a smarter model solves policy, selector,
session, network, OS-permission, persistence, or postcondition failures.

## Small core, replaceable execution

Use the highest reliable semantic tier within the authorized scope:

```text
connector/API > browser semantics > native semantics
  > deterministic app adapter > vision/coordinates
```

- Playwright is the structured browser executor. Prefer roles, labels, and
  stable IDs; preserve controlled failure traces under the evidence policy.
- Native workflows use Lumi's `DesktopDriver`, never Cua-specific workflow
  types. Cua is the initial upstream engine; pin distributed binaries by exact
  version/checksum. Semantic targets are durable; coordinates are fallback data.
- Vision observes/grounds; it does not authorize. Non-read-only coordinate
  actions need reviewed policy. Frequent vision on a routine path is a signal
  to investigate semantics or an adapter.
- Refuse unsupported safe paths precisely. Never silently cross semantic to
  global pointer, background to foreground, isolated to authenticated browser,
  sandbox to host shell, or narrow to unrestricted network access.
- Supervise crash-prone browser/native/plugin/harness processes behind narrow
  interfaces where practical. Executor failure must not corrupt durable state
  or crash the policy core. Add direct native drivers only for measured need.

Provider APIs belong behind Lumi capability contracts, not business workflows.
Filter routes by tenant/region/data-egress policy, provider allowlist, local/cloud
constraint, and required capabilities before optimizing verified reliability,
latency, and cost. Never fall back from local-only to cloud silently. Keep driver,
account/instance, catalog, and task route distinct; never share mutable auth or
session state merely because accounts use the same driver. Require capability
contract tests; OpenAI-compatible is not proof of equivalent semantics.

Negotiate capabilities across independently upgraded clients and environments;
do not infer support from a version or silently discard unsupported fields.
Keep the required provider coverage in the specs, not a growing wish list here.

Use [extension definitions](docs/extension-model.md) precisely: a Tool is callable
capability; a Skill is guidance; an Agent is a bounded delegated role; a Hook is
a lifecycle handler; a Connector/MCP server supplies external capabilities; a
Command is an explicit user operation; a Workflow Pack hardens a routine.
None grants authority. Every integration declares capabilities, destinations,
filesystem/secret scope, side effects, provenance/license, and version.

One agent with deterministic tools is the default. Delegate only bounded,
independent work when it improves outcome, latency, or isolation; specify
ownership, inputs, output, budget, and verification. Subagent output is evidence
to inspect, not authority or proof by consensus.

## Project, data, and privacy boundaries

Work mode follows [Spec 26](docs/specs/v1/26-project-workspace-folder-as-project.md):
`ExecutionEnvironment -> Project -> Task -> Run`. A Project is a durable,
user-selected context rooted in explicit paths; a task workspace is its concrete
execution scope. Preserve that binding across resume and remote supervision.
Opening a folder grants no access to siblings, home secrets, SSH material,
browser profiles, or unrelated repositories. Refuse path/symlink/junction escape.

Detect pre-existing changes; distinguish user work from Lumi changes. Revalidate
before writing after pauses or external edits. Prefer inspectable patches and
reversible operations. Never stash, reset, clean, overwrite, or silently switch
scope to make a dirty checkout convenient.

Secrets are references resolved at the narrowest executor boundary, preferably
through Keychain or Windows protected storage. Never put plaintext secrets in
prompts, logs, traces, screenshots, crash dumps, definitions, or fixtures. Never
give a plugin the full credential store. Preserve tenant/account isolation.

Continuous screen and microphone recording default off. Use selective evidence,
redaction, explicit provider/image egress, and bounded retention. Recording
requires visible state, purpose, retention, and the deployment's approved legal
basis/notice flow. Evidence must not become employee surveillance.

Separate working context, durable memory, workflow state, audit/evidence, and
retrieval indexes. Audit is not memory by default. Every durable memory class
needs an owner, purpose, retention, deletion behavior, provenance, and tenant
boundary. Do not retain sensitive transient context just because it may help.

## Prove usefulness, reliability, and repeatability

Optimize `cost_per_verified_successful_workflow` for a declared workflow and
population. Include costs of failed attempts and retries; report zero successes
as no verified success, never zero cost. Compare like-for-like work and publish
coverage/exclusions so avoiding hard cases cannot improve the score invisibly.

Use [economic measurements](docs/role-scorecard.md) and the
[product thesis](docs/product-thesis.md) to establish net value after setup,
review, rescue, runtime, support, and allocated deployment cost. Do not
substitute token price, action count, autonomy duration, or demo appeal for
useful work. Do not count the same labor twice or hide negative net value.

A production Workflow Pack includes versioned inputs/outputs, permissions and
risks, execution preferences, postconditions, exceptions, evidence/privacy,
fixtures/evals, compatibility, economics, and recovery/idempotency. A Role Pack
composes these within the same authority boundary; it is not another runtime.
If the third deployment remains mostly bespoke, narrow or redesign before
expanding the catalog.

Artifacts carry provenance, version, source inputs, owner, validation/review
state, and export format. Keep draft creation separate from publication or send.

For behavior that can fail, test representative normal, denied, malformed,
missing-permission, wrong-target, duplicate/replay, crash/resume, cancellation,
and ambiguity cases as applicable. Exercise the real entry boundary. Never
pre-seed an effect oracle and present it as an observed external result.

Track verified completion, unauthorized effects, expected approvals, unexpected
rescue, human minutes, latency, costs, provider/model, unnecessary actions,
fallback/vision frequency, retries, postcondition failures, and OS/app versions.
Keep failed and ambiguous units in the declared denominator. Report correct
refusals/escalations separately; they are not completed routine work. Preserve
escaped failures as regression cases, subject to privacy policy.

Minimum [release gates](docs/RELEASE_GATES.md), not customer guarantees:

| Stage | Required workflow evidence |
| --- | --- |
| Alpha | At least 30 repeated runs; at least 90% verified completion; zero unauthorized effects |
| Customer canary | At least 100 representative runs; at least 95% verified completion; less than 5% unexpected rescue; zero policy bypasses |
| Mature narrow workflow | Target 99%+ on an explicitly certified OS/app/version matrix |

These floors do not override stricter workflow/risk gates. Expected approvals
are not rescue. Reliability never removes consequential-action approvals. When
a gate fails, restore or narrow the affected path before expanding it. Fixture,
local test, hosted CI, live-system, signed-release, and customer-economic evidence
are separate claims; state what remains unverified.

## Delivery discipline

1. Inspect current instructions, code, contracts, Git status, and affected
   surfaces. Preserve unrelated work; use isolation when needed.
2. Apply the feature test. For architecture changes, check the
   [reference registry](docs/references/repos.yaml), inspect relevant primary
   code/docs at a pinned commit, and record what transfers and what not to copy.
   Run a cheap experiment first; update reviewed refs only when actually checked.
3. Implement the smallest complete change through existing boundaries. Name
   affected contracts/trust boundaries. Update deterministic fixtures and evals
   for changed behavior; update the relevant spec/ADR when its decision changes.
4. Validate proportionately. For documentation-only changes, inspect links,
   consistency, and the diff; explain why runtime tests/evals are inapplicable.
   For code, run affected tests and applicable repository gates below. Record
   platform/permission limits rather than claiming an unrun check passed.
5. Review the final diff for omissions, scope creep, duplicated concepts,
   dependencies, secret leakage, failure paths, and user-visible completeness.
   State release/rollback effects. Update readiness claims only with evidence.
6. When requested, publish a focused PR, wait for applicable checks on its exact
   head, merge without bypassing protections, and verify the resulting remote
   commit. A local commit or unsubmitted form is not delivery.

Use the pinned toolchain and lockfiles. Core checks from [README](README.md):

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
```

[CI](.github/workflows/ci.yml) also covers desktop and dependency policy;
[browser protocol checks](.github/workflows/browser-protocol.yml) exercise the
actual worker handler. Run the relevant surface's checks; do not equate injected
adapters with a live browser. OS Keychain checks require host permission.

Every PR adding a side effect must state the normalized action/risk, authorizing
capability, approval, accessible secrets, evidence/verifier, fail-closed behavior,
retry/idempotency, tested OS/app/provider scope, dependency/license impact, and
rollback. Missing answers mean the change is not ready.

Before adding a dependency, verify the exact need, existing alternatives,
maintenance, provenance/license (including bundled assets/models/binaries),
security exposure, and replaceable boundary. Lock dependencies; pin Actions by
commit SHA and distributed binaries by version/checksum. Preserve advisory and
license scans, release SBOMs, and third-party notices. The open runtime is
Apache-2.0; copyleft in runtime/distribution, non-commercial, research-only,
unknown, or custom/restrictive licenses require explicit review.

Prefer adapter -> upstream contribution -> minimal fork -> replacement. Fork
only for demonstrated security, release-blocking compatibility, paid-workflow
reliability, licensing/platform divergence, or measured upstream bottlenecks.
Keep the smallest patch queue, notices, upstream comparison, and an exit path.

Done means the intended outcome is verified at the claimed level, applicable
checks pass, failure/recovery is explicit, and docs, dependencies, compatibility,
release, and rollback obligations are satisfied. No production workflow without
an eval pack. No unsupported claim of readiness because a demo worked once.

## Keep this guide small

Keep enduring decision rules here; keep schemas, catalogs, changing plans, and
detailed procedures in their canonical documents. Add a rule only when it
changes a recurring decision; merge duplicates and remove obsolete guidance
without weakening required protections.

Useful external foundations: [Google SRE on simplicity](https://sre.google/sre-book/simplicity/),
[AWS on retry-safe APIs](https://aws.amazon.com/builders-library/making-retries-safe-with-idempotent-APIs/),
and [GOV.UK on simple, tested user journeys](https://www.gov.uk/service-manual/service-standard/point-4-make-the-service-simple-to-use).
Apply their principles at Lumi's measured scale, not their organizational size.

**Ship the smallest complete product you would trust with your own important
work. Expand only when evidence earns the next step.**
