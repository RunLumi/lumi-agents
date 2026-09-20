import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile, readdir, stat } from 'node:fs/promises';
import { workflows, site } from '../src/data/site.mjs';
const html = await readFile('dist/index.html', 'utf8');

test('static page has one Vietnamese h1, canonical metadata and truthful status', () => {
  assert.equal((html.match(/<h1\b/g) || []).length, 1);
  assert.match(html, /lang="vi"/);
  assert.match(html, /https:\/\/agents\.runlumi\.app\//);
  assert.match(html, /Chưa phát hành v1/);
  assert.match(html, /dữ liệu giả lập/);
  assert.doesNotMatch(html, /aggregateRating|trusted by|10,000\+/i);
});
test('every in-page anchor has a real destination; IDs are unique', () => {
  const ids = [...html.matchAll(/\bid="([^"]+)"/g)].map(match => match[1]);
  assert.equal(new Set(ids).size, ids.length);
  for (const [, id] of html.matchAll(/href="#([^"]+)"/g)) assert.ok(ids.includes(id), `Missing #${id}`);
});
test('every scenario has a real sample artifact and a labelled panel', async () => {
  for (const flow of workflows) {
    assert.match(html, new RegExp(`id="panel-${flow.id}"`));
    const content = await readFile(`dist/samples/${flow.artifact}`, 'utf8');
    assert.ok(content.length > 100);
  }
  const csv = await readFile('dist/samples/doi-soat-mau.csv', 'utf8');
  const records = csv.trim().split('\n').slice(1).map(row => row.split(','));
  assert.equal(records.length, 14);
  assert.equal(records.filter(row => row[3] === 'matched').length, 12);
  assert.equal(records.filter(row => row[3] !== 'matched').length, 2);
});
test('security and crawler output is present, without unsafe-inline or external scripts', async () => {
  const headers = await readFile('dist/_headers', 'utf8');
  assert.match(headers, /Content-Security-Policy:/);
  assert.match(headers, /frame-ancestors 'none'/);
  assert.doesNotMatch(headers, /unsafe-inline|unsafe-eval/);
  assert.doesNotMatch(html, /<script[^>]+src="https?:/i);
  assert.match(await readFile('dist/sitemap.xml', 'utf8'), new RegExp(site.origin.replaceAll('.', '\\.')));
  assert.ok((await readFile('dist/robots.txt', 'utf8')).includes('User-agent: *'));
  assert.ok((await readFile('dist/404.html', 'utf8')).includes('noindex'));
});
test('brand tokens and original folded-L geometry are retained', async () => {
  const css = await readFile('src/styles/global.css', 'utf8');
  for (const color of ['#006093', '#102A43', '#F4F0E8', '#FFFFFF', '#E7EAF0', '#5E6677', '#F4A62A', '#C2410C', '#1F7A4D']) assert.ok(css.includes(color), color);
  assert.match(css, /color-scheme: light/);
  assert.match(css, /prefers-reduced-motion/);
  const logo = await readFile('public/brand/lumi-logo.svg', 'utf8');
  assert.match(logo, /M99\.4558/);
});
test('the browser receives no framework runtime and stays within a small JS budget', async () => {
  const files = await readdir('dist/_astro');
  const scripts = files.filter(file => file.endsWith('.js'));
  let total = 0;
  for (const file of scripts) total += (await stat(`dist/_astro/${file}`)).size;
  assert.ok(total < 16000, `JavaScript budget exceeded: ${total} bytes`);
  assert.doesNotMatch(html, /astro-island|react-dom/);
});
