// Lumi Playwright browser worker.
//
// Process boundary (spec 06 section 6.3): this worker runs OUTSIDE the
// privileged policy core. It receives already-authorized operations and
// MUST NOT own policy, secrets, approvals, or tenant credentials. Page
// content is untrusted data (section 6.11): text from pages is returned
// as observations verbatim and is never interpreted as instructions.
//
// Protocol: line-delimited JSON on stdin/stdout.
//   request:  {"id":"r1","op":"navigate","params":{...}}
//   response: {"id":"r1","ok":true,"result":{...}}
//             {"id":"r1","ok":false,"error":{"category":"BROWSER_SELECTOR","message":"...","native_code":"TimeoutError"}}
//
// Failure categories are the canonical Lumi taxonomy (spec 06 section
// 6.14); native Playwright codes ride along for debugging only.

import { chromium } from 'playwright';

const DEFAULT_TIMEOUT_MS = 30_000;
const MAX_REDIRECT_NAVIGATIONS = 3;

// ---------------------------------------------------------------------------
// Browser/session lifecycle
// ---------------------------------------------------------------------------

let browser = null;
let context = null;
let page = null;
let traceOn = false;

async function ensurePage(params) {
  if (!browser) {
    const launchOptions = { headless: params.headless !== false };
    // Proxy/network settings would be explicit instance config, not env.
    browser = await chromium.launch(launchOptions);
  }
  if (!context) {
    // Spec 06 section 6.6: v1 ships ISOLATED managed profiles only.
    // Attaching to an existing user profile is a separate, policy-gated
    // mode and is rejected unless the request carries explicit approval
    // from the policy core.
    if (params.profile && params.profile.mode === 'existing') {
      if (params.profile.policy_approved !== true) {
        throw failure('SECURITY_VIOLATION', 'existing-profile attachment requires explicit policy approval');
      }
      throw failure('SECURITY_VIOLATION', 'existing-profile attachment is not enabled in this build');
    }
    context = await browser.newContext({
      acceptDownloads: true,
      // No stored cookies/state: every session starts clean.
    });
    if (params.trace === true) {
      // Spec 06 section 6.12: tracing only in controlled modes, on demand.
      traceOn = true;
      await context.tracing.start({ screenshots: true, snapshots: true, sources: false });
    }
  }
  if (!page) {
    page = await context.newPage();
    page.setDefaultTimeout(DEFAULT_TIMEOUT_MS);
  }
  return page;
}

// ---------------------------------------------------------------------------
// Locators (spec 06 section 6.5 preference order)
// ---------------------------------------------------------------------------

function locate(target) {
  if (target.test_id !== undefined) {
    return page.getByTestId(target.test_id);
  }
  if (target.role !== undefined) {
    return page.getByRole(target.role, target.name !== undefined ? { name: target.name } : undefined);
  }
  if (target.label !== undefined) {
    return page.getByLabel(target.label, { exact: target.exact === true });
  }
  if (target.text !== undefined) {
    return page.getByText(target.text, { exact: target.exact === true });
  }
  if (target.css !== undefined) {
    // CSS/XPath only when semantic strategies are unavailable.
    return page.locator(target.css);
  }
  throw failure('BROWSER_SELECTOR', 'target carries no supported locator strategy');
}

function failure(category, message, nativeCode) {
  const error = new Error(message);
  error.lumiFailure = { category, message, native_code: nativeCode ?? null };
  return error;
}

function wrapError(error) {
  if (error.lumiFailure) return error.lumiFailure;
  const text = String(error && error.message ? error.message : error);
  if (error && error.name === 'TimeoutError') {
    // A timeout on a locator/interaction is a state failure; callers
    // re-observe (spec 18). Ambiguous submits are declared explicitly
    // by the submit op.
    return { category: 'BROWSER_STATE', message: text, native_code: 'TimeoutError' };
  }
  if (/strict mode violation|resolved to \d+ elements/i.test(text)) {
    return { category: 'BROWSER_SELECTOR', message: text, native_code: null };
  }
  if (/ERR_NAME_NOT_RESOLVED|ERR_CONNECTION|net::/i.test(text)) {
    return { category: 'NETWORK', message: text, native_code: null };
  }
  return { category: 'BROWSER_STATE', message: text, native_code: null };
}

// ---------------------------------------------------------------------------
// Operations (spec 06 section 6.4) — closed set
// ---------------------------------------------------------------------------

const ops = {
  async navigate(params) {
    const current = page.url();
    if (current && current !== 'about:blank' && page.isClosed() === false) {
      // Spec 06 section 6.10: navigation from a non-blank page to a new
      // origin is a state change the caller must observe.
    }
    const response = await page.goto(params.url, { waitUntil: params.wait_until ?? 'load' });
    return {
      url: page.url(),
      status: response ? response.status() : null,
      title: await page.title(),
    };
  },

  async read(params) {
    // Structured DOM read for verification (spec 06 section 6.13): the
    // caller verifies post-state through DOM/URL, not pixels.
    if (params.selector !== undefined) {
      const locator = locate(params.selector);
      await locator.waitFor({ state: params.state ?? 'visible' });
      const text = await locator.innerText();
      return { text, url: page.url() };
    }
    return {
      text: await page.locator('body').innerText(),
      url: page.url(),
      title: await page.title(),
    };
  },

  async click(params) {
    const locator = locate(params.target);
    await locator.click({ timeout: params.timeout_ms, button: params.button === 'right' ? 'right' : 'left' });
    await page.waitForLoadState(params.wait_until ?? 'load').catch(() => {});
    return { url: page.url() };
  },

  async fill(params) {
    const locator = locate(params.target);
    await locator.fill(params.value, { timeout: params.timeout_ms });
    return {};
  },

  async select(params) {
    const locator = locate(params.target);
    const chosen = await locator.selectOption(params.value, { timeout: params.timeout_ms });
    return { selected: chosen };
  },

  async check(params) {
    const locator = locate(params.target);
    if (params.uncheck === true) {
      await locator.uncheck({ timeout: params.timeout_ms });
    } else {
      await locator.check({ timeout: params.timeout_ms });
    }
    return {};
  },

  async upload(params) {
    // Spec 06 section 6.8: uploads identify an exact workspace file;
    // the policy core has already authorized this path.
    const locator = locate(params.target);
    await locator.setInputFiles(params.file_path, { timeout: params.timeout_ms });
    return {};
  },

  async download(params) {
    // Spec 06 section 6.7: downloads are saved into the controlled
    // workspace path supplied by the policy core.
    const downloadPromise = page.waitForEvent('download', { timeout: params.timeout_ms });
    if (params.trigger && params.trigger.target !== undefined) {
      await locate(params.trigger.target).click({ timeout: params.timeout_ms });
    }
    const download = await downloadPromise;
    const fs = await import('node:fs');
    const path = await import('node:path');
    const destination = path.join(params.workspace_dir, download.suggestedFilename());
    await download.saveAs(destination);
    const stat = fs.statSync(destination);
    return { path: destination, size: stat.size, suggested_filename: download.suggestedFilename() };
  },

  async wait(params) {
    if (params.selector !== undefined) {
      await locate(params.selector).waitFor({ state: params.state ?? 'visible', timeout: params.timeout_ms });
      return {};
    }
    await page.waitForTimeout(params.timeout_ms ?? 0);
    return {};
  },

  async screenshot(params) {
    // Spec 06 section 6.12/11.5: screenshots are selective and on demand;
    // element screenshots preferred over full page.
    if (params.target !== undefined) {
      const locator = locate(params.target);
      const buffer = await locator.screenshot({ timeout: params.timeout_ms });
      return { image_base64: buffer.toString('base64'), kind: 'element' };
    }
    if (params.full_page === true) {
      const buffer = await page.screenshot({ fullPage: true });
      return { image_base64: buffer.toString('base64'), kind: 'full_page' };
    }
    const buffer = await page.screenshot();
    return { image_base64: buffer.toString('base64'), kind: 'viewport' };
  },

  async submit(params) {
    // Spec 06 section 6.14/02 section 2.7: a submit whose outcome is not
    // observable in time is AMBIGUOUS, never a silent failure or success.
    const deadline = Date.now() + (params.timeout_ms ?? DEFAULT_TIMEOUT_MS);
    await locate(params.target).click({ timeout: params.timeout_ms });
    if (params.expect === undefined) {
      return { ambiguous: false, url: page.url() };
    }
    const expected = locate(params.expect);
    while (Date.now() < deadline) {
      try {
        if ((await expected.count()) > 0 && (await expected.isVisible())) {
          return { ambiguous: false, url: page.url() };
        }
      } catch {
        // keep polling until deadline
      }
      await page.waitForTimeout(200);
    }
    return {
      ambiguous: true,
      url: page.url(),
      category: 'AMBIGUOUS_STATE',
      message: 'submit outcome not observed within timeout',
    };
  },

  async stop_trace(params) {
    if (context && traceOn) {
      await context.tracing.stop({ path: params.trace_path ?? currentTracePath });
      traceOn = false;
      return { trace_path: params.trace_path ?? currentTracePath };
    }
    return {};
  },

  async close() {
    if (context) {
      if (traceOn) {
        await context.tracing.stop().catch(() => {});
        traceOn = false;
      }
      await context.close().catch(() => {});
      context = null;
      page = null;
    }
    if (browser) {
      await browser.close().catch(() => {});
      browser = null;
    }
    return {};
  },
};

let currentTracePath = null;

// Trace path is set via the navigate op params when tracing is on.
const origNavigate = ops.navigate;
ops.navigate = async function (params) {
  if (params.trace_path) currentTracePath = params.trace_path;
  return origNavigate(params);
};

// ---------------------------------------------------------------------------
// stdio loop
// ---------------------------------------------------------------------------

process.stdin.setEncoding('utf8');

let buffer = '';
let shuttingDown = false;

async function handle(line) {
  let request;
  try {
    request = JSON.parse(line);
  } catch {
    return; // framing error: ignore undecodable line, never execute blind
  }
  const { id, op, params = {} } = request;
  if (id === undefined || !Object.prototype.hasOwnProperty.call(ops, op)) {
    emit({
      id: id ?? null,
      ok: false,
      error: { category: 'BROWSER_STATE', message: `unknown op: ${String(op)}`, native_code: null },
    });
    return;
  }
  try {
    const page_ = await ensurePage(params);
    void page_;
    const result = await ops[op](params);
    emit({ id, ok: true, result });
    if (op === 'close') shuttingDown = true;
  } catch (error) {
    emit({ id, ok: false, error: wrapError(error) });
  }
}

function emit(message) {
  process.stdout.write(`${JSON.stringify(message)}\n`);
}

for await (const chunk of process.stdin) {
  buffer += chunk;
  let newlineIndex;
  while ((newlineIndex = buffer.indexOf('\n')) >= 0) {
    const line = buffer.slice(0, newlineIndex).trim();
    buffer = buffer.slice(newlineIndex + 1);
    if (line.length > 0) await handle(line);
  }
  if (shuttingDown) process.exit(0);
}
process.exit(0);
