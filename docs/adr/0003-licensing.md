# ADR 0003: Licensing and reuse

- Status: Accepted
- Date: 2026-09-18

## Decision

Keep this repository proprietary for now. Do not apply MIT to the whole repository merely because key upstream dependencies are MIT.

MIT permits commercial use, modification, redistribution, and sublicensing when its notice is preserved. It does not force Lumi's original code to use MIT.

If RunLumi later open-sources a clean runtime package, evaluate Apache-2.0 as the default because it includes an explicit patent grant and termination terms. Make that decision per package after confirming IP ownership and contribution policy.

## Reuse rules

- Prefer dependency/adapter integration over copied source.
- Preserve upstream copyright and license notices for copied or redistributed MIT code.
- Record exact versions, commits, checksums, and licenses for bundled binaries.
- No AGPL/GPL/unknown-license dependency in the distributed runtime without explicit review.
- Content/model licenses are reviewed separately from software licenses.
- Do not assume a repository's root license covers every optional model, asset, binary, or submodule.

## Current upstream notes

Cua's root and Cua Driver workspace declare MIT. Cua's README also calls out optional components with different licenses, including OmniParser under CC-BY-4.0 and optional Ultralytics under AGPL-3.0. Lumi will avoid those optional paths unless separately approved.

open-codex-computer-use declares MIT.

This ADR is an engineering dependency policy, not legal advice.
