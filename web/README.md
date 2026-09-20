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
licenses, sample artifacts, robots.txt, sitemap.xml, llms.txt, a 404 page, and Pages security
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

Use the current PR branch for preview builds. `CF_PAGES_BRANCH` other than `main`
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
- `src/data/discovery.mjs`: publisher identity, product facts, JSON-LD and llms.txt.
- `src/components/ProductFacts.astro`: visible, source-linked product fact table.
- `src/pages/llms.txt.ts`: prerendered, plain-text discovery endpoint.
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

## GEO / AI discovery

The supplied NsLookup snapshot (20 September 2026) reported 60/100, with missing
llms.txt, explicit Organization schema, author signals, rich schema and tables.
This change addresses those observable gaps; it does not assert a new external
score or guarantee indexing, rankings, rich results or citations by any AI system.

- **Identity:** an explicit Organization node describes **RunLumi**, the publisher;
  WebSite and WebPage describe **Lumi Agents**. Stable IDs link publisher, author,
  site, page and FAQ. Keep the product name consistent in title/OG/site metadata,
  but do not rename the organization to game a brand-consistency heuristic.
- **FAQ:** the visible native details and FAQPage JSON-LD use the same `faqs` array.
  Only the homepage opts in; a 404 does not acquire homepage FAQ data. No fabricated
  Person, Article, HowTo, ratings, offers, release dates or security guarantees.
- **Extractability:** the visible product definition and semantic, source-linked
  table distinguish present limits from platform ambitions. They work without JS.
  RunLumi is visibly credited as the content author, not an invented expert.
- **llms.txt:** a small, optional machine-readable summary generated from the same
  facts, with canonical first-party links and explicit preview/demo limitations.
  It is not an access-control mechanism or a substitute for robots.txt. Preview
  builds publish only a pointer to production and retain existing noindex controls.
  The production robots policy is unchanged; crawler training and search access
  are not silently reconfigured. Cloudflare WAF/bot settings are a separate layer.
- **Security:** postbuild hashes the actual JSON-LD bytes into CSP; no unsafe-inline,
  external script, dependency or hydration is added. `/llms.txt` gets a plain-text
  MIME type and revalidation cache policy, not immutable caching.

`npm test` covers source invariants and compiled output, including graph links,
exact FAQ parity, script-safe serialization, static facts, crawl files, 404 exclusion
and CSP hashes. Browser tests additionally compare JSON-LD against actual FAQ DOM,
check the text endpoint, cover no-JavaScript content, run axe at four widths, and
capture `facts-*.png` alongside the existing full-page evidence. Tests can also run
against a preview build with the same `CF_PAGES_BRANCH` value for build and test.

After the production deployment is confirmed, fetch `/`, `/llms.txt`, `/robots.txt`
and `/sitemap.xml` from the custom domain; check their HTTP status, MIME and robots
headers. Check Cloudflare bot/WAF behavior separately. Re-run the external GEO
checker on the deployed site and validate structured data. Record observed results;
never infer a new score from a merged PR. Measure qualified discovery/referrals and
useful trial conversations, not a checker score alone.

Primary references:
- [Google: AI features and your website](https://developers.google.com/search/docs/appearance/ai-features)
- [Google: Organization structured data](https://developers.google.com/search/docs/appearance/structured-data/organization)
- [llms.txt proposal](https://llmstxt.org/)

Google's guidance says AI Search features require no special AI text file or schema;
structured data should match visible content. llms.txt is supplementary, not a
required Google ranking signal. This implementation favors accurate discovery over
unsupported claims made by a third-party scoring explanation.
