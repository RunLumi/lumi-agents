# Third-party notices and dependency policy

This file records upstream projects we have evaluated or may integrate. It is not a substitute for the license files shipped by an incorporated dependency.

## Approved upstream candidate: Cua Driver

- Project: `trycua/cua`, specifically `libs/cua-driver`
- Candidate version: `0.28.2`
- Upstream license: MIT
- Integration plan: pinned external binary/SDK behind a Lumi adapter, not a source fork for the MVP.
- Shipping requirement: preserve the MIT notice and verify release checksums.
- Important boundary: Cua documents optional components with other licenses, including OmniParser assets under CC-BY-4.0 and an optional Ultralytics path under AGPL-3.0. Those optional paths are **not approved** for the Lumi distributed runtime.

Upstream: https://github.com/trycua/cua

## Reference project: open-codex-computer-use

- Project: `iFurySt/open-codex-computer-use`
- Upstream license: MIT
- Current use: architecture/reference and interoperability testing only.
- Do not copy substantial source into Lumi without preserving the upstream copyright/license notice and recording the copied paths here.

Upstream: https://github.com/iFurySt/open-codex-computer-use

## Default dependency policy

Allowed without special escalation when the exact package is reviewed: MIT, Apache-2.0, BSD-2-Clause, BSD-3-Clause, ISC, Zlib, Unicode-3.0, and CC0-1.0.

Case-by-case review: MPL-2.0, LGPL, content/data licenses such as CC-BY, SDK-specific terms, model licenses, and dependencies that bundle executable or model artifacts.

Not approved by default for the distributed runtime: AGPL, GPL, SSPL, Commons Clause, source-available licenses with field-of-use restrictions, unknown/unlicensed code, or copied code with unclear provenance.

Every vendored binary or source snapshot must record: upstream URL, exact version/commit, checksum, license, reason for use, and update owner.
