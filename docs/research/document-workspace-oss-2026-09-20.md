# Document Workspace OSS decisions — 2026-09-20

Status: researched recommendation, not a benchmark or dependency approval.
Contract: [Spec 30](../specs/v1/30-document-workspace-preview-edit.md).

## Decision

Build a small React/TypeScript Document Workspace on Tauri with lazy, replaceable
format adapters. Keep file access, safe save, authorization and provenance in the
existing Rust host. No mandatory Office server, cloud viewer, Node-in-WebView or
whole-shell rewrite. Choose mature focused primitives for ordinary formats;
qualify newer native Office editors against actual preservation and security tests.

**Preview fidelity, editable feature coverage, preservation on save, and formula
calculation are four different capabilities.** No library name establishes all
four. Keep the original safe and expose a narrower honest feature set when needed.

## Evidence and current repository

Reviewed Lumi at `f0ed14aa6a3833e0520fdc0b1872b852bd9bb3d9`:

- `apps/desktop/README.md`: the frontend is vanilla HTML/CSS/JS, not React yet.
- `src-tauri/tauri.conf.json`: frontendDist points to `../src`; global Tauri bridge
  enabled and a small existing CSP. Do not weaken the main WebView for Office HTML.
- `src-tauri/src/lib.rs`: existing ProjectService and runtime own operations;
  emergency stop avoids waiting on the main runtime mutex.
- Specs 08/17/23/26/28/29 already cover files, artifacts, privacy, projects and
  extension/secret boundaries. Extend them, not a second file/credential framework.

This review inspected upstream docs, README/license information and selected
package metadata. It did not install these libraries, run hostile documents,
benchmark a release app or demonstrate round-trip fidelity. `Recommended` below
means implementation direction; every exact dependency still needs lockfile,
license/advisory, bundle and platform qualification before shipping.

## Recommended stack

| Job | Package / upstream | Observed license | Decision and limit |
| --- | --- | --- | --- |
| Text, Markdown source, JSON/YAML | CodeMirror 6 modular packages | MIT | Baseline editor; wrap directly in a small React adapter. Not a full Monaco/IDE feature set. |
| Markdown preview | react-markdown + remark-gfm | MIT | Baseline; raw HTML off, custom link/image policy, no MDX execution. |
| CSV/TSV I/O | papaparse | MIT | Baseline; worker/chunk parsing, literal strings by default, explicit formula-safe export. |
| Editable data grid | react-data-grid, Comcast | MIT | Baseline for CSV/XLSX; virtualized DOM and keyboard navigation. Not a workbook engine. Current README requires React 19.2+. |
| XLSX I/O | exceljs | MIT | Baseline bounded workbook profile. No formula calculation; imported complex features need preservation gates. |
| DOCX preview | docx-preview, VolodymyrBaydalka/docxjs | Apache-2.0 | Baseline best-effort layout. Only renderAsync is documented stable; not a write/edit API. |
| Native DOCX edit | @docx-editor.dev/core + @docx-editor.dev/react, EigenPal | Apache-2.0 for these packages | First qualification candidate, not yet a proven Lumi dependency. Pro/editor-api features are separate commercial packages. |
| PPTX preview/basic edit | pptx-react-viewer / pptx-viewer-core, ChristopherVR | Apache-2.0 root; separately licensed bundled components | First native-edit qualification candidate. Declared MPL-2.0 mtx-decompressor needs explicit license-policy review or verified exclusion. |
| PPTX preview fallback | Pagus | MIT | Qualification candidate for static SVG preview only, not native editing; still needs hostile-input and WebView tests. |
| Image crop interaction | react-easy-crop + browser Canvas | MIT for component | Baseline crop UI; application owns EXIF, pixel export, quality and save-copy behavior. |
| PDF preview | pdfjs-dist / PDF.js | Apache-2.0 | Baseline self-hosted worker/viewer components; read-only, scripting/active actions off. |
| HTML/SVG sanitization where needed | DOMPurify | Apache-2.0 OR MPL-2.0 | Elect Apache-2.0; defense in depth, not a network policy, CSS sandbox or IPC boundary. |

Sources for this table:

- [CodeMirror home/license and current development location](https://codemirror.net/)
- [CodeMirror development source](https://code.haverbeke.berlin/codemirror/dev)
- [react-markdown documentation/security/license](https://github.com/remarkjs/react-markdown)
- [remark-gfm](https://github.com/remarkjs/remark-gfm)
- [Papa Parse source/license](https://github.com/mholt/PapaParse) and [API](https://www.papaparse.com/docs)
- [react-data-grid](https://github.com/Comcast/react-data-grid) and [license](https://github.com/Comcast/react-data-grid/blob/main/LICENSE)
- [ExcelJS README](https://github.com/exceljs/exceljs/blob/master/README.md) and [license](https://github.com/exceljs/exceljs/blob/master/LICENSE)
- [docx-preview README](https://github.com/VolodymyrBaydalka/docxjs) and [license](https://github.com/VolodymyrBaydalka/docxjs/blob/master/LICENSE)
- [EigenPal DOCX editor](https://github.com/eigenpal/docx-editor), [React package](https://github.com/eigenpal/docx-editor/blob/main/packages/react/package.json), [product tiers](https://www.docx-editor.dev/pricing)
- [ChristopherVR PPTX toolkit](https://github.com/ChristopherVR/pptx-viewer) and [NOTICE](https://github.com/ChristopherVR/pptx-viewer/blob/main/NOTICE)
- [Pagus](https://github.com/pagus-kit/Pagus)
- [react-easy-crop](https://github.com/ValentinH/react-easy-crop)
- [PDF.js](https://mozilla.github.io/pdf.js/)
- [DOMPurify](https://github.com/cure53/DOMPurify)

The CodeMirror GitHub development mirror was archived after moving development;
do not mistake that for abandonment or pin an obsolete distribution from it.
The current grid README also flags a Vite CSS-minification issue with light-dark
syntax. Verify the chosen Vite/grid combination and explicitly apply Lumi's light
scheme rather than copying an upstream demo theme.

## DOCX: preserve the document, not just its text

`docx-preview` converts DOCX to browser HTML with known layout/field limitations.
Disable its default-enabled HTML altChunk rendering and do not build an editor
on undocumented internal parse structures. This supplies a useful early preview
without requiring the newer edit engine at startup.

EigenPal is a meaningful native-edit option rather than an HTML-conversion
workaround. The React package metadata reviewed declared `@docx-editor.dev/react`
2.21.0, Apache-2.0, React 18/19 peers and core ~2.21.0 (blob
`004257ca3d5b8024447b873cc95f19c142e1d0fb`). This is an observed source snapshot,
not a pin recommendation or proof of a published artifact. The actual selected
package tarball and its transitive notices need validation. Do not rely on older
package names or assume the commercial `pro` / `editor-api` packages are OSS.

Mammoth is an optional semantic extraction fallback, not a lossless editor. Its
README explicitly warns that output is not sanitized and that it favors semantic
HTML over visual reproduction. A DOCX -> HTML -> rich-text -> DOCX pipeline can
lose information even when every conversion succeeds. Limit it to clearly named
simplified copies, not overwriting imported originals.

Source: [Mammoth README, security and BSD-2-Clause license](https://github.com/mwilliamson/mammoth.js).

## XLSX: a grid is not Excel

ExcelJS reads/writes workbook data and styles but explicitly does not calculate
formula results. `fullCalcOnLoad` requests recalculation by another application;
it does not calculate in Lumi. Therefore Spec 30 keeps formula cells read-only
initially, identifies saved caches, invalidates stale totals after input edits,
and rejects a save path unable to represent recalculation needs correctly.

Unsupported features may not survive a library's deserialize/serialize cycle.
Charts, pivots, queries, links, macros and complex workbook structures must not be
dropped silently. Qualify a simple literal-cell editing profile and keep complex
workbooks read-only/external unless a tested native-preservation path exists.

`react-data-grid` supplies presentation and cell interactions. ExcelJS supplies
file I/O. Keep source-cell identity separate from view sorting/filtering. A
Canvas grid such as [Glide Data Grid](https://github.com/glideapps/glide-data-grid)
is an alternative if actual measurements justify it, not another default runtime.

SheetJS CE remains a possible alternative parser, not a second dependency to load
beside ExcelJS. Recheck official distribution, licensing and exact feature needs
before replacing the adapter: [SheetJS installation](https://docs.sheetjs.com/docs/getting-started/installation/).

## PPTX: qualify the modern candidate instead of pretending generation is editing

ChristopherVR's toolkit advertises browser parsing, rendering, native edits and
save-back, including React bindings. Its README distinguishes preservation from
rendering limitations and documents font/export differences. This makes it worth
a bounded test, not an automatic claim that arbitrary PowerPoint decks are safe.
The package has a large feature surface; measure the actual imported subset and
exclude optional 3D, collaboration and media-export paths unless required.

The root Apache-2.0 license is not the entire license story: NOTICE separately
identifies components such as MPL-2.0 mtx-decompressor. Lumi's policy requires
explicit review for that distribution or proof that it is excluded. A switch in
runtime options is not proof a dependency disappeared from the shipped bundle.

Pagus is an alternate static renderer, not an editing replacement. Inspect its
SVG/foreignObject output in the same unprivileged viewer; MIT does not certify
security or fidelity. Do not integrate both engines before a comparison warrants it.

[PptxGenJS](https://github.com/gitbrent/PptxGenJS) is useful for creating new decks,
including template-driven generation. It is not the chosen importer/editor for
arbitrary existing PPTX files. Likewise, a viewer demo with contentEditable is
not evidence that edited bytes safely round-trip to PowerPoint.

## Why not embed a complete Office suite now?

| Option | Evidence | Decision for this feature |
| --- | --- | --- |
| ONLYOFFICE DocumentServer | Full document-server stack; community AGPL-3.0 | Not the default lightweight offline Tauri dependency; separate architecture/license decision. |
| SuperDoc | Native DOCX editor with AGPL/commercial licensing | Technically relevant alternative, but cannot silently enter a permissive-only dependency plan. |
| Univer | Apache-licensed core, while documented XLSX import/export is a Pro server-dependent feature | Do not confuse core grid UI with free browser-only Office round-trip support. |
| User-installed LibreOffice | Local conversion available; MPL and other distributed-component obligations | Optional explicit sandboxed converter/external editor, not mandatory bundle or exact Microsoft renderer. |
| 501351981/pptx-preview | License permits some package use but restricts source modification/redistribution | Exclude as a default OSS dependency under current Lumi policy; do not infer MIT from a demo or npm name. |

Primary references:

- [ONLYOFFICE DocumentServer](https://github.com/ONLYOFFICE/DocumentServer)
- [SuperDoc editor documentation](https://docs.superdoc.dev/editor/)
- [Univer import/export: Pro and server requirement](https://docs.univer.ai/guides/sheets/features/import-export)
- [LibreOffice licensing](https://www.libreoffice.org/licenses/)
- [pptx-preview license text](https://github.com/501351981/pptx-preview/blob/main/LICENSE)

The strongest alternative is to require an existing Office editor for every
change: lowest implementation burden and better compatibility for complex files.
The tradeoff is repeated context switching for small corrections. Lumi should
own ordinary edits while retaining that escape hatch, not replace the full suite.

## Tauri integration and security rationale

Tauri capabilities and CSP are distinct controls. Do not grant the document
renderer the main app's IPC authority. Custom Rust commands also need explicit
scoping; a plugin-permission list alone does not settle the boundary. Test the
actual installed WKWebView/WebView2 behavior, not only a Chromium development tab.

- [Tauri capabilities](https://v2.tauri.app/security/capabilities/)
- [Tauri CSP](https://v2.tauri.app/security/csp/)
- [Tauri scoped opener](https://v2.tauri.app/plugin/opener/)

Use locally bundled engine code, worker/assets and content-addressed caches.
Documents cannot fetch external images/fonts/templates or call file/secrets APIs.
A worker, iframe, Shadow DOM or DOMPurify alone is not the entire sandbox. Native
helpers additionally require real process/filesystem/network controls. Preview
must never become a way to read ignored project credentials or run an Office macro.

## Adoption gates and cheapest qualification test

Before adding a package: resolve an exact release and integrity, review the
source/maintainer/release channel, apply dependency age/advisory policy, inspect
transitive licenses/fonts/WASM/binaries and preserve required notices. No floating
`@latest` installer or runtime CDN. Update lockfiles/SBOM/notices in the actual
implementation PR, not fabricated dependency declarations in this docs-only PR.

Time-box initial DOCX and PPTX spikes to roughly two engineering days each:

1. Use a licensed/synthetic corpus spanning ordinary, complex and hostile files;
   record source hashes and exact engine artifact versions.
2. Compare no-op and one-edit output: semantic change, required relationships,
   untouched opaque parts, reference rendering and unsupported-feature reporting.
3. Test offline networking/IPC denial, cancel, missing fonts, Vietnamese IME,
   actual native WebViews, bundle size and memory—not screenshots alone.
4. Reject unsafe/restricted/lossy paths or retain them as labeled preview-only.
   Do not solve failure by writing a new general Office engine.

Implementation order: host session/save boundary and text/Markdown/images/CSV/PDF;
DOCX preview/XLSX data view; native Office edit qualification; integrated recovery,
accessibility and performance. Spec 30 records proposed budgets, not measurements.
This produces the fastest useful slice while making the hard uncertainty visible:
**can an ordinary edit survive in the user's real document without losing anything
we promised to preserve?**
