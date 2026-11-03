/**
 * Just enough of a Chrome DevTools Protocol client to drive the real app on Windows, on Node's own fetch and
 * WebSocket, so the end-to-end tests need no dependencies. A debug build started with
 * LOGBOOK_E2E_DEVTOOLS_PORT=<port> opens WebView2's DevTools port (src-tauri/src/lib.rs). This replaces
 * tauri-driver: msedgedriver opens that port through WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS, which WebView2
 * ignores because wry passes its own arguments, so the session never started.
 */
import { execFileSync, spawn } from 'node:child_process';

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

/** Starts `app` with `env` (on top of this process's) and connects to its page. */
export async function launch(app, { env = {}, port = 9333, timeout = 60_000 } = {}) {
  const base = `http://127.0.0.1:${port}`;
  if (await fetch(`${base}/json/version`).then(() => true, () => false)) {
    throw new Error(`Port ${port} is already in use; stop whatever is serving it first.`);
  }
  const proc = spawn(app, [], {
    env: { ...process.env, ...env, LOGBOOK_E2E_DEVTOOLS_PORT: String(port) },
    stdio: ['ignore', 'inherit', 'inherit'],
  });
  let exited = null;
  proc.once('exit', (code) => (exited = code ?? 'signal'));
  proc.once('error', (e) => (exited = e.message));

  const end = Date.now() + timeout;
  let page;
  while (!page) {
    if (exited !== null) {
      // The single-instance plugin hands a second launch to a running Logbook and exits.
      throw new Error(`the app exited (${exited}) before its page was up; is Logbook already running?`);
    }
    if (Date.now() > end) {
      kill(proc);
      throw new Error(`no app page on DevTools port ${port} after ${timeout / 1000}s`);
    }
    const targets = await fetch(`${base}/json`).then((r) => r.json(), () => []);
    page = targets.find((t) => t.type === 'page' && !t.url.startsWith('devtools:'));
    if (!page) await sleep(300);
  }
  const s = new Session(proc, new WebSocket(page.webSocketDebuggerUrl));
  await s.opened;
  return s;
}

function kill(proc) {
  if (proc.exitCode !== null || !proc.pid) return;
  try {
    // /T: WebView2's msedgewebview2 processes too.
    if (process.platform === 'win32') execFileSync('taskkill', ['/F', '/T', '/PID', String(proc.pid)], { stdio: 'ignore' });
    else proc.kill();
  } catch {}
}

export class Session {
  constructor(proc, ws) {
    this.proc = proc;
    this.ws = ws;
    this.id = 0;
    this.pending = new Map();
    this.opened = new Promise((resolve, reject) => {
      ws.onopen = resolve;
      ws.onerror = () => reject(new Error('could not connect to the page'));
    });
    ws.onmessage = (e) => {
      const msg = JSON.parse(e.data);
      const p = this.pending.get(msg.id);
      if (!p) return;
      this.pending.delete(msg.id);
      if (msg.error) p.reject(new Error(`${p.method}: ${msg.error.message}`));
      else p.resolve(msg.result);
    };
  }

  send(method, params = {}) {
    return new Promise((resolve, reject) => {
      const id = ++this.id;
      this.pending.set(id, { resolve, reject, method });
      this.ws.send(JSON.stringify({ id, method, params }));
    });
  }

  quit() {
    this.ws.close();
    kill(this.proc);
  }

  /** Runs `fn` (a function or its source; self-contained) in the page with `args`, awaiting a promise. */
  async exec(fn, ...args) {
    const expression = `(${fn}).apply(null, ${JSON.stringify(args)})`;
    const r = await this.send('Runtime.evaluate', { expression, awaitPromise: true, returnByValue: true });
    if (r.exceptionDetails) {
      const d = r.exceptionDetails;
      throw new Error(d.exception?.description?.split('\n')[0] ?? d.text);
    }
    return r.result.value;
  }

  /** Polls `fn` in the page until it returns something truthy, and returns that. */
  async waitFor(what, fn, args = [], timeout = 15000) {
    const end = Date.now() + timeout;
    let last;
    while (Date.now() < end) {
      last = await this.exec(fn, ...args).catch((e) => e);
      if (last && !(last instanceof Error)) return last;
      await sleep(150);
    }
    throw new Error(`timed out waiting for ${what}${last instanceof Error ? ` (${last.message})` : ''}`);
  }

  /** Checks `css` matches an element and returns it as a handle for click, type and escape. */
  async find(css) {
    if (!(await this.exec((c) => !!document.querySelector(c), css))) throw new Error(`no element matches ${css}`);
    return css;
  }

  click(css) {
    return this.exec((c) => document.querySelector(c).click(), css);
  }

  /** Types into an element as a user would, so React sees real input events. */
  async type(css, text) {
    await this.exec((c) => document.querySelector(c).focus(), css);
    await this.send('Input.insertText', { text });
  }

  /** Presses Esc in an element. */
  async escape(css) {
    await this.exec((c) => document.querySelector(c).focus(), css);
    const key = { key: 'Escape', code: 'Escape', windowsVirtualKeyCode: 27 };
    await this.send('Input.dispatchKeyEvent', { type: 'keyDown', ...key });
    await this.send('Input.dispatchKeyEvent', { type: 'keyUp', ...key });
  }

  async screenshot() {
    const r = await this.send('Page.captureScreenshot', { format: 'png' });
    return Buffer.from(r.data, 'base64');
  }
}
