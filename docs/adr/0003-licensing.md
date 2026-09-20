# ADR 0003: Licensing and open-core boundary

- Status: Accepted
- Date: 2026-09-18
- Supersedes: original proprietary-only licensing decision in this ADR

## Decision

License the Lumi Agents local runtime repository under **Apache License 2.0**.

Keep commercial control-plane functionality, managed fleet services, premium workflow packs, hosted analytics/ROI intelligence, deployment services, and proprietary customer adaptations outside this open-core repository unless explicitly released later.

## Why Apache-2.0

Lumi Agents is high-privilege software installed on employee computers. An auditable local runtime can improve trust, ecosystem adoption, and integration velocity.

Apache-2.0 remains permissive while adding an explicit patent grant and patent-termination mechanism. This is preferable to adopting MIT for Lumi-owned core code.

Using MIT-licensed dependencies does not require Lumi to choose MIT.

## Open-core boundary

Apache-2.0 core:

- action/protocol contracts;
- local runtime;
- policy engine;
- public SDK;
- driver interfaces;
- baseline provider adapters;
- workflow-pack schema;
- local audit/evidence primitives;
- eval contracts.

Potential proprietary layers:

- enterprise control plane;
- managed fleet;
- organization policy console;
- premium/vertical workflow packs;
- hosted analytics and ROI intelligence;
- managed deployment;
- proprietary customer-specific adaptations.

## Reuse rules

- Prefer dependency/adapter integration over copied source.
- Preserve upstream copyright/license notices.
- Record exact versions, commits, checksums, and licenses for bundled binaries.
- Generate an SBOM for distributed releases.
- Review software, model, asset, dataset, and binary licenses separately.
- Do not assume a repository root license covers every optional component.

## Restricted licenses

The distributed runtime does not accept AGPL, GPL in runtime-sensitive positions, SSPL-like, non-commercial, research-only, unknown, or field-of-use restricted dependencies without explicit legal/engineering review.

MPL/LGPL and unusual SDK/model licenses require case-by-case review.

## Current upstream notes

### Cua Driver

Cua root and Cua Driver workspace declare MIT.

Cua documents optional components with different licenses, including OmniParser content under CC-BY-4.0 and an optional Ultralytics path under AGPL-3.0. Those optional paths are not approved for the default Lumi distribution.

### open-codex-computer-use

The referenced project declares MIT.

## Contributions

Contributions intentionally submitted to this repository are accepted under Apache-2.0 unless a separate written agreement applies.

The prospective contribution policy is now resolved by [ADR 0012](0012-community-rights-commercial-boundary.md): Apache inbound/outbound plus DCO, with no mandatory assignment or bespoke CLA for ordinary contributions. Historical rights and special imports still require review.

This ADR is an engineering licensing decision, not legal advice.
