import { useEffect, useState } from 'react';
import { getCurrentWindow } from '@tauri-apps/api/window';

/** The OS window, or null in a plain browser (tests, and `dev:mock` before the mock is installed). */
function appWindow() {
  try {
    return getCurrentWindow();
  } catch {
    return null;
  }
}

const GLYPH = { width: 10, height: 10, viewBox: '0 0 10 10', fill: 'none', stroke: 'currentColor', strokeWidth: 1 };

/**
 * Minimise, maximise and close. The window has no native title bar (`decorations: false` in
 * tauri.conf.json); the pane headers are the title bar, so these sit inline at the end of the last one.
 */
export function WindowControls() {
  const [maximized, setMaximized] = useState(false);

  useEffect(() => {
    const w = appWindow();
    if (!w) return;
    let off: (() => void) | undefined;
    let cancelled = false;
    const sync = () => w.isMaximized().then(setMaximized, () => undefined);
    void sync();
    w.onResized(() => void sync()).then(
      (u) => (cancelled ? u() : (off = u)),
      () => undefined,
    );
    return () => {
      cancelled = true;
      off?.();
    };
  }, []);

  const base = 'grid h-[28px] w-[34px] flex-none place-items-center rounded-md text-mute';
  return (
    <div className="ml-[2px] flex flex-none items-center gap-px" role="group" aria-label="Window">
      <button
        type="button"
        title="Minimise"
        aria-label="Minimise"
        onClick={() => void appWindow()?.minimize()}
        className={`${base} hover:bg-btn hover:text-soft`}
      >
        <svg {...GLYPH} aria-hidden="true">
          <path d="M1 5.5h8" />
        </svg>
      </button>
      <button
        type="button"
        title={maximized ? 'Restore' : 'Maximise'}
        aria-label={maximized ? 'Restore' : 'Maximise'}
        onClick={() => void appWindow()?.toggleMaximize()}
        className={`${base} hover:bg-btn hover:text-soft`}
      >
        <svg {...GLYPH} aria-hidden="true">
          {maximized ? (
            <>
              <path d="M2.5 3.5h5.5v5.5h-5.5z" />
              <path d="M4 3.5V1.5h5v5H8" />
            </>
          ) : (
            <rect x="1.5" y="1.5" width="7" height="7" />
          )}
        </svg>
      </button>
      <button
        type="button"
        title="Close"
        aria-label="Close"
        onClick={() => void appWindow()?.close()}
        className={`${base} hover:bg-c-fail hover:text-white`}
      >
        <svg {...GLYPH} aria-hidden="true">
          <path d="M1.5 1.5l7 7M8.5 1.5l-7 7" />
        </svg>
      </button>
    </div>
  );
}
