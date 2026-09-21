// The managed reading adapter permits approved HTTPS origins, not arbitrary
// sockets. Resolve once, reject non-public IPv4, and pin that address to TLS.
// This avoids a DNS-check/Chromium-resolution race. TLS still verifies the
// original hostname. No inherited proxy, credentials, or persistent cookies.
import https from 'node:https';
import http from 'node:http';
import { lookup } from 'node:dns/promises';
import { isIP } from 'node:net';

export function publicIPv4(address) {
  if (isIP(address) !== 4) return false;
  const [a, b, c] = address.split('.').map(Number);
  return !(a === 0 || a === 10 || a === 127 || a >= 224
    || (a === 100 && b >= 64 && b <= 127)
    || (a === 169 && b === 254) || (a === 172 && b >= 16 && b <= 31)
    || (a === 192 && (b === 168 || b === 0 || (b === 88 && c === 99)))
    || (a === 198 && (b === 18 || b === 19 || (b === 51 && c === 100)))
    || (a === 203 && b === 0 && c === 113));
}
export function allowedUrl(raw, origins, testOrigin = null) {
  const url = new URL(raw);
  if (url.username || url.password) throw new Error('credentials in browser URLs are refused');
  const fixture = testOrigin && url.origin === testOrigin && url.hostname === '127.0.0.1' && url.protocol === 'http:';
  if (!fixture && (url.protocol !== 'https:' || isIP(url.hostname) || !url.hostname.includes('.'))) {
    throw new Error('only approved public HTTPS websites are supported');
  }
  if (!origins.includes(url.origin)) throw new Error('website is outside the approved origin scope');
  if (raw.length > 4096) throw new Error('URL exceeds the browser request limit');
  return url;
}
export async function pinnedRead(raw, origins, budget, testOrigin = null) {
  const url = allowedUrl(raw, origins, testOrigin);
  const fixture = testOrigin && url.origin === testOrigin;
  if (++budget.requests > 64) throw new Error('browser network request budget exhausted');
  let timer;
  let records;
  try {
    records = fixture ? [{ address: '127.0.0.1', family: 4 }] : await Promise.race([
      lookup(url.hostname, { family: 4, all: true }),
      new Promise((_, reject) => { timer = setTimeout(() => reject(new Error('DNS lookup timed out')), 5000); }),
    ]);
  } finally { clearTimeout(timer); }
  if (!records.length || (!fixture && records.some((r) => !publicIPv4(r.address)))) {
    throw new Error('browser DNS resolved to a non-public or unsupported address');
  }
  const address = records[0].address;
  return new Promise((resolve, reject) => {
    const transport = fixture ? http : https;
    const request = transport.request(url, {
      method: 'GET', agent: false, timeout: 10_000,
      headers: { accept: 'text/html,text/plain,text/css,image/*;q=0.5,*/*;q=0.1', 'accept-encoding': 'identity' },
      lookup: (_host, options, callback) => {
        if (options.all) callback(null, [{ address, family: 4 }]);
        else callback(null, address, 4);
      },
    }, (response) => {
      const encoding = response.headers['content-encoding'];
      if (encoding && encoding.toLowerCase() !== 'identity') {
        response.destroy();
        reject(new Error('compressed browser responses are refused to preserve byte limits'));
        return;
      }
      const chunks = [];
      let bytes = 0;
      response.on('data', (chunk) => {
        bytes += chunk.length; budget.bytes += chunk.length;
        if (bytes > 4 * 1024 * 1024 || budget.bytes > 16 * 1024 * 1024) {
          response.destroy(new Error('browser response budget exhausted')); return;
        }
        chunks.push(chunk);
      });
      response.on('error', reject);
      response.on('end', () => {
        const headers = {};
        for (const [name, value] of Object.entries(response.headers)) {
          if (value === undefined || ['set-cookie', 'connection', 'transfer-encoding', 'keep-alive'].includes(name)) continue;
          headers[name] = Array.isArray(value) ? value.join(', ') : value;
        }
        resolve({ status: response.statusCode ?? 502, headers, body: Buffer.concat(chunks) });
      });
    });
    request.on('timeout', () => request.destroy(new Error('browser request timed out')));
    request.on('error', reject);
    request.end();
  });
}
