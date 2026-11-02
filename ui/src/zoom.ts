import { getCurrentWebview } from '@tauri-apps/api/webview';

export const ZOOM_MIN = 0.5;
export const ZOOM_MAX = 2;
export const ZOOM_KEY = 'logbook-zoom';

export function clampZoom(z: number): number {
  if (!Number.isFinite(z)) return 1;
  return Math.min(ZOOM_MAX, Math.max(ZOOM_MIN, z));
}

/** Exponential step: one mouse-wheel notch (|deltaY| = 100) is about 10%; touchpad deltas are smooth. */
export function nextZoom(current: number, deltaY: number): number {
  return clampZoom(current * Math.exp(-deltaY / 1000));
}

export function loadZoom(): number {
  try {
    const raw = localStorage.getItem(ZOOM_KEY);
    return raw === null ? 1 : clampZoom(Number(raw));
  } catch {
    return 1;
  }
}

export function saveZoom(z: number): void {
  try {
    localStorage.setItem(ZOOM_KEY, String(z));
  } catch {
    /* storage unavailable: zoom still applies for this session */
  }
}

/** Real webview zoom (like browser zoom): layout reflows, so pane scrolling keeps working. */
export async function applyZoom(z: number): Promise<void> {
  try {
    await getCurrentWebview().setZoom(z);
  } catch {
    /* not running inside Tauri (tests, plain browser) */
  }
}
