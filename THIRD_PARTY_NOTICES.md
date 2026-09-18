# Third-party notices and dependency policy

Lumi Agents is licensed under Apache-2.0. Third-party components remain subject to their own license terms.

This file records upstream projects we have evaluated or may integrate. It is not a substitute for the license files shipped by an incorporated dependency.

## Approved upstream candidate: Cua Driver

- Project: `trycua/cua`, specifically `libs/cua-driver`
- Candidate version: `0.28.2`
- Upstream license: MIT
- Integration plan: pinned binary/SDK behind a Lumi-owned adapter, not a deep source fork for the MVP
- Shipping requirement: preserve MIT notice and verify release checksums
- Important boundary: Cua documents optional components with other licenses, including OmniParser assets under CC-BY-4.0 and an optional Ultralytics path under AGPL-3.0. Those optional paths are not approved for the default Lumi runtime.

Upstream: https://github.com/trycua/cua

## Reference project: open-codex-computer-use

- Project: `iFurySt/open-codex-computer-use`
- Upstream license: MIT
- Current use: architecture/reference and interoperability testing
- Copying substantial source requires preserving upstream notice and recording copied paths here

Upstream: https://github.com/iFurySt/open-codex-computer-use

## Browser automation

Playwright is the planned first-class semantic browser engine.

Its license and exact pinned dependency metadata must be recorded when it enters the build.

## Default dependency policy

Generally acceptable after exact-package review:

- Apache-2.0
- MIT
- BSD-2-Clause
- BSD-3-Clause
- ISC
- Zlib
- Unicode-3.0
- CC0-1.0

Case-by-case review:

- MPL-2.0
- LGPL
- CC-BY or other content/data licenses
- SDK-specific terms
- model licenses
- dependencies bundling executable/model artifacts

Not approved by default for the distributed runtime:

- AGPL
- GPL in runtime-sensitive/distributed positions
- SSPL-like licenses
- Commons Clause
- field-of-use restricted source-available licenses
- non-commercial/research-only licenses
- unknown/unlicensed code
- copied code with unclear provenance

Every vendored binary or source snapshot must record:

- upstream URL
- exact version/commit
- checksum
- license
- reason for use
- update owner
