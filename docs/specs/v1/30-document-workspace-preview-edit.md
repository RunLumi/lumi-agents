# 30 — Document Workspace: Preview & Basic Editing v1

Status: Normative target; not an implementation or format-fidelity claim.
Reviewed: 2026-09-20.

Extends [08 — Files/artifacts](08-files-shell-artifacts.md),
[23 — UX](23-user-experience-handoff.md), and
[26 — Projects](26-project-workspace-folder-as-project.md).
[17 — Security](17-security-privacy-secrets.md) and
[29 — Secrets](29-project-connections-secrets.md) remain ceilings.
Library evidence and adoption decisions: [OSS research](../../research/document-workspace-oss-2026-09-20.md).

## 30.1 Outcome

Open a project file or agent-produced artifact, understand its contents, make a
small justified correction, and save a verified result without leaving Lumi for
routine work or silently damaging the original.

Use one Document Workspace from Files, Artifacts, task results and Automation
Review Queue. These entry points reference the same resource/version; they do
not copy files into separate viewer stores. Preview does not require an LLM.

Deliver a focused document surface, not an Office suite, IDE, design tool or
collaborative document server. Full Office fidelity, macros, formula-engine
parity, arbitrary slide-master editing and real-time collaboration are non-goals.

## 30.2 Existing baseline and React integration

At reviewed commit `f0ed14aa6a3833e0520fdc0b1872b852bd9bb3d9`,
`apps/desktop/README.md` and `src-tauri/tauri.conf.json` describe a Tauri v2 shell
serving vanilla HTML/CSS/JS from `src/`, not an existing React/Vite application.
The backend already delegates project operations through `ProjectService`.

Target new document components in React + TypeScript with a minimal Vite build.
Mount a document-workspace island or a dedicated bundled viewer entry first;
do not rewrite the entire shell as a prerequisite. Establish the build/IPC
boundary in a focused implementation PR, keeping existing navigation working.
Use the React version required by the selected exact dependency releases.
React is a view layer, not a second file store or policy engine. Tauri does not
supply Node.js APIs to browser libraries; use browser distributions and workers,
not `fs`, `child_process`, Electron assumptions or a bundled Node runtime.

## 30.3 Format support is explicit

The table defines bounded feature targets, not support already shipped. Register
capabilities per adapter, exact version, platform and document feature profile.
Until an editing adapter passes qualification, expose preview/external handoff
and label the editing capability unavailable rather than claiming completion.

| Format | Preview target | Basic edit target | Save contract |
| --- | --- | --- | --- |
| TXT, MD, JSON, YAML, source text | Text; Markdown rendered/source/split | Text, undo/redo, find/replace | Preserve encoding/BOM/line endings; format only explicitly |
| CSV, TSV | Virtualized grid and source | Literal cells, bounded paste, row edits | Preserve delimiter/header/encoding semantics; explicit safe export |
| PNG, JPEG, WebP | Fit/zoom/pan, dimensions | Crop, rotate, flip, resize | New image copy by default; explicit format/quality |
| GIF, SVG | Animation or safe inert image preview | SVG source text only; no vector editor | No silent animation/vector flattening |
| PDF | Pages, text selection/search, thumbnails | No PDF content editing in v1 | Preview/open externally; no redaction claim |
| DOCX | Best-effort document layout plus extracted-text fallback | Text, basic formatting, lists, simple table cells via qualified native adapter | Edited copy by default; preservation qualification required to replace |
| XLSX | Data-oriented sheets/grid, formats, formulas/cached values | Literal cells and simple formatting in qualified workbooks | Edited copy; formulas preserved, recalculation limits visible |
| PPTX | Static slides/thumbnails, text and notes | Existing text/notes, basic text styling; simple object edits only when qualified | Edited copy; unsupported slide features stay explicit |
| DOC, XLS, PPT; macro-enabled, encrypted, protected or signed Office files | Identified unsupported/special state | Not edited by the baseline pipeline | Explicit external app or separately approved converter |

Never rename extensions to convert formats. Detect content/container type rather
than trust a filename. A PDF preview converted from a PPTX is not an editable
PPTX. A spreadsheet grid is not an Excel print-layout renderer.

## 30.4 Minimal user experience

Selection opens a preview pane; Open expands it to a document tab. Support Space
for Quick Look where appropriate, Escape to close, keyboard navigation and
Cmd/Ctrl+S. Closing a dirty tab requires Save / Save copy / Discard / Cancel.

```text
Files / Artifacts / Automation result
  -> Document Workspace
     filename · project · source version · capability/fidelity status
     Preview | Edit                  Save / Save copy · Open externally
     document / sheet / slide / image surface
     source / validation / changes / limitations (progressive disclosure)
```

Show only format-relevant controls. Avoid a ribbon, nested card dashboards or a
permanently open inspector. Use DESIGN.md paper/white surfaces, Geist for chrome
and ICON.md glyphs. Preserve document fonts/colors/layout; do not restyle the
customer's document to Lumi branding. Disclose substituted/unavailable fonts.
No runtime font-CDN requests; distribute only appropriately licensed fonts.

Important states: loading, partial preview, unsupported feature, too large,
password/protected, read-only permission, unsaved, saving, saved, conflict,
validation failed, renderer unavailable and source changed. Empty output is not
success. A missing diagram/page/chart should have an explicit limitation.

## 30.5 File identity and adapter boundary

The trusted host opens an authorized resource into a document session containing:

- tenant/principal, registered project, environment and workspace identity;
- opaque file/artifact reference, canonical target and source content hash;
- content type, size and supported feature profile;
- adapter ID/version and capability set;
- preview-fidelity, edit-preservation and calculation status separately;
- source version, draft version, dirty state and validation results.

Illustrative operations, not currently implemented IPC names:

```text
open_document(resource_ref, requested_mode)
read_document_chunk(session_ref, offset, length)
preview_document(session_ref, view_request)
prepare_document_save(session_ref, expected_hash, draft_ref, save_mode)
commit_document_save(prepared_save_ref)
close_document(session_ref)
```

Every command authenticates the caller and checks its session/project scope.
Possessing a handle is not authorization. No arbitrary host path or URL from a
renderer. Chunk lengths, writes, output formats and destination grants are
bounded. Use binary transfer or a session-scoped streaming protocol for large
content; do not repeatedly serialize entire documents as base64/JSON IPC.

Adapters expose inspect/preview/dispose and, only when qualified, edit/serialize/
validate. Keep package-specific models behind the adapter. Reuse project,
workspace, artifact, state and policy primitives; no parallel document filesystem.

## 30.6 Rendering is a trust boundary

Treat documents, generated HTML, SVG, embedded fonts, XML relationships and
plugin output as hostile input. Parsing does not execute document instructions.

Use a separate unprivileged viewer WebView/entry for document-supplied HTML/SVG
and complex Office engines. Give it no general Tauri filesystem, shell, opener,
credential or application-command capability. Explicitly scope custom commands
as well as plugin permissions: absence of a plugin capability alone is not proof
that custom IPC is inaccessible. Validate a narrow host-mediated message bridge.
A file cannot request another file by guessing a path or session reference.

Prefer inert sanitized output. When a renderer requires JavaScript, execute only
bundled renderer code, never embedded scripts/macros/HTML event handlers. An
iframe/Shadow DOM used for styling alone is not isolation. Test the actual IPC
boundary on WKWebView and WebView2, including frame-origin attacks. An error
boundary or Web Worker improves resilience, not privilege isolation.

Default no network for previews, external relationship resolution, CSS URLs,
fonts, telemetry or media. Allow only session-bound local resources. Disallow
`file:`, `javascript:`, executable attachments, arbitrary custom schemes and
remote templates. Link activation becomes an explicit user action mediated by
the host. Never enable whole-home `assetProtocol` scope or wildcard remote IPC.

Bundle workers/fonts/assets locally. Keep the existing shell CSP narrow; add
only demonstrated worker/blob/WASM requirements to the isolated viewer policy.
Do not enable broad `unsafe-eval` or remote script CDNs to make an engine work.
DOMPurify is a sanitizer, not a network policy or CSS isolation substitute.

## 30.7 Text and Markdown

Use CodeMirror 6 for editing; rendered Markdown uses react-markdown + remark-gfm.
Keep raw HTML disabled and intercept links/images through the resource policy.
Do not execute MDX, embedded JavaScript or fenced code. Diagrams and math are
optional lazy adapters, not prerequisites.

Source is authoritative: switching tabs never rewrites Markdown, frontmatter or
whitespace. Preserve BOM, newline style and final-newline behavior. If encoding
cannot round-trip, remain read-only or offer explicit UTF-8 Save copy. Do not
replace undecodable bytes silently. Large logs have bounded/ranged read mode.

## 30.8 DOCX

Default lightweight preview: `docx-preview`, stable `renderAsync` API only.
Disable HTML altChunks (`renderAltChunks: false`), remote relationships and
unreviewed embedded-font behavior. The library's experimental parsed model is
not a stable editing/serialization contract.

First native-edit qualification candidate: Apache-licensed
`@docx-editor.dev/core` + `@docx-editor.dev/react` from EigenPal. Restrict v1 UI to
basic text/paragraph/list/table edits. Do not assume its Pro review, collaboration
or document-automation API is included in the OSS packages. Verify exact package
licenses/dependencies and safe offline configuration before adoption.

Complex sections, fields, tracked changes, comments, content controls, equations,
floating objects and embedded material require explicit preservation tests. A
legal document with tracked changes cannot be silently flattened into ordinary
text. Missing fonts/pagination discrepancies stay visible.

Fallback: Mammoth for explicitly labeled semantic extraction/conversion only;
sanitize its HTML and disable external-file access. DOCX -> HTML -> rich text ->
DOCX is a conversion, not lossless editing. Offer it only as a clearly labeled
simplified copy; never replace the original through this path.

## 30.9 XLSX and delimited tables

Use ExcelJS browser I/O behind a worker plus a virtualized React grid. Start with
`react-data-grid` (Comcast); it is grid UI, not an XLSX parser or formula engine.
Do not load both ExcelJS and SheetJS by default.

Show sheet tabs, row/column addresses, hidden-sheet indicators, selected-cell
value/formula and number format. Preserve empty cells, literal types, leading
zeros, long identifiers, boolean/error cells, dates and the 1900/1904 date system.
Handle Vietnamese separators without silently changing stored values. Sorting
and filtering a view do not reorder the saved workbook. Edit commands bind to
stable sheet/cell coordinates, not a filtered row index.

Initial edit profile: literal text/number/boolean/date values, bounded paste and
simple number/text formatting. Formula cells are read-only in v1. Structural
row/column edits in formula-bearing workbooks, charts, pivots, slicers, external
links, Power Query, macros and protection are outside the baseline edit profile.
Do not strip them merely to produce an editable grid. Unknown relevant features
mean preview-only, explicit simplified copy, or external editing.

ExcelJS does not calculate formula results. On open label cached values
`Last saved result`; an absent cache is `Not calculated`, never zero. After any
input edit, mark workbook formula results stale conservatively unless a qualified
dependency-aware engine proves otherwise. Do not publish stale totals as current.
Preserve formula expressions, remove/mark invalid caches using a tested writer
path and request recalculation on open. Recalculation flags do not themselves
calculate anything. If invalidation cannot be serialized reliably, block that
save and offer external editing. Never implement formulas with JavaScript eval.

CSV/TSV uses Papa Parse with dynamic typing off by default. Preserve duplicate
headers and ragged rows rather than auto-renaming/dropping data. Surface parse
errors. Raw save preserves content semantics; a separate spreadsheet-safe export
can escape formula-like strings with a disclosed transformation. Do not silently
change legitimate negative values or IDs under the name of sanitization.

## 30.10 PPTX

Qualify `pptx-react-viewer` / `pptx-viewer-core` (ChristopherVR) for static preview
and bounded native edits. Upstream advertises import/edit/save, but that is not
Lumi's evidence of preservation. Its root Apache license does not override
separate bundled licenses, including the declared MPL-2.0 component. Required
license approval or a genuinely excluded dependency is a release gate.

Preview: slide rail, fit/zoom, slide number, selectable text where supported and
speaker notes. Do not autoplay video, audio, animation or embedded objects.
Basic edits: existing text and notes, simple character formatting; moving or
resizing an ordinary text/image object only when that operation passes the
supported profile. Masters, themes, SmartArt/chart editing, animation editing,
embedded OLE and arbitrary layout authoring are not v1 requirements.

Keep unsupported objects indicated and preserve them on export when claiming
native edit. Re-render the exact saved candidate to catch overflow and missing
content. Text expansion can overflow a box even when the ZIP/XML is valid.

Fallback candidate: Pagus for static best-effort preview, subject to the same
hostile-file and platform tests; it is not an editing substitute. PptxGenJS may
create new/template-driven decks, but must not be presented as an arbitrary
existing-PPTX importer/editor. A failed edit qualification leaves the limitation
visible; do not build a handwritten general OOXML editor to hide it.

## 30.11 Images and PDF

Images: browser image display with bounded dimensions; react-easy-crop provides
crop interaction, Canvas or a reviewed Rust image adapter performs pixel export.
Crop coordinates map to original-resolution pixels, not the displayed thumbnail.
Normalize EXIF orientation before geometry and test color/alpha handling. Export
copy is default; show JPEG quality and transparency loss. Remove sensitive EXIF
location by default on new exports and disclose metadata stripping. No hidden
remote image fetches. SVG stays inert; do not inject it into the privileged DOM.
Animated GIF/unsupported HEIC/TIFF paths must not silently become one-frame PNGs.

PDF: self-hosted PDF.js (`pdfjs-dist`) with matched worker/assets, lazy pages,
text layer and search. Scripting, launch actions, attachments, auto-navigation,
form submission and remote content are disabled. No browser PDF-plugin assumption
across Tauri platforms. Preview annotations are not secure redaction. PDF editing,
signature validation and OCR are separate future capabilities.

## 30.12 Safe save and conflict handling

Opening/previewing never writes the source. No-op Save preserves original bytes
or does nothing; do not reserialize a file just because it was opened.

Imported Office files default to `Save edited copy`. Replacing the source is
available only for a qualified adapter/document/operation profile, with a recovery
copy and explicit user intent. Save copy still needs fidelity disclosure and
validation; it is not permission to misrepresent a damaged result.

```text
inspect source + hash -> edit draft -> prepare candidate in private staging
-> validate format/preservation -> reauthorize destination and current source
-> publish atomically through host file service -> verify bytes -> record change
```

All UI/agent saves use this path. No direct renderer disk writes. Reuse canonical
path/symlink defenses, stale-write checks and per-file serialization. Use a stable
file handle/identity and revalidation; a watcher event alone is not a lock. Reject
source changes since the edit base. Do not hold the global project mutex while
parsing/rendering. Document interaction must not block emergency stop.

External editor/agent changes: reload clean previews; preserve dirty drafts and
show Reload / Save copy / Compare when conflicted. Never automatically merge
binary Office files. Last-second concurrent external modification remains a
platform limitation to test and disclose; no claim of universal filesystem CAS.

Stage on the destination filesystem for atomic publication. Preserve ACLs and
file metadata where supported; Windows sharing violations, full disk and failed
rename remain unsaved with recovery available. A crash between publishing and
audit finalization is reconciled by the prepared-save ID and output hash, not
blind write replay. Restore/overwrite is a separately authorized operation.

Drafts/caches are private runtime state, not credentials, Git artifacts or generic
memory. Use bounded retention, protection appropriate to source sensitivity and
per-document recovery. Do not persist a draft when source policy forbids it.

## 30.13 Fidelity and verification

Separate these states:

- preview: rendered / approximate / partial / unavailable;
- editing: native-qualified / simplified-copy / unavailable;
- persistence: dirty / staged / saved / conflict / failed;
- calculation: not-applicable / cached / stale / qualified-recalculated;
- artifact/business verification: existing Spec 11 obligations.

A renderer returning successfully proves neither preservation nor business
correctness. Save validation checks type/container, parseability, expected edit,
required parts/relationships, page/sheet/slide inventory and unsupported-feature
preservation. Reopen candidate bytes before publication. Maintain a corpus of
no-op, one-edit and unsupported-feature files from Word/Excel/PowerPoint and
LibreOffice exports. Check semantic structure and visual output where relevant.

Do not promise byte-identical edited ZIP files: container metadata/order may
change. Untouched opaque parts should remain byte-equal after decompression;
modeled-part changes need an explicit expected semantic diff. Unexplained lost
parts, unresolved relationships or hidden content loss blocks replacement.
If preservation is unproven, state it and retain the original.

## 30.14 Parser limits and resource control

Sniff bytes and bounded OOXML package metadata before heavy rendering. Refuse
path traversal, duplicate/colliding ZIP members, symlink entries, encrypted
containers, decompression bombs, excessive XML depth/entities and oversized
embedded assets. Disable DTD/external entity resolution. Never extract arbitrary
package paths into a project. Enforce limits while decompressing, not only against
untrusted declared ZIP sizes.

Initial tunable ceilings: 25 MiB compressed Office input, 200 MiB total expanded
bytes, 10,000 ZIP entries, 20 megapixels per decoded image, 100,000 populated
spreadsheet cells for the basic editable profile. These are conservative product
limits to measure, not library guarantees. Sparse maximum-range dimensions must
not cause dense allocation. Unsupported/over-limit documents retain explicit
read-only/external options; never silently truncate and label complete.

Use cancelable workers for parsing and chunked/virtualized page, slide and grid
rendering. Stop stale jobs when switching files; release object URLs, canvases,
worker processes and buffers on disposal. Limit simultaneous heavyweight viewers.
Worker isolation does not provide a hard process-memory cap; use a sandboxed
helper where required by risk/size and refuse a path that cannot honor its limits.

Cache by tenant/project/resource hash + adapter version + render/font settings,
never just filename. Cache hits require current authorization. Revocation and
project removal clear accessible derived content under retention policy. No
cross-project confidential thumbnails or global-search indexing by default.

## 30.15 Performance targets

Proposed acceptance budgets, measured in release builds on a recorded 8 GB Windows
reference laptop and an Apple Silicon Mac; not measured claims in this spec:

| Operation | Initial p95 target |
| --- | --- |
| Open/Cancel feedback and toolbar response | <=100 ms |
| First useful view: <=1 MiB text or <=5 MiB raster image | <=1 s |
| First useful Office view: <=5 MiB, <=20 pages/slides or 10k populated cells | <=3 s warm; <=5 s cold |
| Cancel parser/viewer job | <=1 s without freezing shell |
| Typical typing/grid edit interaction | <=50 ms; no repeated >100 ms main-thread tasks |

Record package JS/CSS/WASM/font size, cold-load time, peak memory and large-file
failure behavior for each adapter. Keep Office engines out of the initial shell
bundle and defer import until used. Prefer the smallest useful core/subpath;
do not ship collaboration, AI SDKs, media exporters or all languages by default.
A 15-second interactive parse deadline should yield a clear fallback rather than
an indefinite spinner. Expensive optional conversion has a separate visible limit.

## 30.16 Optional full-fidelity escape hatch

Always offer explicit Open externally for an authorized resource through a scoped
host opener. Do not claim an installed Office application exists. Observe returned
file changes with the same conflict rules. No auto-launch from document content.

An optional user-installed LibreOffice conversion adapter may generate a local
PDF preview for difficult/legacy files. It is not a required bundled dependency,
not exact Microsoft rendering, and not a live editing engine. Use an isolated
temporary profile, copied input, explicit output directory, disabled macros and
updates, denied network, sandbox/resource limits and a trusted executable path.
`--headless` alone is not a sandbox. Do not auto-install or bundle it without
separate licensing/distribution review. No hidden Google/Microsoft online viewer
uploads or mandatory document server.

## 30.17 Agent and Automation integration

Agent-generated files open through the same workspace. Human edits create a new
resource version and invalidate prior relevant verification. An agent must not
overwrite an open human draft or treat the rendered preview as the source file.
An optional Ask Lumi action sends only authorized selected content under provider
policy; merely opening a document never sends it to an LLM.

Document text remains data, not an instruction to access secrets, install a
renderer or run a macro. Rich document adapters are reviewed installed code under
Specs 28–29, never downloaded in response to embedded document instructions.
Scheduled output can remain DRAFT/READY_FOR_REVIEW while preview is available;
looking at it or successfully rendering it does not approve publication.

## 30.18 Required acceptance corpus

Test through actual Tauri entrypoints, not only injected JavaScript mocks:

1. Same file/version opened from Files, Artifacts and Review Queue; no duplicate store.
2. Text/Markdown edits preserve encoding, newlines, frontmatter and Vietnamese IME.
3. Markdown/HTML/SVG links, scripts, CSS URLs and document relationships cause no network or privileged IPC.
4. DOCX paragraphs/lists/tables/images, headers/footers, Vietnamese; complex tracked-change/field file takes honest fallback.
5. XLSX exact literal/date/type edits; leading-zero IDs, 1904 dates, hidden sheets, merged cells and formula caches tested.
6. Changing an input never presents old totals as fresh; invalid cache writer blocks save.
7. Filtering/sorting the grid cannot redirect a write to the wrong source cell.
8. CSV duplicate headers/ragged rows/quotes/newlines and formula-injection-aware export without silent raw-save mutation.
9. PPTX text/notes edits, overflow, missing fonts, slide/master/media preservation and unsupported-object warnings.
10. Image EXIF orientation, crop pixel mapping, alpha/quality, oversized decode and no accidental GIF flattening.
11. PDF lazy render/search works offline; active actions cannot run or escape.
12. No-op save unchanged; candidate reopens; unchanged opaque OOXML parts survive or replacement is blocked.
13. Dirty-draft conflict with human/agent writer; source swap/symlink; rejected project/secret path; protected-file refusal.
14. Disk full, Windows file locked, cancel, crash before/after publish and restart recovery preserve original and draft.
15. ZIP/XML bombs, malformed/renamed/encrypted/macro-enabled files fail boundedly without side effects.
16. Viewer cannot call custom app IPC, enumerate files, resolve credentials or open arbitrary URLs; revoked sessions stop serving bytes.
17. Worker/adapter failure, cache isolation/invalidation and repeated open/close do not leak memory or freeze emergency stop.
18. Actual WKWebView and WebView2 tests: bundled assets/CSP/workers, keyboard/IME, HiDPI, focus, screen-reader paths and size budgets.

Public fixtures are synthetic/licensed; private Ads/customer documents are never
uploaded to public CI. Office reference renders can be produced in a separately
licensed QA environment. Record artifact/source hashes and renderer versions.

## 30.19 Delivery sequence and stop rules

A. Host document session, preview isolation, staged save/conflict and minimal
React build. Prove text/Markdown/images/CSV/PDF first with no Office-server dependency.
B. DOCX preview and XLSX data view; qualify basic XLSX edits and formula staleness.
C. Time-box native DOCX/PPTX candidate evaluation before production adoption:
real-file corpus, no-network behavior, package/transitive licenses, payload cost,
preservation and platform tests. Integrate only bounded passing operations.
D. Complete task/artifact/review integration, recovery, accessibility and release
measurements. Optional external converter is a separate later decision.

A candidate with unresolvable licensing, required network, unsafe IPC, silent data
loss or unacceptable resource use is rejected or retained as preview-only. Do not
weaken the contract, conceal failure behind prompts or rewrite an Office engine.
A narrower phased release must list missing edit capabilities; it cannot claim
all-format basic editing until DOCX/XLSX/PPTX acceptance cases genuinely pass.

## 30.20 Done

The feature is complete at its declared support matrix when a user can preview,
make a bounded correction and inspect a validated saved result while the original,
project boundaries, credentials and concurrent work remain protected. The exact
adapter licenses/versions, known losses and tested platform matrix are documented.
Passing repository CI for this specification does not establish that outcome.
