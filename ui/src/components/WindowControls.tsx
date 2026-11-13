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

  // Windows 11 caption buttons: 46px wide, the full height of the bar, square, flush with the top-right
  // corner (-mr-3 cancels the header's right padding), close turns red edge to edge.
  const base = 'grid w-[46px] flex-none place-items-center text-soft';
  return (
    <div className="-mr-3 ml-1 flex flex-none self-stretch" role="group" aria-label="Window">
      <button
        type="button"
        title="Minimise"
        aria-label="Minimise"
        onClick={() => void appWindow()?.minimize()}
        className={`${base} hover:bg-btn hover:text-ink`}
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
        className={`${base} hover:bg-btn hover:text-ink`}
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
        className={`${base} hover:bg-[#c42b1c] hover:text-white`}
      >
        <svg {...GLYPH} aria-hidden="true">
          <path d="M1.5 1.5l7 7M8.5 1.5l-7 7" />
        </svg>
      </button>
    </div>
  );
}
