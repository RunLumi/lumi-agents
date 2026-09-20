import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile, stat } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import { site, faqs, previewBuild } from '../src/data/site.mjs';
import { publisher, productFacts, renderLlmsText } from '../src/data/discovery.mjs';

const html = await readFile('dist/index.html', 'utf8');
const headers = await readFile('dist/_headers', 'utf8');
function schemas(text) {
  return [...text.matchAll(/<script\b[^>]*type="application\/ld\+json"[^>]*>([\s\S]*?)<\/script>/g)].map(([, value]) => JSON.parse(value));
}
const graph = schemas(html).flatMap(schema => schema['@graph'] || [schema]);
const node = type => graph.find(item => item['@type'] === type);

test('built HTML exposes the entity graph, author and consistent product metadata', () => {
  assert.equal(schemas(html).length, 1);
  for (const type of ['Organization', 'WebSite', 'WebPage', 'FAQPage']) assert.ok(node(type), type);
  assert.equal(node('WebSite').name, site.name);
  assert.ok(html.includes(`property="og:site_name" content="${site.name}"`));
  assert.ok(html.includes(`<title>${site.title}</title>`));
  assert.equal(node('Organization').name, publisher.name);
  assert.ok(html.includes(`name="author" content="${publisher.name}"`));
  assert.ok(html.includes('rel="author"'));
  assert.deepEqual(node('FAQPage').mainEntity.map(item => [item.name, item.acceptedAnswer.text]), faqs);
});

test('semantic product table, attribution and every fact are server-rendered', () => {
  assert.match(html, /<table\b/);
  assert.match(html, /<caption\b[^>]*>Thông tin sản phẩm và giới hạn hiện tại<\/caption>/);
  assert.equal((html.match(/scope="row"/g) || []).length, productFacts.length);
  assert.match(html, /id="thong-tin"/);
  assert.match(html, /href="\/llms\.txt"/);
  assert.match(html, /Biên soạn và duy trì bởi/);
  for (const fact of productFacts) {
    assert.ok(html.includes(fact.value), fact.label);
    assert.ok(html.includes(`href="${fact.source}"`), fact.source);
  }
});

test('generated llms.txt is plain text and stays synchronized with the product facts', async () => {
  const text = await readFile('dist/llms.txt', 'utf8');
  assert.equal(text, renderLlmsText({ preview: previewBuild }));
  assert.doesNotMatch(text, /<!doctype|<html|<script/i);
  assert.match(headers, /\/llms\.txt\n  Content-Type: text\/plain; charset=utf-8\n  Cache-Control: public, max-age=0, must-revalidate/);
  for (const [, link] of text.matchAll(/\[[^\]]+\]\((https:\/\/[^)]+)\)/g)) {
    const url = new URL(link);
    if (url.origin === site.origin && url.hash) assert.ok(html.includes(`id="${url.hash.slice(1)}"`), link);
  }
});

test('robots, sitemap, canonical and noindex respect the build environment', async () => {
  const robots = await readFile('dist/robots.txt', 'utf8');
  assert.ok(html.includes(`rel="canonical" href="${site.origin}/"`));
  if (previewBuild) {
    assert.match(robots, /Disallow: \//);
    assert.match(html, /content="noindex, nofollow"/);
    assert.match(headers, /X-Robots-Tag: noindex, nofollow/);
  } else {
    assert.match(robots, /Allow: \//);
    assert.ok(robots.includes(`Sitemap: ${site.origin}/sitemap.xml`));
    assert.match(html, /content="index, follow"/);
  }
  const sitemap = await readFile('dist/sitemap.xml', 'utf8');
  assert.ok(sitemap.includes(`<loc>${site.origin}/</loc>`));
  assert.doesNotMatch(sitemap, /404|llms\.txt|pages\.dev/);
});

test('404 remains noindex and never advertises homepage FAQs', async () => {
  const notFound = await readFile('dist/404.html', 'utf8');
  assert.match(notFound, /content="noindex, nofollow"/);
  assert.equal(schemas(notFound).flatMap(item => item['@graph'] || [item]).some(item => item['@type'] === 'FAQPage'), false);
});

test('entity logo exists and every inline script still matches the strict CSP', async () => {
  const logo = new URL(node('Organization').logo);
  assert.equal(logo.origin, site.origin);
  assert.ok((await stat(`dist${logo.pathname}`)).isFile());
  for (const [, attributes, content] of html.matchAll(/<script\b([^>]*)>([\s\S]*?)<\/script>/gi)) {
    if (/\bsrc\s*=/.test(attributes) || !content.trim()) continue;
    const hash = createHash('sha256').update(content).digest('base64');
    assert.ok(headers.includes(`'sha256-${hash}'`), 'Inline script is absent from CSP');
  }
  assert.doesNotMatch(headers, /unsafe-inline|unsafe-eval/);
  assert.ok(headers.split('\n').every(line => line.length <= 2000));
});
