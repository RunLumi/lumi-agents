// UI-only contract test. The explicitly installed fixture transport is not
// execution evidence; the separate Chromium worker test covers real page I/O.
import test from 'node:test';
import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright';

const uiRoot = fileURLToPath(new URL('../../../apps/desktop/ui/', import.meta.url));
const vite = fileURLToPath(new URL('../../../apps/desktop/ui/node_modules/vite/bin/vite.js', import.meta.url));
const base = 'http://127.0.0.1:4197';

test('project composer and automation controls use the selected project and preserve multiline drafts', { timeout: 90000 }, async () => {
  const server = spawn(process.execPath, [vite, '--host', '127.0.0.1', '--port', '4197', '--strictPort'], {
    cwd: uiRoot, stdio: ['ignore', 'pipe', 'pipe'],
  });
  let output = '';
  server.stdout.on('data', data => { output += data; });
  server.stderr.on('data', data => { output += data; });
  let browser;
  try {
    let ready = false;
    for (let attempt = 0; attempt < 120; attempt++) {
      try { if ((await fetch(base)).ok) { ready = true; break; } } catch { /* server starting */ }
      await new Promise(resolve => setTimeout(resolve, 150));
    }
    assert.ok(ready, `Vite did not start: ${output}`);
    browser = await chromium.launch({ headless: true });
    const page = await browser.newPage({ viewport: { width: 1440, height: 1000 }, locale: 'en-US' });
    const failures = [];
    page.on('pageerror', error => failures.push(error.message));
    await page.goto(`${base}/?devmock`);
    await page.getByText('PrintUp', { exact: true }).first().waitFor();
    await page.evaluate(async () => {
      const { getTransport, setMockTransport } = await import('/src/ipc/transport.ts');
      const original = getTransport();
      const fixture = { requests: [], starts: 0, automations: [] };
      window.__engagementFixture = fixture;
      setMockTransport({
        async invoke(command, args) {
          switch (command) {
            case 'engagement_snapshot': return {
              capabilities: [
                { id: 'files', state: 'ready', reason: 'files_ready' },
                { id: 'browser', state: 'setup_required', reason: 'browser_setup' },
                { id: 'chrome', state: 'unavailable', reason: 'chrome_unavailable' },
                { id: 'computer', state: 'unavailable', reason: 'computer_unavailable' },
                { id: 'shell', state: 'ready', reason: 'shell_manual' },
              ], browser_origins: [], provider_configured: true, stopped: false,
            };
            case 'provider_get_config': return { configured: true };
            case 'engagement_create_task': {
              fixture.requests.push(args.request);
              return { task_id: 'fixture-created-task', goal: args.request.goal, status: 'CREATED', created_at: new Date().toISOString() };
            }
            case 'task_run': fixture.starts++; return { started: true, task_id: args.taskId };
            case 'automation_list': return { items: structuredClone(fixture.automations), provider_configured: true, stopped: false, scheduler_available: true };
            case 'automation_preview': return [Math.floor(Date.now() / 1000) + 86400];
            case 'automation_save': {
              const saved = { ...args.input, automation_id: 'fixture-automation', project_id: args.projectId, revision: 1,
                next_run_at: Math.floor(Date.now() / 1000) + 86400, runs: [] };
              fixture.automations = [saved];
              return structuredClone(saved);
            }
            case 'automation_toggle': fixture.automations[0].enabled = args.enabled; fixture.automations[0].revision++; return null;
            default: return original.invoke(command, args);
          }
        },
      });
    });
    await page.getByText('PrintUp', { exact: true }).first().click();
    await page.getByRole('tab', { name: 'Tasks', exact: true }).click();
    const composer = page.getByRole('combobox', { name: 'What would you like Lumi to complete?', exact: true });
    await composer.fill('First paragraph.');
    await composer.press('Enter');
    await composer.pressSequentially('Second paragraph.');
    assert.equal(await composer.inputValue(), 'First paragraph.\nSecond paragraph.');
    assert.equal(await page.evaluate(() => window.__engagementFixture.requests.length), 0);
    await page.getByRole('tab', { name: 'Automations', exact: true }).click();
    await page.getByRole('heading', { name: 'Repeat useful work' }).waitFor();
    await page.getByRole('tab', { name: 'Tasks', exact: true }).click();
    assert.equal(await composer.inputValue(), 'First paragraph.\nSecond paragraph.');
    await composer.press('Control+Enter');
    await page.waitForFunction(() => window.__engagementFixture.starts === 1);
    await page.waitForFunction(() => document.querySelector('textarea[role="combobox"]')?.value === '');
    const request = await page.evaluate(() => window.__engagementFixture.requests[0]);
    assert.equal(request.project_id, 'devmock-printup');
    assert.equal(request.goal, 'First paragraph.\nSecond paragraph.');
    assert.deepEqual(request.tools, ['files']);
    await composer.fill('@ch');
    const chrome = page.getByRole('option', { name: /@chrome/ });
    await chrome.waitFor();
    assert.equal(await chrome.getAttribute('aria-disabled'), 'true');
    await composer.press('Escape');
    await composer.fill('Read the project files.\nCreate a new summary.');
    await page.getByRole('button', { name: 'Schedule…', exact: true }).click();
    await page.getByLabel('Name', { exact: true }).fill('Fixture weekly summary');
    await page.getByLabel('Timezone (IANA)', { exact: true }).fill('Asia/Ho_Chi_Minh');
    await page.getByLabel('Allow unattended runs with these tools until the date below', { exact: true }).check();
    await page.getByRole('button', { name: 'Save automation', exact: true }).click();
    await page.getByRole('heading', { name: 'Fixture weekly summary', exact: true }).waitFor();
    const automation = await page.evaluate(() => window.__engagementFixture.automations[0]);
    assert.equal(automation.project_id, 'devmock-printup');
    assert.equal(automation.timezone, 'Asia/Ho_Chi_Minh');
    assert.equal(automation.enabled, true);
    assert.equal(automation.goal, 'Read the project files.\nCreate a new summary.');
    await page.getByRole('button', { name: 'Pause', exact: true }).click();
    await page.getByRole('button', { name: 'Enable', exact: true }).waitFor();
    assert.equal(await page.evaluate(() => window.__engagementFixture.automations[0].enabled), false);
    assert.deepEqual(failures, []);
  } finally {
    if (browser) await browser.close();
    server.kill();
  }
});
