# Tauri dependency license review

Status: case-by-case review complete for the exact app build dependencies; external release evidence remains incomplete.

Evidence date: 2026-09-19 (Asia/Ho_Chi_Minh)

This review covers the five MPL-2.0-only packages reported by the Tauri dependency graph at `/private/tmp/lumi-desktop-dependencies.json` and the exact entries in [`apps/desktop/src-tauri/Cargo.lock`](../apps/desktop/src-tauri/Cargo.lock). The graph identifies `apps/desktop/src-tauri` as the workspace root and `lumi-desktop-app` as its root package. The repository policy remains defined by [`deny.toml`](../deny.toml), [`THIRD_PARTY_NOTICES.md`](../THIRD_PARTY_NOTICES.md), and [ADR 0003](adr/0003-licensing.md).

## Decision

The app build policy in [`apps/desktop/deny.toml`](../apps/desktop/deny.toml) permits MPL-2.0 only for these five exact reviewed versions. [`apps/desktop/THIRD_PARTY_NOTICES.md`](../apps/desktop/THIRD_PARTY_NOTICES.md) supplies exact source archives and checksums. The root workspace allowlist is unchanged. External binary release remains blocked until the complete notice/source bundle, SBOM and artifact-bound checks described below are produced.

The MPL findings alone do not require removing the dependencies or blocking local/internal development. An externally distributed desktop binary remains release-blocked until the app-specific dependency scan, generated SBOM, third-party notices, and source-availability evidence are attached to the release artifact. This is an evidence gate, not a conclusion that MPL-2.0 is incompatible with Lumi's Apache-2.0 larger work.

## Exact packages and graph paths

The lockfile checksums below are the Cargo registry checksums from the app lockfile. The local manifest and source paths are recorded to make the review reproducible on this host.

| Package | Version | Cargo.lock checksum | License evidence | Role and path | Upstream |
| --- | --- | --- | --- | --- | --- |
| `cssparser` | `0.36.0` | `dae61cf9c0abb83bd659dab65b7e4e38d8236824c85f0f804f173567bda257d2` | `/Users/james/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cssparser-0.36.0/Cargo.toml` says `MPL-2.0`; `LICENSE`; MPL headers in `src/` | Build/proc-macro graph: `tauri-codegen → tauri-utils(build-2) → dom_query → cssparser`. It is also reached through `selectors`. Under the current app feature graph this is compile-time tooling, not a final app runtime dependency. | [servo/rust-cssparser](https://github.com/servo/rust-cssparser) |
| `cssparser-macros` | `0.6.1` | `13b588ba4ac1a99f7f2964d24b3d896ddc6bf847ee3855dbd4366f058cfcd331` | `/Users/james/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cssparser-macros-0.6.1/Cargo.toml` says `MPL-2.0`; `LICENSE`; `lib.rs` MPL header | Proc-macro dependency of `cssparser`; compile-time only in this graph. | [servo/rust-cssparser](https://github.com/servo/rust-cssparser) |
| `dtoa-short` | `0.3.5` | `cd1511a7b6a56299bd043a9c167a6d2bfb37bf84a6dfceaba651168adfb43c87` | `/Users/james/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/dtoa-short-0.3.5/Cargo.toml` says `MPL-2.0`; `LICENSE`; `src/lib.rs` MPL header | Dependency of `cssparser`; compile-time tooling path through Tauri's HTML-manipulation build feature. | [upsuper/dtoa-short](https://github.com/upsuper/dtoa-short) |
| `option-ext` | `0.2.0` | `04744f49eae99ab78e0d5c0b603ab218f515ea8cfe5a456d7629ad883a3b6e7d` | `/Users/james/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/option-ext-0.2.0/Cargo.toml` says `MPL-2.0`; `LICENSE.txt` | Runtime normal-dependency path: `tauri → tauri-runtime-wry → wry → dirs → dirs-sys → option-ext`. Treat this one as present in the distributed runtime's transitive license set. | [soc/option-ext](https://github.com/soc/option-ext) |
| `selectors` | `0.36.1` | `c5d9c0c92a92d33f08817311cf3f2c29a3538a8240e94a6a3c622ce652d7e00c` | `/Users/james/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/selectors-0.36.1/Cargo.toml` says `MPL-2.0`; MPL headers in `lib.rs`, `build.rs`, and source files; no standalone `LICENSE` file was present in the registry source directory | Build/proc-macro graph: `tauri-utils(build-2) → dom_query → selectors`. It is compile-time tooling under the current app feature graph, not a final app runtime dependency. | [servo/stylo](https://github.com/servo/stylo) |

The registry source base used for this review was `/Users/james/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/`. The `.cargo_vcs_info.json` files record upstream source revisions for the unpacked crates; the Cargo.lock checksums remain the release pin that must be used for artifact provenance.

The runtime/build distinction was checked with Cargo's feature graph. The four `cssparser`/`selectors` family packages are pulled by Tauri's `tauri-codegen`/`tauri-macros` and `tauri-build` `build-2` path, which enables `dom_query`. `option-ext` is pulled by the normal `wry` runtime path. A release SBOM should still enumerate both runtime and build dependencies because the build graph is part of reproducibility and supply-chain evidence.

## MPL-2.0 obligations relevant to this app

The authoritative sources are the [MPL-2.0 license text](https://www.mozilla.org/en-US/MPL/2.0/) and Mozilla's [MPL-2.0 FAQ](https://www.mozilla.org/en-US/MPL/2.0/FAQ/). The FAQ is guidance, not a substitute for the license or legal advice.

- MPL-2.0 grants rights to use, modify, distribute, and combine covered software in a larger work. Mozilla describes the copyleft as file-level, so it does not relicense unrelated Lumi files merely because they are linked into the same application.
- For external distribution of executable form, Section 3.2 requires that the covered MPL source form also be available and that recipients be told, by reasonable means, how to obtain it. The executable may carry different terms if those terms do not restrict the recipient's MPL source rights.
- If an MPL-covered file is modified, the modified file remains within the MPL source obligations. License notices in covered source may not be removed or materially altered. New Lumi files that contain no MPL code are not thereby converted to MPL.
- Internal/private use and distribution within the organization do not trigger the external-distribution obligations described in Mozilla's FAQ. A customer-facing or otherwise external desktop release does.
- Nothing in this review concludes that a particular distribution method satisfies every legal obligation. The release owner should obtain legal review for the actual customer, reseller, and source-offer flow.

No source modifications or copied source from these five crates were found in the Lumi repository. The current relationship is Cargo dependency integration; it is not a source fork or vendored snapshot.

## Source and notice strategy

1. Keep the app lockfile pinned. Do not replace the exact packages with floating versions or a broad registry exception.
2. Generate a release SBOM and dependency-license report from `apps/desktop/src-tauri/Cargo.lock`, including both normal/runtime and build/proc-macro packages.
3. Add a generated release notice bundle that names all five packages, versions, checksums, upstream URLs, and the canonical MPL-2.0 text. Preserve the package-provided license files where they exist. For `selectors`, which has no standalone license file in the unpacked registry source, include the canonical MPL text and identify its MPL source headers explicitly.
4. Provide a stable source-availability route for the exact MPL package source used by the release. The preferred implementation is to archive or otherwise retain the exact crates.io source archives identified by the lockfile checksums and publish the corresponding source links with the binary. A bare “available on GitHub” statement is insufficient for a reproducibility record unless it identifies the exact source revision.
5. Verify the final bundle from a clean checkout: the app lockfile, SBOM, notices, source references, and checksums must describe the same graph. Do not claim runtime-only coverage while omitting build/proc-macro packages from the reproducibility inventory.
6. If future work patches any of these crates or vendors their source, preserve MPL headers, record the modified files, and publish those covered files under the applicable MPL source terms. That would be a new review, separate from this unchanged-dependency assessment.

## What this review does not approve

This review does not cover unreviewed package versions, copied/modified upstream source, native system-library distribution, signing or store packaging. It does not turn a dependency graph into proof of a signed or distributable release. Other transitive licenses remain subject to the app-specific dependency-policy job.

The recommended policy shape is an app-scoped MPL-2.0 exception backed by exact package review and generated release evidence. If the app dependency-policy job cannot be made app-scoped, keep the release blocked until the policy check can distinguish this reviewed set from unreviewed MPL packages.

## Advisory triage (2026-09-19)

The app-scoped advisories scan fails on five RUSTSEC entries, all one root cause: the unmaintained `unic` crate family (`unic-char-range`, `unic-char-property`, `unic-ucd-ident`, `unic-ucd-version`), reached only via `tauri-utils v2.9.3 -> urlpattern v0.3.0`. Each advisory (RUSTSEC-2025-0075, -0080, -0081, -0098, -0100) is an unmaintained notice, not a vulnerability, and reports no safe upgrade: the fix belongs upstream to Tauri replacing `urlpattern`. They are therefore ignored in [`apps/desktop/deny.toml`](../apps/desktop/deny.toml) with exact IDs. Removing this ignore list is required review work on any Tauri bump that drops the `urlpattern` path; new RUSTSEC IDs must not be added to the list without individual triage recorded here.

## Validation record

- Read `/private/tmp/lumi-desktop-dependencies.json` (captured 2026-09-19 19:37:45); it reports the five packages as `MPL-2.0` and the Tauri app as the workspace root.
- Read the five exact entries in `apps/desktop/src-tauri/Cargo.lock`, including versions and checksums.
- Inspected local Cargo registry manifests, license files, source headers, and upstream repository metadata for each package.
- Ran `cargo tree --manifest-path apps/desktop/src-tauri/Cargo.toml -e features` and inverse dependency queries to classify the compile-time/build graph versus the normal runtime path.
- `cargo-deny` is not installed on this host, so this document does not claim a fresh deny pass. The parent implementation adds a tracked app lockfile, exact app-scoped exceptions and a separate CI scan; hosted results must be recorded after that PR runs.

This is an engineering dependency review and release-readiness recommendation, not a legal opinion or legal guarantee.
