# Desktop dependency source notices

The desktop app uses the unchanged MPL-2.0 packages below. The npm entry (`mtx-decompressor`, a transitive dependency of the approved `pptx-react-viewer` PPTX preview adapter) was approved by an explicit license-policy review on 2026-09-21 per the case-by-case MPL-2.0 rule. Lumi-owned files remain Apache-2.0. Preserve all package notices and include these source references with any external binary release. A signed release also requires its complete SBOM/license bundle; this list is not a release certification.

| Package | Version | Exact source archive | SHA-256 |
|---|---|---|---|
| cssparser | 0.36.0 | [crates.io source](https://static.crates.io/crates/cssparser/cssparser-0.36.0.crate) | `dae61cf9c0abb83bd659dab65b7e4e38d8236824c85f0f804f173567bda257d2` |
| cssparser-macros | 0.6.1 | [crates.io source](https://static.crates.io/crates/cssparser-macros/cssparser-macros-0.6.1.crate) | `13b588ba4ac1a99f7f2964d24b3d896ddc6bf847ee3855dbd4366f058cfcd331` |
| dtoa-short | 0.3.5 | [crates.io source](https://static.crates.io/crates/dtoa-short/dtoa-short-0.3.5.crate) | `cd1511a7b6a56299bd043a9c167a6d2bfb37bf84a6dfceaba651168adfb43c87` |
| option-ext | 0.2.0 | [crates.io source](https://static.crates.io/crates/option-ext/option-ext-0.2.0.crate) | `04744f49eae99ab78e0d5c0b603ab218f515ea8cfe5a456d7629ad883a3b6e7d` |
| selectors | 0.36.1 | [crates.io source](https://static.crates.io/crates/selectors/selectors-0.36.1.crate) | `c5d9c0c92a92d33f08817311cf3f2c29a3538a8240e94a6a3c622ce652d7e00c` |
| mtx-decompressor (npm) | 1.6.0 | [npm source archive](https://registry.npmjs.org/mtx-decompressor/-/mtx-decompressor-1.6.0.tgz) | `5030e71fdd2e3f6339e0d3fef9d2e1c6e5650f6418042f0ac3632f042137307b` |

The license text is available from [Mozilla](https://www.mozilla.org/en-US/MPL/2.0/). The exact upstream archives above contain the covered source used by the locked build; `selectors` carries MPL notices in its source headers. Release automation must retain and verify these archives against the listed checksums and provide recipients an accessible source route. If a covered file is modified, retain its notices and provide the corresponding modified covered source.

See [case-by-case review](../../docs/dependency-review-tauri.md). The app-only deny policy permits only the listed versions. The core workspace policy is unchanged; additional MPL versions/packages require a new review.
