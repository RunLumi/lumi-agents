# Commercial boundary and rights review

Decision owner: RunLumi project owner. Reviewed: 2026-09-20.
Status: operating policy and review checklist, **not an executed agreement, legal
title opinion, investor endorsement or evidence that commercial modules ship**.
[LICENSING.md](../../LICENSING.md) owns public-use guidance.

## Business design

The auditable local agent is the distribution and trust surface. The proposed
paid value is reliable organizational operation: managed fleet rollout, hosted
coordination, administration, private team services, certified vertical packs,
support and deployment. Charge for those distinct deliverables, not permission
to use or compete with the Apache community code.

This is a business hypothesis. A restrictive license alone neither establishes
product-market fit nor makes a company venture-fundable. Developer adoption and
retained, profitable paid use are different measures. Product/community/value fit
are useful separate tests, as discussed in [a16z's commercialization framework](https://a16z.com/open-source-from-community-to-commercialization/).
That is an investor perspective, not an investment promise or a required license.

The strongest counterargument: a permissively licensed fork can compete with our
community product. We accept that exposure rather than describing Apache as
anti-cloning protection. Keep legitimately confidential commercial implementation
outside the public repo, improve distribution and reliability, and demonstrate
paid value that is not just a repackaged local binary. Independent rivals may
still build similar features. Trademarks protect identity, not feature exclusivity.

## Release a commercial module only after this review

1. Identify a separate deliverable, its owner, source repository, distribution
   and users' paid value. A name in a roadmap is not an implemented private asset.
2. Before putting source in a public repo/package, review authorship and all
   imported code, models, data and assets. Choose its own reviewed commercial
   terms. Do not apply a public root license accidentally and attempt to retract it.
3. Keep public interfaces and local safety enforcement in the community edition.
   Paid code may use documented interfaces but must not bypass local authority,
   require proprietary code for community builds or disable emergency stop/export.
4. Declare build inputs, dependency notices, compatibility, customer data rights,
   support/update obligations and an exit path. A package label is not a legal
   boundary; actual copied/linked code and distribution matter.
5. Do not copy a published Apache module into a private repo and claim exclusive
   rights over that code. Preserve the Apache portion's terms and attributions.
   Commercial terms govern distinct rights/services, not retroactive removal of
   community rights. Do not import Lumi BI's ELv2 code by default into this core.

Private modules stay in separate repositories. This change does not create one,
relicense any published file, impose paid feature checks or publish a registry
package. The public proprietary-scope inventory is deliberately empty.

## Contribution model and optional special agreements

Use Apache inbound/outbound plus DCO for normal contributions. Contributors retain
title; Apache already includes commercial-use/sublicensing grants subject to its
conditions. A DCO is not a bespoke relicensing license or an assignment. Do not
claim exclusive ownership of all submitted code or rely on a blanket retroactive CLA.

For a substantial code donation, employee/contractor work, customer-funded IP or
rights not covered by existing grants, have counsel review the actual agreements.
Use an additional license grant or assignment only where necessary and executed by
an authorized rights holder. Do not burden every typo fix with a speculative CLA.
No legal agreement or consent is executed by these docs or by CI.

## Private diligence record before financing or exclusive licensing

| Evidence to obtain | Current evidence level / action |
|---|---|
| Legal contracting entity, registration and authorized signatory | Not established by a GitHub organization. Verify the intended company and signatory privately. |
| Founder-created IP and transfers to the company | Public attribution is not an assignment. Record actual signed transfers where needed. |
| Employee, contractor and customer-funded code rights | Obtain applicable IP clauses, schedules and exceptions; do not infer from commit emails. |
| External contributions and imported components | Preserve Apache grants, DCO trails prospectively, notices and source provenance. Review historical contributions; do not invent past DCO/CLA consent. |
| Third-party software, fonts, drivers, models and datasets | Maintain artifact-specific SBOM, licenses, modifications/source obligations and redistribution evidence. Existing notice lists are not a complete legal audit. |
| Community vs commercial code boundary | Review actual source and build graph for accidental publication or unapproved license mixing. |
| Brand and distribution identity | Check authorship, signing-key custody, domains and actual trademark rights/registrations. No registration is asserted by TRADEMARKS.md. |
| Customer material and generated artifacts | Define ownership, processing permission, retention, model-provider terms and exit/export; no training right inferred. |
| Paid offering evidence | Retained use, contracts, support burden, gross margin and concentration; stars/downloads alone do not establish this. |

Keep documents and personal information in a private data room. Public records may
link to an access-controlled evidence ID and status, never upload contracts here.
Baseline inspected: `46d91abfb0649ee08ecf48c88c8d7e99855d56bd`. A scan of Git history
and contributor-policy text is not a chain-of-title certification. Existing rights
need human review even when authors appear related to the project owner.

## Seven-day falsification test

Ask three independent developers to explain internal use, paid consulting,
private modifications and fork rights from LICENSING.md. Pass only when all can
answer without a custom permission negotiation. Discuss one named paid
organizational workflow with two plausible buyers; record a concrete access/trial
or payment commitment, not praise for being open source. These are proposed
small-sample decision tests, not market claims or proof of legal validity.

Reconsider packaging when buyers only need the free local runtime, paid support
cost erases margin, or community users cannot build/use the project independently.
Do not respond to weak demand by quietly withdrawing already granted freedoms.

Counsel review is needed before exclusive IP warranties, custom source licenses,
commercial trademark permissions or signed partner/customer commitments. Contact:
hello@runlumi.app. No price, SLA or indemnity is promised by this file.

## Existing dependency evidence gap

At the inspected baseline, `workers/playwright/package.json` has no committed
`package-lock.json`. Its Apache package metadata is checked; a narrowly listed
exception records the absent lock without changing the dependency graph. A
reviewed lock and browser-worker release audit remain required for a reproducible
distributed dependency claim. This licensing change does not certify them.
