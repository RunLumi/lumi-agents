# Lumi Agents

**A local-first runtime for verified digital labor.**

Lumi aims to own bounded recurring operational work across APIs, browsers,
files, shells and native applications, with evidence and human exception control.
The unit of value is useful work verified complete with fewer human minutes and
lower fully loaded cost.

**Current status: engineering foundation; NO-GO for unattended customer operation
or V1 release.** The repository has 23 core crates, three synthetic Workflow Pack
examples and a desktop scaffold. They do not establish live reliability,
customer ROI or role replacement. See [readiness](docs/v1-readiness.md) and the
[implementation truth map](docs/implementation-truth.md).

## One trust boundary

```text
Models propose → policy authorizes → executors act
→ verifiers determine actual outcome → evidence records → economics measures
```

Execution preference: connector/API → browser semantics → native semantics →
app-specific adapter → vision/coordinates. Local policy is authoritative;
models, plugins, webpages and remote supervisors cannot grant themselves access.

Required properties include bounded authority, scoped approval, secret isolation,
idempotency, durable recovery, cancellation and minimized evidence. Transport
success is not effect success. Ambiguous work remains ambiguous.

## Product direction

Work mode supports novel work, exceptions and discovery. Its core local primitive
is **Folder-as-Project**: a user opens a folder/repository as a durable Project,
then Lumi can safely search, read, create, edit, move and delete files, run bounded
project-local commands, inspect Git, validate changes and resume tasks later.
Project roots are filesystem authority boundaries, not whole-machine access. See
[Spec 26](docs/specs/v1/26-project-workspace-folder-as-project.md).

Workflow Packs harden repeatable work with contracts and evaluations. Minimal Role
Packs will compose workflows into bounded operational responsibilities and
measurable queues.

The provisional lighthouse is Finance Operations reconciliation and exception
reporting, excluding autonomous money movement. Ads, HR and Legal Ops are also
under evaluation. Access, baseline and buyer evidence decide the wedge; existing
code does not prove a market. See the [lighthouse decision](docs/lighthouse-role.md).

The [roadmap](docs/roadmap.md) puts real-system canaries, human-time measurement,
role economics and independent deployment reuse ahead of horizontal feature
breadth. The [execution plan](docs/plan.md) links the active GitHub work graph.

## Repository

- `crates/`: protocol, policy, state, audit/verification, orchestration, execution,
  providers, secrets, workspaces, packs/evals, scheduler, extensions and UI contracts.
- `packs/`: synthetic examples; never production evidence.
- `workers/playwright/`: semantic browser worker.
- `apps/desktop/`: Tauri shell; separate build from the core workspace.
- `docs/specs/v1/`: normative contracts; `docs/adr/`: architecture decisions.

Provider engines stay replaceable. Four adapter families have hermetic contract
coverage; actual provider/workflow reliability must be measured inside each
customer's privacy and data-egress constraints.

## Development

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
```

Rust is pinned in `rust-toolchain.toml`. The OS Keychain integration test requires
host secure-store permission; a sandbox failure is not a passing test. Core CI
covers Ubuntu/macOS/Windows and dependency policy. It does not certify Tauri or
signed release artifacts.

## License and security

Lumi's open runtime is Apache-2.0. Preserve compatible upstream notices and
review dependency, binary and asset licenses separately. See [LICENSE](LICENSE),
[THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md), [SECURITY.md](SECURITY.md) and the
[security model](docs/security-model.md).

Commercial control-plane services and premium vertical work may remain outside
this open repository. The runtime's standard is verified outcomes, recoverable
failure and measurable value, not a convincing demo.
