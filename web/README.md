# Lumi Agents — public website

Vietnamese, static Astro landing page for **https://agents.runlumi.app**.
This is a separate build from the Rust runtime and desktop app. It does not run
agents, connect user accounts, collect form submissions, or imply a released v1.

## Develop and verify

Use the Node version in `.node-version`.

```sh
cd web
npm install --ignore-scripts --before=2026-09-13T00:00:00Z # first lock bootstrap only
# Once package-lock.json is present: npm ci --ignore-scripts
npm run dev
npm run build
npm test
npx --no-install playwright install chromium
npm run test:e2e
```

`build` generates static HTML/CSS/JS, bundled self-hosted fonts, sample artifacts,
robots.txt, sitemap.xml, a 404 page, and Pages security headers. Browser tests
apply the generated CSP because `astro preview` does not implement `_headers`.
They cover 320 / 375 / 768 / 1440 px, keyboard tabs, downloads, native FAQ/menu,
no-JavaScript content, reduced motion, overflow and axe accessibility checks.
A test existing in source is not a passing result: inspect the actual CI run.

## Cloudflare Pages — Git integration

Create a **Pages** project connected to this repository after review/merge.
Do not create an SSR Worker or install `@astrojs/cloudflare` for this static site.

| Setting | Value |
| --- | --- |
| Project name | `lumi-agents-web` |
| Production branch | `main` |
| Root directory | `web` |
| Framework preset | Astro |
| Build command | `npm run build` |
| Build output directory | `dist` |
| Node version | `24.13.0` (`NODE_VERSION` if needed) |

For previewing this change, use branch `feat/astro-landing-page`.
`CF_PAGES_BRANCH` other than `main` produces noindex robots/meta/headers.
Production is indexed with the canonical domain `agents.runlumi.app`.

In Pages → Custom domains, add `agents.runlumi.app` and follow the DNS validation
shown for the existing `runlumi.app` zone. Do not overwrite unrelated DNS records.
Only connect DNS after the Pages preview passes its checks. Creating this code or
merging its PR does **not** create the Pages project, attach the domain, or deploy it.

`wrangler.jsonc` declares Pages output for an optional CLI deployment from `web/`:

```sh
# Use an explicitly reviewed/pinned Wrangler installation and your own credentials.
wrangler pages deploy dist --project-name=lumi-agents-web --branch=main
```

No Cloudflare token belongs in this repository or in browser code. Git integration
is the default route; no deploy token is needed by the test workflow.

References: [Cloudflare Astro / Pages guide](https://developers.cloudflare.com/pages/framework-guides/deploy-an-astro-site/)
and [Astro deployment documentation](https://docs.astro.build/en/guides/deploy/cloudflare/).

## Content, identity and trust

- `src/data/site.mjs`: canonical URLs, contact link, three synthetic scenarios and FAQ.
- `src/pages/index.astro`: editorial narrative and section structure.
- `src/components/WorkDemo.astro`: progressively enhanced product illustration.
- `src/styles/global.css`: light-only brand tokens and responsive composition.
- `src/scripts/landing.js`: small, local-only interaction layer; no React hydration.
- `public/samples/`: downloadable synthetic examples, not real execution evidence.
- `scripts/postbuild.mjs`: CSP hashes and Cloudflare security/cache headers.

Follow [DESIGN.md](../DESIGN.md), [ICON.md](../ICON.md), and
[readiness](../docs/v1-readiness.md). Never publish invented release availability,
customer logos, ratings, ROI, adoption counts, provider certifications or security
claims. A source change in readiness is the prerequisite to changing release copy.
The three demo scenarios are visibly labelled; buttons never execute live actions.

The CTA opens the visitor's email app to `hello@runlumi.app`; it does not register
an account or claim a message was sent. Verify ownership/monitoring of that mailbox
before enabling a campaign. No personal data is submitted by this static page.

### Assets

The folded-L SVG is copied byte-for-byte from
`apps/desktop/src/assets/brand/lumi-logo.svg`. Its supplied artwork uses
`#019EDD` / `#0162A4`, whereas DESIGN.md specifies `#006093` for UI authority.
This existing asset/token drift is preserved, not silently rebranded. All authored
UI tokens retain the DESIGN.md anchors. The initial Open Graph PNG reuses the
existing brand artwork, not a product screenshot. Fonts are self-hosted at build
time from the declared Fontsource packages; see THIRD_PARTY_NOTICES.md.

### Changes and deployment checks

Keep this package independent of the desktop runtime. No framework, database,
API endpoint, consent banner or analytics script is needed for this page. Any new
remote resource requires an explicit privacy decision and a reviewed CSP change.
Check real deployed response headers, sample downloads, font loading, mobile menu,
404 status, robots, canonical URL and domain TLS after configuring Pages. Test a
preview branch and confirm it is noindex before sharing it publicly.

Rollback: revert the website change or use Pages deployment rollback. Neither
requires changing the desktop app or Rust crates. Website quality does not certify
agent runtime reliability.
