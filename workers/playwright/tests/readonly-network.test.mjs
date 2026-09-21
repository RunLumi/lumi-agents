import assert from 'node:assert/strict';
import test from 'node:test';
import http from 'node:http';
import { allowedUrl, pinnedRead, publicIPv4 } from '../readonly-network.js';

test('rejects private, metadata, mapped, reserved and multicast addresses', () => {
  for (const ip of ['127.0.0.1', '10.0.0.1', '169.254.169.254', '172.16.0.1', '192.168.1.1',
    '100.64.0.1', '192.0.0.1', '198.18.0.1', '224.0.0.1', '::ffff:127.0.0.1', '0.0.0.0']) assert.equal(publicIPv4(ip), false, ip);
  assert.equal(publicIPv4('93.184.215.14'), true);
});
test('exact origin boundaries; no credentials, local files, IP literals or scheme tricks', () => {
  const origins = ['https://example.com'];
  assert.equal(allowedUrl('https://example.com/docs', origins).origin, origins[0]);
  for (const url of ['https://example.com.evil.test/', 'file:///etc/passwd', 'http://example.com/',
    'https://user:secret@example.com/', 'https://127.0.0.1/', 'javascript:alert(1)', 'https://example.com:444/']) {
    assert.throws(() => allowedUrl(url, origins), undefined, url);
  }
});
test('hermetic GET uses a real socket but cannot expand its fixture origin', async () => {
  const server = http.createServer((_req, res) => res.end('observed fixture body'));
  await new Promise((resolve) => server.listen(0, '127.0.0.1', resolve));
  const origin = `http://127.0.0.1:${server.address().port}`;
  try {
    const result = await pinnedRead(`${origin}/`, [origin], { requests: 0, bytes: 0 }, origin);
    assert.equal(result.status, 200);
    assert.equal(result.body.toString(), 'observed fixture body');
    await assert.rejects(pinnedRead('http://127.0.0.1:1/', [origin], { requests: 0, bytes: 0 }, origin));
    await assert.rejects(pinnedRead(`${origin}/`, [origin], { requests: 64, bytes: 0 }, origin));
  } finally { await new Promise((resolve) => server.close(resolve)); }
});
