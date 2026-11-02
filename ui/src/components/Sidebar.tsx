import { memo, useCallback, useMemo, useState } from 'react';
import { useShallow } from 'zustand/react/shallow';
import { cx } from '../cx';
import { useNow } from '../hooks/useNow';
import { RANGES, rangeCounts, repos, type Range } from '../model/select';
import { plural, shortRepo } from '../model/format';
import { useApp } from '../state/store';
import { Icon, type IconName } from './Icon';
import { SearchBox } from './SearchBox';

const RANGE_ICONS: Record<Range, IconName> = { all: 'sessions', today: 'today', week: 'week', month: 'month' };

/** Hidden (but still read out) when the window is narrow enough for the sidebar to be an icon rail. */
const RAIL_HIDE = 'max-[959px]:sr-only';

export function Sidebar() {
  const { sessions, range, setRange, repo, setRepo, status, theme, toggleTheme, reindex } = useApp(
    useShallow((s) => ({
      sessions: s.sessions,
      range: s.range,
      setRange: s.setRange,
      repo: s.repo,
      setRepo: s.setRepo,
      status: s.status,
      theme: s.theme,
      toggleTheme: s.toggleTheme,
      reindex: s.reindex,
    })),
  );
  // Counts only change at midnight or when the sessions do, not on every tick of the clock.
  const dayKey = useNow().toDateString();
  const today = useMemo(() => new Date(dayKey), [dayKey]);
  const [repoQuery, setRepoQuery] = useState('');
  const counts = useMemo(() => rangeCounts(sessions, today), [sessions, today]);
  const allRepos = useMemo(() => repos(sessions, range, today), [sessions, range, today]);
  const q = repoQuery.trim().toLowerCase();
  const toggleRepo = useCallback((name: string, on: boolean) => setRepo(on ? null : name), [setRepo]);
  const shown = useMemo(() => allRepos.filter((r) => r.name.toLowerCase().includes(q)), [allRepos, q]);
  const indexing = status?.indexing ?? false;
  const statusText = !status
    ? 'Loading index…'
    : indexing
      ? 'Indexing sessions…'
      : `${plural(status.total, 'session', 'sessions')} indexed`;

  return (
    <aside className="flex min-h-0 flex-col border-r border-line bg-side">
      <div
        data-tauri-drag-region
        className="flex h-[60px] flex-none items-center gap-[10px] border-b border-line px-[14px] max-[959px]:justify-center max-[959px]:px-0"
      >
        <span className="pointer-events-none grid h-[26px] w-[26px] flex-none place-items-center rounded-[7px] bg-accent text-accent-ink">
          <Icon name="logbook" size={15} />
        </span>
        <span data-tauri-drag-region className={cx('truncate text-[14px] font-semibold tracking-[-0.01em]', RAIL_HIDE)}>
          Logbook
        </span>
      </div>

      <nav aria-label="Time range" className="flex flex-none flex-col gap-[2px] px-2 pb-2 pt-3">
        {RANGES.map(({ key, label, hotkey }) => (
          <NavItem
            key={key}
            icon={RANGE_ICONS[key]}
            label={label}
            hotkey={hotkey}
            count={counts[key]}
            on={range === key}
            onClick={() => setRange(key)}
          />
        ))}
      </nav>

      <div className="flex flex-none items-center px-[18px] pb-[6px] pt-[14px] max-[959px]:hidden">
        <h2 className="flex-1 text-[12px] font-medium text-mute">Repositories</h2>
        {repo && (
          <button
            type="button"
            onClick={() => setRepo(null)}
            className="rounded-md px-[6px] text-[12px] leading-[18px] text-mute hover:bg-btn hover:text-soft"
          >
            Clear<span className="sr-only"> repository filter</span>
          </button>
        )}
      </div>
      {allRepos.length > 6 && (
        <div className="flex-none px-2 pb-1 max-[959px]:hidden">
          <SearchBox
            value={repoQuery}
            onChange={setRepoQuery}
            label="Filter repositories"
            placeholder="Filter repositories"
            searchable={false}
          />
        </div>
      )}
      <div className="flex min-h-0 flex-1 flex-col gap-px overflow-y-auto px-2 pb-2 max-[959px]:invisible">
        {shown.map((r) => (
          <RepoRow key={r.name} name={r.name} count={r.count} on={repo === r.name} onToggle={toggleRepo} />
        ))}
        {q !== '' && shown.length === 0 && (
          <p className="px-[10px] py-2 text-[12px] text-mute">No repositories match.</p>
        )}
        {sessions.length > 0 && allRepos.length === 0 && (
          <p className="px-[10px] py-2 text-[12px] text-mute">No repositories in this range.</p>
        )}
      </div>

      <footer className="flex flex-none items-center gap-2 border-t border-line px-[14px] py-3 max-[959px]:flex-col max-[959px]:px-2">
        <span
          className={cx('h-[6px] w-[6px] flex-none rounded-full', indexing && 'animate-pulse')}
          style={{ background: indexing ? 'var(--accent)' : status ? 'var(--ok)' : 'var(--mute)' }}
          title={statusText}
        />
        <span role="status" className={cx('min-w-0 flex-1 truncate text-[11px] leading-[1.35] text-mute', RAIL_HIDE)}>
          {statusText}
        </span>
        <IconButton label="Re-index sessions (r)" onClick={() => void reindex()} disabled={indexing}>
          <Icon name="refresh" size={14} className={cx(indexing && 'animate-spin')} />
        </IconButton>
        <IconButton label={theme === 'dark' ? 'Switch to light theme' : 'Switch to dark theme'} onClick={toggleTheme}>
          <Icon name={theme === 'dark' ? 'sun' : 'moon'} size={14} />
        </IconButton>
      </footer>
    </aside>
  );
}

function NavItem(p: {
  icon: IconName;
  label: string;
  hotkey: string;
  count: number;
  on: boolean;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      onClick={p.onClick}
      aria-current={p.on ? 'page' : undefined}
      aria-keyshortcuts={p.hotkey}
      title={`${p.label} (${p.hotkey})`}
      className={cx(
        'relative flex w-full items-center gap-[9px] rounded-md px-[10px] py-[7px] text-left text-[13px] font-medium max-[959px]:justify-center max-[959px]:px-0 max-[959px]:py-[9px]',
        p.on ? 'bg-btn text-ink' : 'text-soft hover:bg-hover',
      )}
    >
      <span
        className={cx(
          '-ml-[6px] h-[14px] w-[3px] rounded-sm max-[959px]:absolute max-[959px]:left-0 max-[959px]:ml-0',
          p.on ? 'bg-accent' : 'bg-transparent',
        )}
      />
      <Icon name={p.icon} size={15} className={cx('flex-none', p.on ? 'text-accent-text' : 'text-mute')} />
      <span className={cx('flex-1', RAIL_HIDE)}>{p.label}</span>
      <span className={cx('font-mono text-[11px] tabular-nums text-mute', RAIL_HIDE)}>{p.count}</span>
    </button>
  );
}

/** Memoized: picking a repository only re-renders the two rows whose state changes. */
const RepoRow = memo(function RepoRow(p: {
  name: string;
  count: number;
  on: boolean;
  onToggle: (name: string, on: boolean) => void;
}) {
  return (
    <button
      type="button"
      onClick={() => p.onToggle(p.name, p.on)}
      aria-pressed={p.on}
      title={p.name}
      className={cx(
        'flex w-full flex-none items-center gap-2 rounded-md px-[10px] py-[6px] text-left text-[13px]',
        p.on ? 'bg-btn text-ink' : 'text-soft hover:bg-hover',
      )}
    >
      <span
        className={cx('h-[6px] w-[6px] flex-none rounded-full', p.on ? 'bg-accent' : 'bg-line')}
        aria-hidden="true"
      />
      <span className="min-w-0 flex-1 truncate">{shortRepo(p.name)}</span>
      <span className="whitespace-nowrap font-mono text-[11px] tabular-nums text-mute">{p.count}</span>
    </button>
  );
});

function IconButton(p: { label: string; onClick: () => void; disabled?: boolean; children: React.ReactNode }) {
  return (
    <button
      type="button"
      title={p.label}
      aria-label={p.label}
      onClick={p.onClick}
      disabled={p.disabled}
      className="grid h-7 w-7 flex-none place-items-center rounded-md text-mute hover:bg-btn hover:text-soft disabled:pointer-events-none"
    >
      {p.children}
    </button>
  );
}
