// Scoped Work-mode adapter for public page reading. It shares Lumi's existing
// BrowserOp/WorkerResponse stdio protocol, but deliberately implements only
// navigate/read/close. Existing-profile attachment and mutating operations are
// refused. Network authority comes from trusted host argv, never model input.
import { chromium } from 'playwright';
import readline from 'node:readline';
import { allowedUrl, pinnedRead } from './readonly-network.js';

const origins = JSON.parse(process.argv[2] ?? '[]');
if (!Array.isArray(origins) || origins.length > 32 || origins.some((x) => typeof x !== 'string')) {
  throw new Error('invalid approved origin configuration');
}
// Hermetic fixture only. The desktop host never passes this flag and clears
// this environment variable when it launches workers.
const testOrigin = process.env.LUMI_BROWSER_HERMETIC_TEST === '1'
  && process.argv[3]?.startsWith('--test-origin=') ? process.argv[3].slice(14) : null;
let browser;
let context;
let page;
let sessionStarted = false;
const networkBudget = { requests: 0, bytes: 0 };

async function ensurePage() {
  if (page) return;
  browser = await chromium.launch({ headless: true });
  context = await browser.newContext({
    javaScriptEnabled: false, serviceWorkers: 'block', acceptDownloads: false,
    permissions: [],
  });
  await context.route('**/*', async (route) => {
    try {
      if (!['GET', 'HEAD'].includes(route.request().method())) throw new Error('mutating requests are refused');
      const response = await pinnedRead(route.request().url(), origins, networkBudget, testOrigin);
      await route.fulfill(response);
    } catch { await route.abort('blockedbyclient').catch(() => {}); }
  });
  if (context.routeWebSocket) await context.routeWebSocket('**/*', (socket) => socket.close());
  page = await context.newPage();
  page.setDefaultTimeout(15_000);
  page.on('dialog', (dialog) => dialog.dismiss().catch(() => {}));
  context.on('page', (other) => { if (other !== page) void other.close().catch(() => {}); });
}
async function dispatch(request) {
  if (!request || typeof request !== 'object' || Array.isArray(request)
    || typeof request.id !== 'string' || typeof request.op !== 'string') throw new Error('invalid worker request');
  if (request.session) {
    if (sessionStarted) throw new Error('session cannot be replaced');
    if (request.session.profile && request.session.profile.mode !== 'isolated') throw new Error('existing browser profiles are refused');
    if (request.session.trace && request.session.trace !== 'off') throw new Error('tracing is disabled for managed reads');
    if (request.session.headless === false) throw new Error('managed reads require an isolated headless session');
  }
  if (!['navigate', 'read', 'close'].includes(request.op)) throw new Error('operation unavailable in the read-only browser');
  if (request.op === 'close') { if (browser) await browser.close(); page = null; browser = null; context = null; return {}; }
  await ensurePage(); sessionStarted = true;
  if (request.op === 'navigate') {
    if (typeof request.url !== 'string') throw new Error('navigate requires a URL');
    if (request.url !== 'about:blank') allowedUrl(request.url, origins, testOrigin);
    const response = await page.goto(request.url, { waitUntil: 'domcontentloaded', timeout: 20_000 });
    if (page.url() !== 'about:blank') allowedUrl(page.url(), origins, testOrigin);
    if (response && response.status() >= 400) throw new Error(`website returned HTTP ${response.status()}`);
    return { url: page.url(), status: response?.status() ?? null };
  }
  const url = page.url();
  if (url !== 'about:blank') allowedUrl(url, origins, testOrigin);
  const text = (await page.locator('body').innerText()).slice(0, 16_000);
  const links = await page.locator('a[href]').evaluateAll((anchors) => anchors.slice(0, 24).map((a) => a.href).filter((href) => href.length <= 2048));
  // Wrapped JSON preserves link observations through WorkerResult's untagged
  // fallback, without changing the existing general worker's ReadResult wire.
  return { observation: { source: 'browser', trust: 'untrusted_page_content', url, title: await page.title(), text, links } };
}
const input = readline.createInterface({ input: process.stdin, crlfDelay: Infinity });
for await (const line of input) {
  if (!line.trim()) continue;
  let request;
  try {
    if (line.length > 65_536) throw new Error('worker request too large');
    request = JSON.parse(line);
    const result = await dispatch(request);
    process.stdout.write(`${JSON.stringify({ id: request.id, ok: true, result })}\n`);
  } catch (error) {
    process.stdout.write(`${JSON.stringify({ id: request?.id ?? 'invalid', ok: false,
      error: { category: 'BROWSER_STATE', message: String(error.message ?? error).slice(0, 1000), native_code: null } })}\n`);
  }
}
if (browser) await browser.close();
