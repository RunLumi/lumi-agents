# Lumi Agents Specifications

## Current normative version

**v1** is the current normative specification suite.

Start here:

- [v1 index](./v1/00-v1-index.md)
- [v1 definition of done](./v1/22-v1-definition-of-done.md)
- [v1 implementation order](./v1/25-v1-implementation-order.md)
- [Project workspace / Folder-as-Project](./v1/26-project-workspace-folder-as-project.md)
- [Global and Project Automations](./v1/27-global-and-project-automations.md)
- [Project Skills and Plugins](./v1/28-project-skills-plugins.md)
- [Project Connections and Secrets](./v1/29-project-connections-secrets.md)

The v1 suite is deliberately numbered so architecture, code, tests, and reviews can reference stable spec IDs.

Example:

> Implements Spec 04 approval binding, Spec 11 postcondition verification, and Spec 26 Project root isolation.

## Version policy

- `v0` files are historical design sketches and are superseded by v1.
- Breaking normative changes after v1 SHOULD create a new major spec version or explicit migration.
- Minor clarifications MAY update v1 when they do not change externally observable contract semantics.

## Relationship to other docs

- `AGENTS.md` — engineering constitution and decision rules.
- `docs/roadmap.md` — product/engineering sequencing and strategic gates.
- `docs/architecture.md` — high-level architecture.
- `docs/adr/` — architecture decisions and why.
- `docs/specs/v1/` — normative contracts and v1 acceptance criteria.
- `docs/evals/` — test/eval implementation guidance.

If a code change affects a normative contract, the relevant spec and tests MUST change in the same PR.

## Work-mode Project rule

Folder-as-Project is a v1 core Work-mode capability, not a desktop convenience feature.

Implementations MUST preserve the distinction:

```text
ExecutionEnvironment
  -> Project
      -> Task
          -> Run
```

Project defines durable working context and authorized roots.

Task workspace defines the concrete execution scope used by a Task/Run.

See spec 26 for the normative contract.

## Automations and project extensions

Spec 27 specializes the existing Spec 13 scheduler and Spec 23 UX: the global
Automations menu and project-filtered view manage the same records. Neither
creates global execution authority. Read state never implies approval.

Specs 28–29 specialize Specs 14/17/24/26. Skills and plugin declarations live in
`.lumi/skills/` and `.lumi/plugins/`; actual secrets stay in protected storage
outside the project. Portable connection requirements are not live credentials
or grants. Runtime protection remains independent of `.gitignore`.

The [research note](../research/project-extensions-and-global-automations-2026-09-20.md)
separates source observations from Lumi design decisions. These specs add target
contracts and acceptance gates, not evidence of shipped runtime functionality.
