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
