import { readdir, readFile, writeFile } from 'node:fs/promises';
import { join } from 'node:path';
import { createHash } from 'node:crypto';
import { previewBuild } from '../src/data/site.mjs';

async function htmlFiles(directory) {
  const entries = await readdir(directory, { withFileTypes: true });
  const groups = await Promise.all(entries.map(entry => entry.isDirectory() ? htmlFiles(join(directory, entry.name)) : entry.name.endsWith('.html') ? [join(directory, entry.name)] : []));
  return groups.flat();
}
const scriptHashes = new Set();
const styleHashes = new Set();
for (const file of await htmlFiles('dist')) {
  const html = await readFile(file, 'utf8');
  for (const [tag, hashes] of [['script', scriptHashes], ['style', styleHashes]]) {
    const pattern = new RegExp(`<${tag}\\b([^>]*)>([\\s\\S]*?)<\\/${tag}>`, 'gi');
    for (const [, attributes, content] of html.matchAll(pattern)) {
      if (tag === 'script' && /\bsrc\s*=/.test(attributes)) continue;
      if (content.trim()) hashes.add(`'sha256-${createHash('sha256').update(content).digest('base64')}'`);
    }
  }
}
const csp = [
  "default-src 'none'", `script-src 'self' ${[...scriptHashes].join(' ')}`.trim(),
  `style-src 'self' ${[...styleHashes].join(' ')}`.trim(), "img-src 'self'", "font-src 'self'",
  "connect-src 'none'", "object-src 'none'", "base-uri 'none'", "form-action 'none'", "frame-ancestors 'none'",
].join('; ');
await writeFile('dist/_headers', `/*\n  Content-Security-Policy: ${csp}\n  X-Content-Type-Options: nosniff\n  Referrer-Policy: strict-origin-when-cross-origin\n  Permissions-Policy: camera=(), microphone=(), geolocation=(), payment=(), usb=()\n  X-Frame-Options: DENY\n  Cache-Control: public, max-age=0, must-revalidate\n${previewBuild ? '  X-Robots-Tag: noindex, nofollow\n' : ''}\n/_astro/*\n  Cache-Control: public, max-age=31536000, immutable\n\n/samples/*\n  X-Robots-Tag: noindex\n`);
console.log(`Pages headers generated; ${scriptHashes.size} inline script hashes, ${styleHashes.size} inline style hashes. Preview noindex: ${previewBuild}.`);
