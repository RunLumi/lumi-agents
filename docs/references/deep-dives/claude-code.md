# Deep Dive: Anthropic Claude Code

Reviewed: 2026-09-18  
Repository: https://github.com/anthropics/claude-code  
Reviewed commit: `31a3b00bef145a0393d9dbf840a98674fec07712`  
Observed repository license: proprietary, use subject to Anthropic Commercial Terms.

## Important source limitation

The public Claude Code repository exposes documentation, plugins, examples, changelog material, and extension contracts, but not the entire product implementation as an open-source runtime.

Therefore Lumi should learn from observable product contracts and extension design, not infer hidden internals.

Do **not** copy source from this repository into Lumi unless the applicable terms explicitly permit it.

## What Lumi should learn

### 1. Distinguish tools, skills, agents, hooks, commands, and integrations

Claude Code's plugin model separates:

- commands: explicit user actions;
- agents: delegated reasoning roles;
- skills: context-activated procedural knowledge;
- hooks: deterministic event handlers;
- MCP servers: external capability providers;
- scripts/assets: implementation support.

This separation is excellent.

**Apply to Lumi:** avoid calling everything a "tool" or "agent".

Recommended Lumi vocabulary:

- Tool: callable capability.
- Skill: reusable model guidance/knowledge activated by context.
- Agent: bounded delegated reasoning/execution role.
- Hook: deterministic handler triggered by runtime lifecycle events.
- Connector/MCP: external capability provider.
- Workflow Pack: production-grade recurring business automation.
- Command: explicit user-invoked operation.

### 2. Hooks are the deterministic counterpart to model judgment

Claude Code hooks can run on lifecycle events such as PreToolUse, PostToolUse, Stop, SubagentStop, SessionStart, SessionEnd, and UserPromptSubmit. PreToolUse can allow, deny, ask, or rewrite inputs.

**Apply to Lumi:** provide typed runtime hooks for deterministic local policy-adjacent automation, observability, validation, and context injection.

But:

- hooks cannot grant capabilities beyond policy;
- hook failures must have clear fail-open/fail-closed semantics;
- hooks should be scoped and versioned;
- external/community hooks are lower trust.

### 3. Skills should be lazy/contextual, not globally stuffed into prompts

Claude skills carry metadata describing when to use them and can include scripts, references, examples, and assets.

**Apply to Lumi:** Skills should be retrievable, scoped bundles rather than permanent prompt baggage.

A Skill package can include:

- metadata/trigger description;
- instructions;
- examples;
- reference docs;
- deterministic helper scripts;
- eval cases.

Workflow Packs may reference Skills, but a Skill is not itself a production workflow guarantee.

### 4. Extension packaging should be conventional and portable

Claude plugins use predictable layout, manifests, relative roots, and auto-discovery.

**Apply to Lumi:** create a small extension manifest and conventional directory layout. Do not require every extension to compile against internal Rust APIs.

Portability principle:

- no hardcoded install paths;
- explicit version;
- declared capabilities;
- declared hooks/tools/agents/skills;
- declared network/filesystem/secret needs;
- license/provenance metadata.

### 5. Organization policy must be able to constrain customization

Claude Code managed settings can restrict permission rules, hooks, marketplaces, and dangerous permission skipping. Enterprise distribution is designed around managed policy.

**Apply to Lumi:** organization policy should be an upper ceiling that user/project extensions cannot widen.

Policy layering:

```text
hard platform invariant
  ∩ organization managed policy
  ∩ user policy
  ∩ workflow/pack policy
  ∩ task temporary grant
```

Lower layers may narrow. They may not widen upper ceilings.

### 6. Independent specialized agents can help when they reduce correlated errors

Anthropic's public code-review plugin runs specialized reviewers in parallel and applies confidence filtering.

**Apply to Lumi:** multi-agent patterns are justified when:

- work partitions cleanly;
- independent perspectives reduce correlated misses;
- aggregation has an explicit verification/scoring contract;
- latency/cost remain acceptable.

Good examples:

- independent verification;
- research partitioning;
- code/security review;
- competing extraction with reconciliation.

Do not build agent swarms for routine execution.

### 7. Completion hooks are useful

Stop/SubagentStop hooks can validate task completeness before an agent exits.

**Apply to Lumi:** before task completion, run deterministic completion checks and postcondition verification. A "Stop hook" should never substitute for domain verification, but it is a useful lifecycle slot.

## Concrete architecture changes for Lumi

1. Add a canonical extension taxonomy.
2. Add typed lifecycle hooks.
3. Define Skill package format.
4. Add extension manifests with capability declarations.
5. Define organization policy as an immutable upper ceiling.
6. Add optional independent-reviewer pattern for selected evals/high ambiguity.
7. Keep Workflow Packs distinct from Skills.

## What not to copy blindly

- Claude Code's public repository is not an open-source runtime reference. Treat product behavior as reference, not reusable implementation.
- Prompt-based hooks are useful for heuristics but should not become Lumi's core security boundary.
- Model-selected skill activation is not deterministic enough for production workflow requirements by itself.
- Multi-agent review improves some tasks, but consensus is not proof.
