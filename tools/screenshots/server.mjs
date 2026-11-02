/**
 * The UI's dev server in mock mode (`ui/src/dev/mock.ts`'s fake sessions), started and stopped by the
 * scripts that drive it in a browser: capture.mjs and check.mjs.
 */
import { spawn } from 'node:child_process';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = join(dirname(fileURLToPath(import.meta.url)), '..', '..');

const up = (url) => fetch(url).then((r) => r.ok, () => false);

/** Starts `vp dev --mode mock` on `port` and resolves once it answers. */
export async function startServer(port) {
  const url = `http://127.0.0.1:${port}/`;
  // A server left over from an earlier run would serve stale code; don't silently use that.
  if (await up(url)) throw new Error(`Port ${port} is already in use; stop whatever is serving ${url} first.`);
  const win = process.platform === 'win32';
  const vp = join(root, 'ui', 'node_modules', '.bin', win ? 'vp.cmd' : 'vp');
  const proc = spawn(vp, ['dev', '--mode', 'mock', '--host', '127.0.0.1', '--port', String(port), '--strictPort'], {
    cwd: join(root, 'ui'),
    stdio: ['ignore', 'pipe', 'inherit'],
    shell: win,
    // Its own process group, so stopServer() takes Vite's workers down with it.
    detached: !win,
  });
  for (let i = 0; i < 100; i++) {
    if (await up(url)) return { proc, url };
    await new Promise((r) => setTimeout(r, 300));
  }
  stopServer(proc);
  throw new Error('Dev server did not start');
}

export function stopServer(proc) {
  if (process.platform === 'win32') spawn('taskkill', ['/pid', String(proc.pid), '/T', '/F'], { stdio: 'ignore' });
  else {
    try {
      process.kill(-proc.pid);
    } catch {
      /* already gone */
    }
  }
}
