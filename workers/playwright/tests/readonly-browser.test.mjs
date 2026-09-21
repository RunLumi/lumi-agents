import test from 'node:test';
import assert from 'node:assert/strict';
import http from 'node:http';
import { spawn } from 'node:child_process';
import { createInterface } from 'node:readline';
import { fileURLToPath } from 'node:url';

test('real Chromium returns page observations and refuses interactive operations', { timeout: 60000 }, async () => {
  const server = http.createServer((request, response) => {
    response.setHeader('content-type', 'text/html');
    response.end('<title>Orders fixture</title><h1>Orders: 3</h1><a href="/report">Report</a><script>document.body.innerHTML="UNSAFE SCRIPT RAN"</script>');
  });
  await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
  const origin = `http://127.0.0.1:${server.address().port}`;
  const worker = spawn(process.execPath, [fileURLToPath(new URL('../readonly-worker.js', import.meta.url)), JSON.stringify([origin]), `--test-origin=${origin}`], {
    env: { ...process.env, LUMI_BROWSER_HERMETIC_TEST: '1' }, stdio: ['pipe', 'pipe', 'pipe'],
  });
  const pending = new Map(); let sequence = 0; let stderr = '';
  worker.stderr.on('data', data => { stderr += data.toString(); });
  const lines = createInterface({ input: worker.stdout });
  lines.on('line', line => { const value = JSON.parse(line); pending.get(value.id)?.(value); pending.delete(value.id); });
  const request = op => new Promise((resolve, reject) => {
    const id = `test-${++sequence}`;
    const timeout = setTimeout(() => { pending.delete(id); reject(new Error(`worker timed out: ${stderr}`)); }, 25000);
    pending.set(id, value => { clearTimeout(timeout); resolve(value); });
    worker.stdin.write(`${JSON.stringify({ id, ...op })}\n`);
  });
  try {
    assert.equal((await request({ op: 'navigate', url: origin })).ok, true);
    const read = await request({ op: 'read' });
    assert.equal(read.ok, true);
    assert.match(read.result.observation.text, /Orders: 3/);
    assert.doesNotMatch(read.result.observation.text, /UNSAFE SCRIPT RAN/);
    assert.ok(read.result.observation.links.includes(`${origin}/report`));
    assert.equal((await request({ op: 'click', target: { strategy: 'text', text: 'Report', exact: true } })).ok, false);
    assert.equal((await request({ op: 'navigate', url: 'https://not-approved.example/' })).ok, false);
    assert.equal((await request({ op: 'navigate', url: 'file:///etc/passwd' })).ok, false);
    assert.equal((await request({ op: 'close' })).ok, true);
  } finally {
    lines.close(); worker.kill(); server.closeAllConnections(); await new Promise(resolve => server.close(resolve));
  }
});
