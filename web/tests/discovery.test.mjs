import test from 'node:test';
import assert from 'node:assert/strict';
import { site, faqs } from '../src/data/site.mjs';
import { publisher, productFacts, createStructuredData, serializeStructuredData, renderLlmsText } from '../src/data/discovery.mjs';

const homepage = createStructuredData({ includeFaq: true });
const node = type => homepage['@graph'].find(item => item['@type'] === type);

test('publisher and product identity stay distinct, explicit and linked', () => {
  const organization = node('Organization');
  const website = node('WebSite');
  const webpage = node('WebPage');
  assert.equal(organization.name, 'RunLumi');
  assert.equal(organization.url, publisher.url);
  assert.equal(organization.logo, `${site.origin}/brand/lumi-logo.svg`);
  assert.deepEqual(organization.sameAs, ['https://github.com/RunLumi']);
  assert.equal(website.name, site.name);
  assert.equal(webpage.name, site.title);
  assert.equal(webpage.url, `${site.origin}/`);
  assert.equal(website.publisher['@id'], organization['@id']);
  assert.equal(webpage.author['@id'], organization['@id']);
  assert.equal(webpage.isPartOf['@id'], website['@id']);
});

test('all structured-data references resolve to unique, stable graph nodes', () => {
  const ids = homepage['@graph'].map(item => item['@id']);
  assert.equal(new Set(ids).size, ids.length);
  for (const id of ids) assert.equal(new URL(id).protocol, 'https:');
  function check(value) {
    if (!value || typeof value !== 'object') return;
    if (value['@id']) assert.ok(ids.includes(value['@id']), value['@id']);
    for (const child of Object.values(value)) check(child);
  }
  check(homepage);
});

test('FAQ schema has exact question/answer parity with the visible content source', () => {
  const faq = node('FAQPage');
  assert.equal(faq.url, `${site.origin}/#cau-hoi`);
  assert.equal(faq.mainEntity.length, faqs.length);
  assert.deepEqual(faq.mainEntity.map(item => [item.name, item.acceptedAnswer.text]), faqs);
  assert.equal(node('WebPage').hasPart['@id'], faq['@id']);
  for (const item of faq.mainEntity) {
    assert.equal(item['@type'], 'Question');
    assert.equal(item.acceptedAnswer['@type'], 'Answer');
  }
});

test('FAQ schema requires explicit opt-in and never leaks onto a non-homepage', () => {
  for (const options of [{}, { pathname: '/404/', includeFaq: true }, { pathname: '/other/', includeFaq: true }]) {
    const graph = createStructuredData(options)['@graph'];
    assert.equal(graph.some(item => item['@type'] === 'FAQPage'), false);
    assert.equal(graph.find(item => item['@type'] === 'WebPage').hasPart, undefined);
  }
  assert.equal(createStructuredData({ pathname: '/other/', description: 'Other page' })['@graph'].find(item => item['@type'] === 'WebSite').description, site.description);
});

test('JSON-LD serialization cannot terminate its script element and round-trips', () => {
  const value = { text: '</script><script>alert("x")</script>\u2028\u2029', vi: 'Giữ quyền làm chủ.' };
  const serialized = serializeStructuredData(value);
  assert.doesNotMatch(serialized, /<|\u2028|\u2029/);
  assert.deepEqual(JSON.parse(serialized), value);
});

test('llms.txt shares every visible fact and uses real first-party sources', () => {
  const text = renderLlmsText();
  assert.ok(text.startsWith(`# ${site.name}\n\n> ${site.description}\n`));
  assert.ok(text.endsWith('\n'));
  for (const fact of productFacts) {
    assert.ok(fact.value.length > 40);
    assert.ok(text.includes(fact.value));
    assert.ok(text.includes(fact.source));
    assert.ok(fact.source.startsWith(`${site.repository}/blob/main/`));
  }
  assert.ok(text.includes('chưa phát hành v1'));
  assert.ok(text.includes('không phải bằng chứng'));
  assert.doesNotMatch(text, /localhost|pages\.dev|example\.com|guaranteed/i);
});

test('preview llms.txt points to production without publishing a second fact sheet', () => {
  const text = renderLlmsText({ preview: true });
  assert.ok(text.includes(`${site.origin}/`));
  assert.ok(text.includes('không phải website chính thức'));
  for (const fact of productFacts) assert.equal(text.includes(fact.value), false);
});

test('metadata adds no invented author, commercial offer, review or release claim', () => {
  const serialized = JSON.stringify(homepage);
  assert.doesNotMatch(serialized, /"Person"|aggregateRating|reviewCount|"offers"|downloadUrl|datePublished|dateModified|"HowTo"|"Article"/);
  assert.equal(node('WebSite').description, site.description);
});
