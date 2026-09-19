// Lumi Playwright browser worker.
//
// Process boundary (spec 06 section 6.3): this worker runs OUTSIDE the
// privileged policy core. It receives already-authorized operations and
// MUST NOT own policy, secrets, approvals, or tenant credentials. Page
// content is untrusted data (section 6.11): text from pages is returned
// as observations verbatim and is never interpreted as instructions.
//
// Protocol: line-delimited JSON on stdin/stdout.
//   request:  {"id":"r1","op":"navigate","url":"https://…"}
//   response: {"id":"r1","ok":true,"result":{...}}
//             {"id":"r1","ok":false,"error":{"category":"BROWSER_SELECTOR","message":"...","native_code":"TimeoutError"}}
//
// Failure categories are the canonical Lumi taxonomy (spec 06 section
// 6.14); native Playwright codes ride along for debugging only.

import { chromium } from 'playwright';

const DEFAULT_TIMEOUT_MS = 30_000;
const MAX_REDIRECT_NAVIGATIONS = 3;
const MAX_U32 = 0xFFFF_FFFF;

// ---------------------------------------------------------------------------
// Browser/session lifecycle
// ---------------------------------------------------------------------------

let browser = null;
let context = null;
let page = null;
let traceOn = false;
let currentTracePath = null;
let sessionStarted = false;
let sessionConfig = null;

const DEFAULT_SESSION = Object.freeze({
  headless: true,
  profile: null,
  trace: 'off',
});

const FAILURE_PROTOCOL = 'ProtocolError';

function hasOwn(value, key) {
  return value !== null && typeof value === 'object'
    && Object.prototype.hasOwnProperty.call(value, key);
}

function protocolFailure(message) {
  return failure('BROWSER_STATE', message, FAILURE_PROTOCOL);
}

function requireObject(value, path) {
  if (value === null || typeof value !== 'object' || Array.isArray(value)) {
    throw protocolFailure(`${path} must be an object`);
  }
  return value;
}

function rejectUnknownKeys(value, allowed, path) {
  for (const key of Object.keys(value)) {
    if (!allowed.has(key)) {
      throw protocolFailure(`${path}.${key} is not part of the browser protocol`);
    }
  }
}

function optionalString(value, path) {
  if (value === undefined) return;
  if (typeof value !== 'string' || value.length === 0) {
    throw protocolFailure(`${path} must be a non-empty string`);
  }
}

function requiredString(value, path) {
  optionalString(value, path);
  if (value === undefined) throw protocolFailure(`${path} is required`);
}

function optionalBoolean(value, path) {
  if (value !== undefined && typeof value !== 'boolean') {
    throw protocolFailure(`${path} must be a boolean`);
  }
}

function optionalTimeout(value, path) {
  if (value === undefined) return;
  if (!Number.isSafeInteger(value) || value < 0) {
    throw protocolFailure(`${path} must be a non-negative safe integer`);
  }
}

function validateTarget(value, path) {
  const target = requireObject(value, path);
  const strategy = target.strategy;
  if (typeof strategy !== 'string') {
    throw protocolFailure(`${path}.strategy is required`);
  }
  const common = new Set(['strategy', 'nth']);
  if (hasOwn(target, 'nth')
      && (!Number.isSafeInteger(target.nth) || target.nth < 0 || target.nth > MAX_U32)) {
    throw protocolFailure(`${path}.nth must be a non-negative u32`);
  }
  switch (strategy) {
    case 'test_id':
      rejectUnknownKeys(target, new Set([...common, 'test_id']), path);
      requiredString(target.test_id, `${path}.test_id`);
      break;
    case 'role':
      rejectUnknownKeys(target, new Set([...common, 'role', 'name']), path);
      requiredString(target.role, `${path}.role`);
      if (target.name !== undefined && target.name !== null) {
        requiredString(target.name, `${path}.name`);
      }
      break;
    case 'label':
      rejectUnknownKeys(target, new Set([...common, 'label', 'exact']), path);
      requiredString(target.label, `${path}.label`);
      if (typeof target.exact !== 'boolean') {
        throw protocolFailure(`${path}.exact must be a boolean`);
      }
      break;
    case 'text':
      rejectUnknownKeys(target, new Set([...common, 'text', 'exact']), path);
      requiredString(target.text, `${path}.text`);
      if (typeof target.exact !== 'boolean') {
        throw protocolFailure(`${path}.exact must be a boolean`);
      }
      break;
    case 'css':
      rejectUnknownKeys(target, new Set([...common, 'css']), path);
      requiredString(target.css, `${path}.css`);
      break;
    default:
      throw protocolFailure(`${path}.strategy ${JSON.stringify(strategy)} is unsupported`);
  }
  return target;
}

function validateOptionalTarget(value, path) {
  if (value !== undefined && value !== null) validateTarget(value, path);
  else if (value === null) throw protocolFailure(`${path} must be omitted, not null`);
}

function validateState(value, path) {
  if (value === undefined) return;
  if (!['attached', 'detached', 'visible', 'hidden'].includes(value)) {
    throw protocolFailure(`${path} is unsupported`);
  }
}

function validateSession(value) {
  const session = requireObject(value, 'session');
  rejectUnknownKeys(session, new Set(['headless', 'profile', 'trace']), 'session');
  optionalBoolean(session.headless, 'session.headless');

  let profile = null;
  if (session.profile !== undefined && session.profile !== null) {
    const rawProfile = requireObject(session.profile, 'session.profile');
    if (typeof rawProfile.mode !== 'string') {
      throw protocolFailure('session.profile.mode is required');
    }
    if (rawProfile.mode === 'isolated') {
      rejectUnknownKeys(rawProfile, new Set(['mode']), 'session.profile');
      profile = { mode: 'isolated' };
    } else if (rawProfile.mode === 'existing') {
      rejectUnknownKeys(rawProfile, new Set(['mode', 'policy_approved']), 'session.profile');
      if (typeof rawProfile.policy_approved !== 'boolean') {
        throw protocolFailure('session.profile.policy_approved must be a boolean');
      }
      if (rawProfile.policy_approved !== true) {
        throw failure('SECURITY_VIOLATION', 'existing-profile attachment requires explicit policy approval');
      }
      throw failure('SECURITY_VIOLATION', 'existing-profile attachment is not enabled in this build');
    } else {
      throw protocolFailure(`session.profile.mode ${JSON.stringify(rawProfile.mode)} is unsupported`);
    }
  } else if (session.profile === null) {
    profile = null;
  }

  let trace = 'off';
  if (session.trace !== undefined && session.trace !== null) {
    if (!['off', 'on_demand'].includes(session.trace)) {
      throw protocolFailure('session.trace must be "off" or "on_demand"');
    }
    trace = session.trace;
  } else if (session.trace === null) {
    trace = 'off';
  }

  return {
    headless: session.headless === undefined ? true : session.headless,
    profile,
    trace,
  };
}

function sessionForRequest(request) {
  const carriesSession = hasOwn(request, 'session');
  if (carriesSession && sessionStarted) {
    throw protocolFailure('session may only be supplied on the first request');
  }
  if (carriesSession) return validateSession(request.session);
  return sessionStarted ? sessionConfig : DEFAULT_SESSION;
}

async function ensurePage(session) {
  // Reject unsupported profile attachment before launching a browser.
  if (session.profile && session.profile.mode === 'existing') {
    if (session.profile.policy_approved !== true) {
      throw failure('SECURITY_VIOLATION', 'existing-profile attachment requires explicit policy approval');
    }
    throw failure('SECURITY_VIOLATION', 'existing-profile attachment is not enabled in this build');
  }
  if (!browser) {
    const launchOptions = { headless: session.headless };
    // Proxy/network settings would be explicit instance config, not env.
    browser = await chromium.launch(launchOptions);
  }
  if (!context) {
    // Spec 06 section 6.6: v1 ships ISOLATED managed profiles only.
    // Attaching to an existing user profile is a separate, policy-gated
    // mode and is rejected unless the request carries explicit approval
    // from the policy core.
    context = await browser.newContext({
      acceptDownloads: true,
      // No stored cookies/state: every session starts clean.
    });
    if (session.trace === 'on_demand') {
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
  validateTarget(target, 'target');
  let locator;
  if (target.test_id !== undefined) {
    locator = page.getByTestId(target.test_id);
  } else if (target.role !== undefined) {
    locator = page.getByRole(target.role, target.name !== undefined && target.name !== null
      ? { name: target.name } : undefined);
  } else if (target.label !== undefined) {
    locator = page.getByLabel(target.label, { exact: target.exact });
  } else if (target.text !== undefined) {
    locator = page.getByText(target.text, { exact: target.exact });
  } else if (target.css !== undefined) {
    // CSS/XPath only when semantic strategies are unavailable.
    locator = page.locator(target.css);
  } else {
    throw failure('BROWSER_SELECTOR', 'target carries no supported locator strategy');
  }
  return target.nth === undefined ? locator : locator.nth(target.nth);
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
    if (params.trace_path) currentTracePath = params.trace_path;
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
    await locator.click({ timeout: params.timeout_ms });
    await page.waitForLoadState('load').catch(() => {});
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
    await locate(params.trigger).click({ timeout: params.timeout_ms });
    const download = await downloadPromise;
    const fs = await import('node:fs');
    const path = await import('node:path');
    const filename = download.suggestedFilename();
    if (typeof filename !== 'string' || filename.trim().length === 0
        || filename.trim() !== filename || filename.endsWith('.')
        || /[\\/:\x00-\x1f]/.test(filename)) {
      throw failure('BROWSER_STATE', 'download filename is not a safe single path component');
    }
    const destination = path.join(params.workspace_dir, filename);
    await download.saveAs(destination);
    const stat = fs.statSync(destination);
    return { path: destination, size: stat.size, suggested_filename: download.suggestedFilename() };
  },

  async wait_for(params) {
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
    try {
      await locate(params.target).click({ timeout: params.timeout_ms });
    } catch (error) {
      // A timeout may occur after the remote control was dispatched. Keep
      // the submit conservative so the caller cannot blindly retry it.
      if (error && error.name === 'TimeoutError') {
        return {
          ambiguous: true,
          url: page.url(),
          category: 'AMBIGUOUS_STATE',
          message: 'submit click outcome not observed within timeout',
        };
      }
      throw error;
    }
    const expected = locate(params.expect.selector);
    while (Date.now() < deadline) {
      try {
        if ((await expected.count()) > 0 && (await expected.isVisible())) {
          return { ambiguous: false, url: page.url() };
        }
      } catch {
        // keep polling until deadline
      }
      try {
        await page.waitForTimeout(200);
      } catch {
        return {
          ambiguous: true,
          url: page.url(),
          category: 'AMBIGUOUS_STATE',
          message: 'submit outcome could not be observed after clicking submit',
        };
      }
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
      if (!currentTracePath) {
        throw failure('BROWSER_STATE', 'trace_path is required before stopping an on_demand trace', 'TracePathMissing');
      }
      await context.tracing.stop({ path: currentTracePath });
      traceOn = false;
      return { trace_path: currentTracePath };
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

function validateRequest(value) {
  const request = requireObject(value, 'request');
  if (hasOwn(request, 'params')) {
    throw protocolFailure('legacy nested params envelope is unsupported; operation fields must be flattened');
  }
  if (typeof request.id !== 'string') throw protocolFailure('request.id must be a string');
  if (typeof request.op !== 'string') throw protocolFailure('request.op must be a string');

  const session = sessionForRequest(request);
  const common = new Set(['id', 'op', 'session']);
  switch (request.op) {
    case 'navigate':
      rejectUnknownKeys(request, new Set([...common, 'url', 'wait_until', 'trace_path']), 'request');
      requiredString(request.url, 'request.url');
      optionalString(request.wait_until, 'request.wait_until');
      if (request.wait_until !== undefined
          && !['commit', 'domcontentloaded', 'load', 'networkidle'].includes(request.wait_until)) {
        throw protocolFailure('request.wait_until is unsupported');
      }
      optionalString(request.trace_path, 'request.trace_path');
      if (request.trace_path !== undefined && session.trace !== 'on_demand') {
        throw protocolFailure('request.trace_path requires session.trace="on_demand"');
      }
      break;
    case 'read':
      rejectUnknownKeys(request, new Set([...common, 'selector']), 'request');
      validateOptionalTarget(request.selector, 'request.selector');
      break;
    case 'click':
      rejectUnknownKeys(request, new Set([...common, 'target', 'timeout_ms']), 'request');
      validateTarget(request.target, 'request.target');
      optionalTimeout(request.timeout_ms, 'request.timeout_ms');
      break;
    case 'fill':
      rejectUnknownKeys(request, new Set([...common, 'target', 'value', 'timeout_ms']), 'request');
      validateTarget(request.target, 'request.target');
      requiredString(request.value, 'request.value');
      optionalTimeout(request.timeout_ms, 'request.timeout_ms');
      break;
    case 'select':
      rejectUnknownKeys(request, new Set([...common, 'target', 'value', 'timeout_ms']), 'request');
      validateTarget(request.target, 'request.target');
      if (!Array.isArray(request.value) || request.value.some((item) => typeof item !== 'string')) {
        throw protocolFailure('request.value must be an array of strings');
      }
      optionalTimeout(request.timeout_ms, 'request.timeout_ms');
      break;
    case 'check':
      rejectUnknownKeys(request, new Set([...common, 'target', 'uncheck', 'timeout_ms']), 'request');
      validateTarget(request.target, 'request.target');
      optionalBoolean(request.uncheck, 'request.uncheck');
      optionalTimeout(request.timeout_ms, 'request.timeout_ms');
      break;
    case 'upload':
      rejectUnknownKeys(request, new Set([...common, 'target', 'file_path', 'timeout_ms']), 'request');
      validateTarget(request.target, 'request.target');
      requiredString(request.file_path, 'request.file_path');
      optionalTimeout(request.timeout_ms, 'request.timeout_ms');
      break;
    case 'download':
      rejectUnknownKeys(request, new Set([...common, 'trigger', 'workspace_dir', 'timeout_ms']), 'request');
      validateTarget(request.trigger, 'request.trigger');
      requiredString(request.workspace_dir, 'request.workspace_dir');
      optionalTimeout(request.timeout_ms, 'request.timeout_ms');
      break;
    case 'wait_for':
      rejectUnknownKeys(request, new Set([...common, 'selector', 'state', 'timeout_ms']), 'request');
      validateOptionalTarget(request.selector, 'request.selector');
      validateState(request.state, 'request.state');
      optionalTimeout(request.timeout_ms, 'request.timeout_ms');
      break;
    case 'screenshot':
      rejectUnknownKeys(request, new Set([...common, 'target', 'full_page', 'timeout_ms']), 'request');
      validateOptionalTarget(request.target, 'request.target');
      optionalBoolean(request.full_page, 'request.full_page');
      optionalTimeout(request.timeout_ms, 'request.timeout_ms');
      break;
    case 'submit': {
      rejectUnknownKeys(request, new Set([...common, 'target', 'expect', 'timeout_ms']), 'request');
      validateTarget(request.target, 'request.target');
      const expect = requireObject(request.expect, 'request.expect');
      rejectUnknownKeys(expect, new Set(['selector']), 'request.expect');
      validateTarget(expect.selector, 'request.expect.selector');
      optionalTimeout(request.timeout_ms, 'request.timeout_ms');
      break;
    }
    case 'stop_trace':
    case 'close':
      rejectUnknownKeys(request, common, 'request');
      break;
    default:
      throw protocolFailure(`unknown op: ${String(request.op)}`);
  }
  return session;
}

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
  const id = request && typeof request === 'object' && typeof request.id === 'string'
    ? request.id : null;
  try {
    const session = validateRequest(request);
    const op = request.op;
    if (op !== 'close' && op !== 'stop_trace') {
      await ensurePage(session);
      if (!sessionStarted) {
        sessionConfig = session;
        sessionStarted = true;
      }
    } else if (!sessionStarted) {
      // close/stop_trace do not need to create a browser. Record a valid
      // first-session envelope only after validation so malformed requests
      // cannot alter lifecycle state.
      sessionConfig = session;
      sessionStarted = true;
    }
    const result = await ops[op](request);
    emit({ id, ok: true, result });
    if (op === 'close') shuttingDown = true;
  } catch (error) {
    emit({
      id: id ?? null,
      ok: false,
      error: wrapError(error),
    });
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
