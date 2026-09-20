# Lumi Agents: open runtime, separately commercial services

Reviewed: 2026-09-20. This guide explains the project policy; it does not amend
[Apache License 2.0](LICENSE). The license and applicable third-party terms prevail.

## The decision

**The existing public Lumi Agents application and runtime remain Apache-2.0.**
Developers can learn, use it at work, modify it privately, build extensions, host
it, redistribute it and sell products or services using it, subject to that
license. No individual, revenue, employee-count or commercial-use fee is added.
Model-provider, infrastructure and optional paid-service costs are separate.

Commercial value is reserved in **separately developed, separately licensed
services and modules outside this repository**, not an undisclosed restriction
on the community edition. There are **no proprietary product modules in this
repository** under this policy. Product names in a roadmap do not establish one.
The machine-readable [scope inventory](docs/licensing/scope.json) records the
actual package boundary; it is not an alternative license.

## What developers may do

| Use | Community-code position |
|---|---|
| Personal projects, school, research and paid employment | Permitted under Apache-2.0; no business-size threshold. |
| Use your own model keys or supported local models | No extra Lumi source-license fee; provider terms/capabilities still apply. |
| Private patches and independently authored workflow packs | Permitted; no mandatory publication of modifications. |
| Paid freelance work, consulting, training and integrations | Permitted, including work for clients. |
| Distribute or commercially host a fork, including a competitor | Permitted subject to Apache terms, notices and applicable trademark law. |
| Submit fixes, docs, translations, tests, adapters or packs | Apache-2.0 inbound contribution plus the prospective DCO process below. |
| Access Lumi's separately operated paid service or unreleased code | Not granted by this repository's license; requires its separate terms/access. |
| Present a modified product as an official RunLumi release | Not authorized by this source license; see [trademarks](TRADEMARKS.md). |

This is not a non-compete license. Neither a NOTICE file, README, service
subscription nor package name silently adds a hosting ban or royalty. Forking
may compete with RunLumi; that tradeoff is accepted for the community edition.

## Ownership and contributions

Authors retain copyright in their contributions. The project receives the
Apache-2.0 grants, including its copyright, patent and sublicensing provisions,
subject to that license. Contributors and other recipients receive the same
public license. Incorporation into a commercial offering does not erase
third-party attribution or previously granted rights.

Ordinary community contributions do **not** require copyright assignment or a
bespoke CLA. Use the [Developer Certificate of Origin](DCO) as described in
[CONTRIBUTING.md](CONTRIBUTING.md). A DCO is a provenance certification, not an IP
assignment, blanket warranty of ownership, electronic signature verification or
permission to ignore someone else's license. Commercial use of Apache code does
not require a second CLA solely because revenue is involved.

A substantial external code import, employer-owned work, or a proposed license
exception needs review of actual rights. A special agreement is valid only when
actually executed by an authorized rights holder. A checked box, GitHub login,
commit email, AI-generated assertion or CI success cannot create that authority.

## Open and commercial boundaries

Keep the desktop/CLI runtime, local policy/approval enforcement, cancellation,
secrets handling, local audit, baseline adapters, protocols, SDK interfaces and
public examples usable without a proprietary runtime dependency. Security fixes,
local data export and essential safety controls are not paywall mechanisms.

New managed fleet operations, hosted coordination, organizational administration,
certified vertical content and contracted deployment/support may form separate
paid offerings. These are **commercial candidates, not shipped-feature claims**.
The [commercial and rights review](docs/licensing/commercial-and-rights-review.md)
sets approval, code provenance, customer-contract and evidence requirements.

Do not move already published code into a private folder and claim its Apache
grants disappeared. A new independently authored proprietary implementation must
stay outside this public repository until its own rights, license and release
scope have been approved. Public extension interfaces remain usable by third
parties; this policy does not prohibit alternative implementations.

## User work, data and independent extensions

RunLumi does not claim ownership of your files, private prompts, credentials,
business data or independently authored outputs merely because Lumi processes
or produces them. Running the agent does not automatically Apache-license your
project. This is not a guarantee that AI output is copyrightable or exclusive.
Rights in source material, copied/generated code, model outputs and provider
terms must be evaluated separately. Copying repository code into an output can
carry that code's license obligations. Independent extensions may choose their
own license; redistribution of Lumi components retains their applicable terms.
No permission to train on customer material is inferred from the source license.

## Distribution, marks and history

Retain LICENSE, NOTICE, required changed-file notices and relevant third-party
terms when distributing covered software. An on-screen promotional badge is not
an added license requirement. Third-party software, fonts, drivers, models and
datasets retain their own licenses. Existing package-level reviews still apply.

Use a distinct identity for independently distributed products, keep attribution
accurate, and do not imply official signing, support or endorsement. See
[TRADEMARKS.md](TRADEMARKS.md); copyright permissions in an artwork and trademark
permission are different questions. No registered trademark is asserted here.

Historical Apache grants remain in force under their terms. This policy does
not rewrite previous releases or claim ownership of community contributions.
A future license proposal needs an explicit owner decision and rights review;
other RunLumi repositories' licenses do not carry over to this repository.

## Verification and sources

Run `python3 scripts/check_licensing.py` and
`python3 -m unittest discover -s scripts -p 'test_licensing*.py'`.
These checks establish metadata/packaging consistency, not legal title or a
complete third-party compliance audit. Required branch-protection configuration
and any signed agreements must be verified separately.

Primary references, reviewed 2026-09-20:
- [Apache-2.0 terms](https://www.apache.org/licenses/LICENSE-2.0.html), sections 2–6 and 9.
- [Apache licensing FAQ](https://www.apache.org/foundation/license-faq).
- [Developer Certificate of Origin 1.1](https://developercertificate.org/).
- [Open Source Definition](https://opensource.org/osd).

For licensing/partnership questions, contact hello@runlumi.app. Specific contracts,
rights transfers and jurisdiction-dependent interpretations need qualified review.
