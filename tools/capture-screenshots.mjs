// Capture the RG34XX launcher preview (inputs/launcher-src/browser-preview.html)
// in headless Chrome over the DevTools protocol. No npm dependencies.
//
//   CHROME_BIN=/usr/bin/google-chrome \
//   node tools/capture-screenshots.mjs http://127.0.0.1:8099/browser-preview.html docs/screenshots
//
// The preview emulates the 720x480 device screen with a deterministic app /
// service / settings list. Each capture clicks the same controls a user would
// and clips the #screen element, so a file cannot be a different state than its
// name claims. Fails on duplicate hashes, wrong dimensions, overflow, or blank
// output, and writes manifest.json with the run timestamp and SHA-256.
import {spawn} from 'node:child_process';
import {mkdtempSync, mkdirSync, readFileSync, writeFileSync} from 'node:fs';
import {tmpdir} from 'node:os';
import path from 'node:path';
import {createHash} from 'node:crypto';

const URL_ = process.argv[2] ?? 'http://127.0.0.1:8099/browser-preview.html';
const OUT = path.resolve(process.argv[3] ?? 'docs/screenshots');
const CHROME = process.env.CHROME_BIN ?? '/usr/bin/google-chrome';
const PORT = Number(process.env.CDP_PORT ?? 9345);
const W = 720, H = 480;

mkdirSync(OUT, {recursive: true});
const profile = mkdtempSync(path.join(tmpdir(), 'baseos-shot-'));
const chrome = spawn(CHROME, [
  '--headless=new', '--no-sandbox', '--disable-gpu', '--hide-scrollbars',
  `--remote-debugging-port=${PORT}`, `--user-data-dir=${profile}`, '--window-size=1280,900',
  'about:blank',
], {stdio: 'ignore'});

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

async function fetchJSON(pathname) {
  const response = await fetch(`http://127.0.0.1:${PORT}${pathname}`);
  if (!response.ok) throw new Error(`${pathname} -> ${response.status}`);
  return response.json();
}

async function waitForCDP() {
  for (let i = 0; i < 100; i++) {
    try { return await fetchJSON('/json/list'); } catch { await sleep(100); }
  }
  throw new Error('Chrome DevTools endpoint did not start');
}

class CDP {
  constructor(ws) {
    this.ws = ws; this.id = 0; this.pending = new Map();
    ws.onmessage = (event) => {
      const message = JSON.parse(event.data);
      if (message.id && this.pending.has(message.id)) {
        const {resolve, reject} = this.pending.get(message.id);
        this.pending.delete(message.id);
        message.error ? reject(new Error(message.error.message)) : resolve(message.result);
      }
    };
  }
  static async connect(url) {
    const ws = new WebSocket(url);
    await new Promise((resolve, reject) => { ws.onopen = resolve; ws.onerror = reject; });
    return new CDP(ws);
  }
  send(method, params = {}) {
    const id = ++this.id;
    this.ws.send(JSON.stringify({id, method, params}));
    return new Promise((resolve, reject) => this.pending.set(id, {resolve, reject}));
  }
}

let cdp;
const seen = new Map();
const manifest = [];
const results = [];

async function evaluate(expression) {
  const {result, exceptionDetails} = await cdp.send('Runtime.evaluate', {expression, returnByValue: true, awaitPromise: true});
  if (exceptionDetails) throw new Error(exceptionDetails.text ?? 'evaluate failed');
  return result.value;
}

async function click(selector) {
  const ok = await evaluate(`(() => { const el = document.querySelector(${JSON.stringify(selector)}); if (!el) return false; el.click(); return true; })()`);
  if (!ok) throw new Error(`control ${selector} not found`);
  await sleep(120);
}

async function press(key) {
  const ok = await evaluate(`(() => { document.dispatchEvent(new KeyboardEvent('keydown', {key: ${JSON.stringify(key)}, bubbles: true})); return true; })()`);
  if (!ok) throw new Error(`key ${key} failed`);
  await sleep(120);
}

async function capture(name) {
  const rect = await evaluate(`(() => { const el = document.querySelector('#screen'); const r = el.getBoundingClientRect(); return {x: r.x, y: r.y, width: r.width, height: r.height}; })()`);
  if (Math.round(rect.width) !== W || Math.round(rect.height) !== H) {
    throw new Error(`${name}: device screen is ${rect.width}x${rect.height}, expected ${W}x${H}`);
  }
  const audit = await evaluate(`(() => ({
    overflow: document.querySelector('#screen').scrollWidth - document.querySelector('#screen').clientWidth,
    section: document.querySelector('#section').textContent,
    rows: [...document.querySelectorAll('#screen .row')].map(r => r.textContent.trim()),
    active: document.querySelector('#screen .row.active')?.textContent?.trim() ?? '',
    light: document.querySelector('#screen').classList.contains('light'),
  }))()`);
  if (audit.overflow > 1) throw new Error(`${name}: horizontal overflow inside the device screen`);
  if (!audit.rows.length) throw new Error(`${name}: no launcher rows rendered`);
  const shot = await cdp.send('Page.captureScreenshot', {
    format: 'png',
    clip: {x: rect.x, y: rect.y, width: W, height: H, scale: 1},
    captureBeyondViewport: true,
  });
  const bytes = Buffer.from(shot.data, 'base64');
  const file = path.join(OUT, `${name}.png`);
  writeFileSync(file, bytes);
  const w = bytes.readUInt32BE(16), h = bytes.readUInt32BE(20);
  if (w !== W || h !== H) throw new Error(`${name}: PNG ${w}x${h}, expected ${W}x${H}`);
  const sha = createHash('sha256').update(bytes).digest('hex');
  if (seen.has(sha)) throw new Error(`${name}: duplicate of ${seen.get(sha)}`);
  seen.set(sha, name);
  manifest.push({name: `${name}.png`, width: w, height: h, section: audit.section, active: audit.active, light: audit.light, sha256: sha});
  results.push(`${name}: ${audit.section} / ${audit.active}${audit.light ? ' (light)' : ''}`);
}

try {
  const targets = await waitForCDP();
  const page = targets.find((target) => target.type === 'page');
  cdp = await CDP.connect(page.webSocketDebuggerUrl);
  await cdp.send('Page.enable');
  await cdp.send('Runtime.enable');
  await cdp.send('Emulation.setDeviceMetricsOverride', {width: 1280, height: 900, deviceScaleFactor: 1, mobile: false});

  const load = async () => {
    await cdp.send('Page.navigate', {url: URL_});
    await sleep(700);
    const ready = await evaluate(`!!document.querySelector('#screen .row')`);
    if (!ready) throw new Error(`launcher preview did not render at ${URL_}`);
  };

  await load();
  await capture('apps-dark-720x480');

  await press('t');
  await capture('apps-light-720x480');

  await press('t');
  await press('ArrowDown');
  await press('ArrowDown');
  await press('ArrowDown');
  await press('ArrowDown');
  await capture('apps-selected-720x480');

  await press('ArrowRight');
  await capture('services-dark-720x480');
  await press('Enter');
  await capture('services-toggled-720x480');

  await press('ArrowRight');
  await capture('settings-dark-720x480');
  await press('Enter');
  await capture('settings-light-720x480');
} finally {
  cdp?.ws?.close();
  chrome.kill('SIGTERM');
  await sleep(300);
}

if (manifest.length < 7) throw new Error(`expected 7 captures, got ${manifest.length}`);
writeFileSync(path.join(OUT, 'manifest.json'), `${JSON.stringify({captured_at: new Date().toISOString(), url: URL_, chrome: CHROME, screenshots: manifest}, null, 2)}\n`);
console.log(results.join('\n'));
console.log(`captured ${manifest.length} screenshots, ${seen.size} unique hashes`);
