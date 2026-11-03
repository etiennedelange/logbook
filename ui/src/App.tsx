import { useEffect } from 'react';
import { api, onIndexUpdated } from './api';
import { About } from './components/About';
import { DetailPane } from './components/DetailPane';
import { ListPane } from './components/ListPane';
import { Notice } from './components/Notice';
import { Sidebar } from './components/Sidebar';
import { StatsPane } from './components/StatsPane';
import { WhatsNew } from './components/WhatsNew';
import { presentAgents } from './model/agents';
import { RANGES } from './model/select';
import { useApp } from './state/store';
import { scheduleUpdateChecks, takeWhatsNew } from './update';
import { applyZoom, loadZoom, nextZoom, saveZoom } from './zoom';

function isTyping(t: EventTarget | null): boolean {
  return t instanceof HTMLElement && /^(INPUT|TEXTAREA|SELECT)$/.test(t.tagName);
}

/** Coming back to the window re-scans for new sessions, at most this often. */
const REINDEX_ON_FOCUS_MS = 30_000;

export default function App() {
  const refresh = useApp((s) => s.refresh);
  const view = useApp((s) => s.view);

  useEffect(() => {
    void refresh();
    let unlisten: (() => void) | undefined;
    let cancelled = false;
    void onIndexUpdated(() => void refresh()).then((u) => {
      if (cancelled) u();
      else unlisten = u;
    });
    let lastScan = Date.now();
    const onVisible = () => {
      if (document.visibilityState !== 'visible' || Date.now() - lastScan < REINDEX_ON_FOCUS_MS) return;
      lastScan = Date.now();
      void useApp.getState().reindex();
    };
    document.addEventListener('visibilitychange', onVisible);
    window.addEventListener('focus', onVisible);
    return () => {
      cancelled = true;
      unlisten?.();
      document.removeEventListener('visibilitychange', onVisible);
      window.removeEventListener('focus', onVisible);
    };
  }, [refresh]);

  useEffect(() => {
    // After an update, the new version shows the notes it was installed with, once.
    let cancelled = false;
    api.appInfo().then(
      ({ version }) => {
        const saved = takeWhatsNew(version);
        if (saved && !cancelled) useApp.getState().setWhatsNew({ ...saved, mode: 'installed' });
      },
      () => undefined,
    );
    const stop = scheduleUpdateChecks(() => useApp.getState().checkUpdate());
    return () => {
      cancelled = true;
      stop();
    };
  }, []);

  useEffect(() => {
    let zoom = loadZoom();
    void applyZoom(zoom);
    const set = (z: number) => {
      zoom = z;
      saveZoom(z);
      void applyZoom(z);
    };
    // WebView2's own zoom hotkeys are off in Tauri, so Ctrl+wheel is handled here.
    const onWheel = (e: WheelEvent) => {
      if (!e.ctrlKey) return;
      e.preventDefault();
      set(nextZoom(zoom, e.deltaY));
    };
    const onReset = (e: KeyboardEvent) => {
      if (e.ctrlKey && !e.metaKey && !e.altKey && e.key === '0') {
        e.preventDefault();
        set(1);
      }
    };
    window.addEventListener('wheel', onWheel, { passive: false });
    window.addEventListener('keydown', onReset);
    return () => {
      window.removeEventListener('wheel', onWheel);
      window.removeEventListener('keydown', onReset);
    };
  }, []);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const s = useApp.getState();
      // The dialogs are modal: they close themselves on Esc, and nothing behind them reacts.
      if (s.about || s.whatsNew) return;
      // Alt+1, Alt+2, ... pick an agent in sidebar order (again to clear it). Alt types nothing, so
      // this works in the search box too.
      if (e.altKey && !e.metaKey && !e.ctrlKey && /^[1-9]$/.test(e.key)) {
        const present = presentAgents(s.sessions);
        const agent = present[Number(e.key) - 1];
        if (agent && present.length > 1) {
          e.preventDefault();
          s.setAgent(s.agent === agent ? null : agent);
        }
        return;
      }
      if (isTyping(e.target) || e.metaKey || e.ctrlKey || e.altKey) return;
      if (e.key === '?') {
        s.setAbout(true);
        return;
      }
      if (e.key === 'Escape' && s.view === 'stats') {
        s.setView('sessions');
        return;
      }
      if (e.key === 'Escape') {
        // From the detail pane, Esc goes back to the selected row; from anywhere else it closes the detail.
        const inDetail = document.activeElement?.closest('[data-pane="detail"]');
        const row = document.querySelector<HTMLElement>('[role="option"][aria-selected="true"]');
        if (inDetail && row) row.focus();
        else s.clearSelection();
        return;
      }
      if (e.key === '/') {
        e.preventDefault();
        const focusSearch = () => {
          const box = document.querySelector<HTMLInputElement>('[data-search]');
          box?.focus();
          box?.select();
        };
        // From Stats, back to the list first: the search box is there once it has rendered.
        if (s.view === 'stats') {
          s.setView('sessions');
          requestAnimationFrame(focusSearch);
        } else focusSearch();
        return;
      }
      if (e.key === '5') {
        s.setView(s.view === 'stats' ? 'sessions' : 'stats');
        return;
      }
      if (e.key === 'r') void s.reindex();
      const range = RANGES.find((r) => r.hotkey === e.key);
      if (range) s.setRange(range.key);
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, []);

  return (
    // The list gives way before the detail pane does; below 960px the sidebar folds to an icon rail.
    <div className="grid h-full grid-cols-[220px_minmax(300px,2fr)_minmax(380px,3fr)] grid-rows-[minmax(0,1fr)] overflow-hidden bg-bg font-sans text-ink max-[959px]:grid-cols-[56px_minmax(260px,2fr)_minmax(340px,3fr)]">
      <Sidebar />
      {view === 'stats' ? (
        <StatsPane />
      ) : (
        <>
          <ListPane />
          <DetailPane />
        </>
      )}
      <Notice />
      <About />
      <WhatsNew />
    </div>
  );
}
