/**
 * Just enough of a W3C WebDriver client to drive the real app through tauri-driver, on fetch, so the
 * end-to-end tests need no dependencies. tauri-driver hands the session to WebKitWebDriver on Linux and
 * msedgedriver (WebView2) on Windows.
 */
import { spawn } from 'node:child_process';

const ELEMENT = 'element-6066-11e4-a23d-4ad52743d6ad';

async function call(base, method, path, body) {
  const res = await fetch(base + path, {
    method,
    headers: { 'content-type': 'application/json' },
    body: body === undefined ? undefined : JSON.stringify(body),
  });
  const json = await res.json().catch(() => ({}));
  if (!res.ok) {
    const v = json.value ?? {};
    throw new Error(`${method} ${path}: ${v.error ?? res.status}: ${String(v.message ?? '').split('\n')[0]}`);
  }
  return json.value;
}

const up = (url) => fetch(url + '/status').then((r) => r.ok, () => false);

/** Starts tauri-driver on `port` with `env` (the app inherits it) and resolves once it answers. */
export async function startDriver({ port = 4444, env = {} } = {}) {
  const base = `http://127.0.0.1:${port}`;
  if (await up(base)) throw new Error(`Port ${port} is already in use; stop whatever is serving it first.`);
  const args = ['--port', String(port)];
  // GitHub's Windows images ship msedgedriver here; locally, tauri-driver looks on PATH.
  if (process.platform === 'win32' && process.env.EDGEWEBDRIVER) {
    args.push('--native-driver', `${process.env.EDGEWEBDRIVER}\\msedgedriver.exe`);
  }
  const proc = spawn('tauri-driver', args, { env: { ...process.env, ...env }, stdio: ['ignore', 'inherit', 'inherit'] });
  const failed = new Promise((_, reject) => proc.once('error', reject));
  for (let i = 0; i < 100; i++) {
    if (await Promise.race([up(base), failed])) return { proc, base };
    await new Promise((r) => setTimeout(r, 200));
  }
  proc.kill();
  throw new Error('tauri-driver did not start');
}

export class Session {
  static async start(base, application) {
    const value = await call(base, 'POST', '/session', {
      capabilities: { alwaysMatch: { 'tauri:options': { application } } },
    });
    return new Session(base, value.sessionId);
  }

  constructor(base, id) {
    this.base = base;
    this.path = `/session/${id}`;
  }

  cmd(method, path, body) {
    return call(this.base, method, this.path + path, body);
  }

  quit() {
    return this.cmd('DELETE', '');
  }

  /** Runs `fn` in the page; its source must be self-contained. Arguments arrive as `arguments`. */
  exec(fn, ...args) {
    return this.cmd('POST', '/execute/sync', { script: `return (${fn}).apply(null, arguments)`, args });
  }

  /** Like exec, but `fn` may return a promise. */
  execAsync(fn, ...args) {
    const script = `const done = arguments[arguments.length - 1];
      Promise.resolve((${fn}).apply(null, Array.from(arguments).slice(0, -1)))
        .then((v) => done({ ok: v }), (e) => done({ err: String(e && e.message || e) }));`;
    return this.cmd('POST', '/execute/async', { script, args }).then((r) => {
      if (r && 'err' in r) throw new Error(r.err);
      return r?.ok;
    });
  }

  /** Polls `fn` in the page until it returns something truthy, and returns that. */
  async waitFor(what, fn, args = [], timeout = 15000) {
    const end = Date.now() + timeout;
    let last;
    while (Date.now() < end) {
      last = await this.exec(fn, ...args).catch((e) => (last = e));
      if (last && !(last instanceof Error)) return last;
      await new Promise((r) => setTimeout(r, 150));
    }
    throw new Error(`timed out waiting for ${what}${last instanceof Error ? ` (${last.message})` : ''}`);
  }

  async find(css) {
    const v = await this.cmd('POST', '/element', { using: 'css selector', value: css });
    // WebKitWebDriver may answer with the pre-W3C `ELEMENT` key.
    return v[ELEMENT] ?? v.ELEMENT ?? Object.values(v)[0];
  }

  click(el) {
    return this.cmd('POST', `/element/${el}/click`, {});
  }

  /** Types into an element as a user would, so React sees real input events. */
  type(el, text) {
    return this.cmd('POST', `/element/${el}/value`, { text });
  }

  /** Presses Esc in an element. WebDriver's own clear sets the value without an input event, which React misses. */
  escape(el) {
    return this.type(el, '');
  }

  async screenshot() {
    return Buffer.from(await this.cmd('GET', '/screenshot'), 'base64');
  }
}
