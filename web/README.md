# Lumi Agents — public website

Vietnamese, static Astro landing page for **https://agents.runlumi.app**.
This is a separate build from the Rust runtime and desktop app. It does not run
agents, connect user accounts, collect form submissions, or imply a released v1.

## Develop and verify

Use Node `24.13.0` from `.node-version` and the committed npm lockfile.

```sh
cd web
npm ci --ignore-scripts
npm run dev
npm run build
npm test
npx --no-install playwright install chromium
npm run test:e2e
npm audit --audit-level=high
```

`build` generates static HTML/CSS/JS, bundled self-hosted fonts and their upstream
licenses, sample artifacts, robots.txt, sitemap.xml, a 404 page, and Pages security
headers. Browser tests apply the generated CSP to documents because `astro preview`
does not implement `_headers`. They cover 320 / 375 / 768 / 1440 px, keyboard tabs,
downloads, native FAQ/menu, no-JavaScript content, reduced motion, overflow and axe
accessibility checks. Inspect CI results; test source alone is not a passing result.

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

Preview this PR on `feat/astro-landing-page`. `CF_PAGES_BRANCH` other than `main`
produces noindex robots/meta/headers. Pages-owned hostnames also receive noindex
headers. Production uses the canonical domain `agents.runlumi.app`.

In Pages → Custom domains, add `agents.runlumi.app` and follow DNS validation for
the existing `runlumi.app` zone. Do not overwrite unrelated DNS records. Connect
DNS only after the Pages preview passes its checks. Creating this code or merging
its PR does **not** create the Pages project, attach the domain, or deploy it.

`wrangler.jsonc` declares Pages output for optional CLI deployment from `web/`:

```sh
# Use an explicitly reviewed/pinned Wrangler installation and your credentials.
wrangler pages deploy dist --project-name=lumi-agents-web --branch=main
```

No Cloudflare token belongs in the repository or browser code. Git integration is
the default route. The retained test workflow has read-only repository permission,
does not persist checkout credentials and does not push commits or deploy anything.

References: [Cloudflare Astro / Pages guide](https://developers.cloudflare.com/pages/framework-guides/deploy-an-astro-site/),
[Astro deployment documentation](https://docs.astro.build/en/guides/deploy/cloudflare/),
and [Pages header matching](https://developers.cloudflare.com/pages/configuration/headers/).

## Content, identity and trust

- `src/data/site.mjs`: canonical URLs, contact link, synthetic scenarios and FAQ.
- `src/pages/index.astro`: editorial narrative and section structure.
- `src/components/WorkDemo.astro`: progressively enhanced product illustration.
- `src/styles/global.css`: light-only brand tokens and responsive composition.
- `src/scripts/landing.js`: small, local-only interactions; no React hydration.
- `public/samples/`: downloadable synthetic examples, not execution evidence.
- `scripts/postbuild.mjs`: CSP hashes and Cloudflare security/cache headers.

Follow [DESIGN.md](../DESIGN.md), [ICON.md](../ICON.md), and
[readiness](../docs/v1-readiness.md). Never publish invented release availability,
customer logos, ratings, ROI, adoption counts, provider certifications or security
claims. Readiness evidence is the prerequisite to changing release copy. Demo
scenarios are visibly labelled; website buttons never execute live agent actions.

The CTA opens the visitor's email app to `hello@runlumi.app`; it does not register
an account or claim a message was sent. Verify monitoring of that mailbox before
a campaign. No personal data is submitted by this static page.

### Assets

The folded-L SVG is copied byte-for-byte from
`apps/desktop/src/assets/brand/lumi-logo.svg`. Its supplied artwork uses
`#019EDD` / `#0162A4`, whereas DESIGN.md specifies `#006093` for UI authority.
This existing asset/token drift is preserved, not silently rebranded. All authored
UI tokens retain the DESIGN.md anchors. The initial Open Graph PNG reuses the
existing brand artwork, not a product screenshot. Fonts are self-hosted at build
time from Fontsource; see THIRD_PARTY_NOTICES.md and generated `dist/licenses/`.

### Changes and deployment checks

Keep this package independent of the desktop runtime. No database, API endpoint,
consent banner or analytics script is needed. Any new remote resource requires an
explicit privacy decision and a reviewed CSP change. Check actual deployed headers,
font loading, sample downloads, mobile menu, 404 status, robots, canonical and TLS.
Verify a preview branch is noindex before sharing it publicly.

Rollback: revert the website change or use Pages deployment rollback. Neither
requires changing desktop code or Rust crates. Website quality does not certify
agent runtime reliability.
