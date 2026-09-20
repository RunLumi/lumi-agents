# ADR 0012: Developer-friendly open core with commercial separation

- Status: Accepted project policy; ownership and contract verification remain open.
- Date: 2026-09-20
- Extends: [ADR 0003](0003-licensing.md). Resolves its open DCO/CLA decision.

## Decision

Retain the unmodified Apache-2.0 license for all existing original public code,
including the desktop application. Add Apache inbound/outbound plus prospective
DCO certification, clear trademark guidance, an explicit package inventory,
distribution notices and checks. No mandatory copyright assignment or bespoke
CLA for ordinary patches. Keep future distinct commercial modules outside this
repo and require a rights/distribution review before their first publication.

The user's goal is developer use/contribution plus defensible commercial value,
not a specific mandated license identifier. This is intentionally different from
Lumi BI's source-available policy. The local execution runtime is a high-privilege,
extensible adoption surface; gratuitous licensing uncertainty would work against
its chosen role. This reasoning is a strategy judgment, not evidence of traction.

## Alternatives and tradeoffs

| Choice | Benefit | Why not the default here |
|---|---|---|
| Apache-2.0 core + separate commercial assets | Genuine open-source use, private extensions, explicit patent and commercial grants | Competitors can host and redistribute core; no mandatory upstream contribution. Accepted tradeoff. |
| AGPL application + commercial alternative | Reciprocity for covered distribution/modified network use | Compliant competitors can still charge; combined-work/extension obligations and additional rights management change the developer contract. |
| ELv2 application | Restricts substantial hosted/managed services and license-key circumvention | Not OSI open source; its hosting restriction is not a general ban on distributing competing desktop apps. |
| FSL/BSL or a custom non-compete | Time-limited or configurable competing-use restrictions | Different public promise, future conversion/configuration burden and more adoption/legal friction; not selected. |

Existing Apache grants cannot be retracted by replacing the root LICENSE. Apache
contributions do not become our exclusive property. This policy does not represent
a signed CLA, completed IP assignment or trademark registration. The substantive
terms of LICENSE, not FAQ prose, govern existing community rights.

## Implementation and acceptance

- All first-party Rust/npm packages declare Apache-2.0; root lock metadata matches.
- Product LICENSE/NOTICE and relevant existing notice inventories are included
  by direct web/desktop UI builds and mapped into Tauri resources/release artifacts.
  Complete third-party binary compliance remains a separate release obligation.
- An offline checker and rejection tests detect changed license text, missing
  packages, contradictory metadata and missing notice resources.
- The DCO job uses trusted base policy and checks new commit/co-author trailers;
  bootstrap does not claim any historical certification. No automatic sign-offs.
- The commercial scope is empty in this public tree. Community builds have no
  new paid-service dependency, runtime feature gate or privilege changes.

See [public guidance](../../LICENSING.md) and
[commercial/rights review](../licensing/commercial-and-rights-review.md).
Tests establish engineering consistency, not venture attractiveness or legal title.

## Sources reviewed 2026-09-20

- [Apache-2.0](https://www.apache.org/licenses/LICENSE-2.0.html).
- [Apache FAQ](https://www.apache.org/foundation/license-faq).
- [DCO 1.1](https://developercertificate.org/).
- [AGPLv3 section 13](https://opensource.org/license/agpl-3.0).
- [Elastic License 2.0](https://www.elastic.co/licensing/elastic-license).
- [FSL](https://fsl.software/) and [BSL 1.1](https://mariadb.com/bsl11/).
- [Open Source Definition](https://opensource.org/osd).
