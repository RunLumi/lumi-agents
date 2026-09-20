# Website work

Ship a fast, credible introduction to Lumi Agents, not a simulation of a released
product. DESIGN.md, ICON.md and the repo readiness documents are authoritative.

Keep this Astro package static and independent of desktop/runtime builds. Prefer
semantic HTML plus small progressive enhancements. Preserve keyboard, no-JavaScript
and reduced-motion paths. No fake customer proof, working-agent claims or releases.

Use the Node pin and `npm ci --ignore-scripts`. Run build, Node tests, browser tests
and dependency audit. Inspect real desktop and mobile renders, not source alone.
Keep generated lockfiles, font licenses and CSP/cache behavior correct. DNS changes,
production deployment and external sending are separate actions, not implied by a
successful build. Never commit credentials, generated dist or dependency directories.

Discovery changes share `src/data/discovery.mjs` facts between visible HTML and
llms.txt; FAQ schema uses the same `faqs` data as the visible answers. Keep RunLumi
(publisher) distinct from Lumi Agents (product). Preserve demo/readiness limits,
strict CSP hashes, preview noindex and 404 schema exclusion. Do not invent authors,
ratings, prices or release dates to improve a checker score. Verify built output
and rendered FAQ parity; do not claim a new GEO score without a deployed recheck.
