import { memo, useCallback, useMemo, useState } from 'react';
import { useShallow } from 'zustand/react/shallow';
import { cx } from '../cx';
import { useNow } from '../hooks/useNow';
import { AGENT_ROWS, agentInfo, presentAgents, shownAgents } from '../model/agents';
import { RANGES, agentCounts, rangeCounts, repos, type Range } from '../model/select';
import { plural, shortRepo } from '../model/format';
import { useApp } from '../state/store';
import type { Agent } from '../types';
import { Icon, type IconName } from './Icon';
import { SearchBox } from './SearchBox';
import { Sources } from './Sources';

const RANGE_ICONS: Record<Range, IconName> = { all: 'sessions', today: 'today', week: 'week', month: 'month' };

/** Hidden (but still read out) when the window is narrow enough for the sidebar to be an icon rail. */
const RAIL_HIDE = 'max-[959px]:sr-only';

export function Sidebar() {
  const { sessions, range, setRange, repo, setRepo, agent, setAgent, status, theme, toggleTheme, reindex } = useApp(
    useShallow((s) => ({
      sessions: s.sessions,
      range: s.range,
      setRange: s.setRange,
      repo: s.repo,
      setRepo: s.setRepo,
      agent: s.agent,
      setAgent: s.setAgent,
      status: s.status,
      theme: s.theme,
      toggleTheme: s.toggleTheme,
      reindex: s.reindex,
    })),
  );
  const { view, setView } = useApp(useShallow((s) => ({ view: s.view, setView: s.setView })));
  const stats = view === 'stats';
  // Counts only change at midnight or when the sessions do, not on every tick of the clock.
  const dayKey = useNow().toDateString();
  const today = useMemo(() => new Date(dayKey), [dayKey]);
  const [repoQuery, setRepoQuery] = useState('');
  const counts = useMemo(() => rangeCounts(sessions, today), [sessions, today]);
  const allRepos = useMemo(() => repos(sessions, range, today, agent), [sessions, range, today, agent]);
  const present = useMemo(() => presentAgents(sessions), [sessions]);
  const [allAgents, setAllAgents] = useState(false);
  const agents = useMemo(() => {
    if (present.length < 2) return [];
    const shown = shownAgents(present, agent, allAgents);
    return agentCounts(sessions, shown, range, repo, today);
  }, [sessions, present, agent, allAgents, range, repo, today]);
  const toggleAgent = useCallback((a: Agent, on: boolean) => setAgent(on ? null : a), [setAgent]);
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
    <aside className={cx('flex min-h-0 flex-col bg-side', stats && 'st-on')}>
      <div
        data-tauri-drag-region
        className="flex h-[60px] flex-none items-center gap-[9px] px-[18px] max-[959px]:justify-center max-[959px]:px-0"
      >
        <Icon name="logbook" size={19} className="pointer-events-none flex-none text-accent-text" />
        {/* The wordmark is lowercase; the name in prose stays "Logbook". */}
        <span data-tauri-drag-region className={cx('truncate text-[16px] font-semibold tracking-[-0.02em]', RAIL_HIDE)}>
          logbook
        </span>
      </div>

      <nav aria-label="Time range" className="flex flex-none flex-col gap-[2px] px-2 pb-2 pt-1">
        {RANGES.map(({ key, label, hotkey }) => (
          <NavItem
            key={key}
            icon={RANGE_ICONS[key]}
            label={label}
            hotkey={hotkey}
            count={counts[key]}
            on={!stats && range === key}
            onClick={() => setRange(key)}
          />
        ))}
        <NavItem
          icon="stats"
          label="Stats"
          hotkey="5"
          on={stats}
          onClick={() => setView(stats ? 'sessions' : 'stats')}
        />
      </nav>

      {agents.length > 0 && (
        <nav aria-labelledby="agents-heading" className="flex flex-none flex-col">
          <h2
            id="agents-heading"
            className="px-[18px] pb-[6px] pt-[14px] text-[12px] font-medium text-mute max-[959px]:sr-only"
          >
            Agents
          </h2>
          <div className="flex flex-col gap-px px-2 max-[959px]:border-t max-[959px]:border-line max-[959px]:pt-2">
            {agents.map((a) => (
              <AgentRow
                key={a.agent}
                agent={a.agent}
                count={a.count}
                hotkey={hotkeyFor(present.indexOf(a.agent))}
                on={agent === a.agent}
                onToggle={toggleAgent}
              />
            ))}
            {present.length > AGENT_ROWS && (
              <button
                type="button"
                onClick={() => setAllAgents(!allAgents)}
                aria-expanded={allAgents}
                className="rounded-md px-[10px] py-[5px] text-left text-[12px] text-mute hover:bg-hover hover:text-soft max-[959px]:hidden"
              >
                {allAgents ? 'Show fewer' : `Show all ${present.length}`}
              </button>
            )}
          </div>
        </nav>
      )}

      <div className="flex flex-none items-center px-[18px] pb-[6px] pt-[14px] max-[959px]:hidden">
        <h2 className="flex-1 text-[12px] font-medium text-mute">Repositories</h2>
        {repo && (
          <button
            type="button"
            onClick={() => setRepo(null)}
            className="rounded-md px-[6px] text-[12px] leading-[18px] text-mute hover:bg-side-hover hover:text-soft"
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
      <div className="relative flex min-h-0 flex-1 flex-col gap-px overflow-y-auto px-2 pb-2 max-[959px]:invisible">
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

      {/* No visible index status: the spinning re-index button says it's working. Screen readers still hear it. */}
      <footer className="flex flex-none items-center justify-end gap-1 px-[10px] py-[10px] max-[959px]:flex-col max-[959px]:px-2">
        <span role="status" className="sr-only">
          {statusText}
        </span>
        <Sources />
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
  /** None for Stats, which isn't a set of sessions. */
  count?: number;
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
        'flex w-full items-center gap-[9px] rounded-lg px-[10px] py-[7px] text-left text-[13px] font-medium max-[959px]:justify-center max-[959px]:px-0 max-[959px]:py-[9px]',
        p.on ? 'bg-side-sel text-ink' : 'text-soft hover:bg-side-hover',
      )}
    >
      <Icon name={p.icon} size={15} className={cx('flex-none', p.on ? 'text-ink' : 'text-mute')} />
      <span className={cx('flex-1', RAIL_HIDE)}>{p.label}</span>
      {p.count !== undefined && (
        <span className={cx('font-mono text-[11px] tabular-nums text-mute', RAIL_HIDE)}>{p.count}</span>
      )}
    </button>
  );
}

/** `Alt+1` to `Alt+9`, in sidebar order; agents past the ninth have none. */
const hotkeyFor = (index: number): string | undefined => (index >= 0 && index < 9 ? `Alt+${index + 1}` : undefined);

/** One agent: its dot, name and count, like a repository row. In the icon rail only the dot shows. */
const AgentRow = memo(function AgentRow(p: {
  agent: Agent;
  count: number;
  hotkey: string | undefined;
  on: boolean;
  onToggle: (agent: Agent, on: boolean) => void;
}) {
  const info = agentInfo(p.agent);
  return (
    <button
      type="button"
      onClick={() => p.onToggle(p.agent, p.on)}
      aria-pressed={p.on}
      aria-keyshortcuts={p.hotkey}
      title={p.hotkey ? `${info.name} (${p.hotkey})` : info.name}
      className={cx(
        'flex w-full flex-none items-center gap-2 rounded-md px-[10px] py-[6px] text-left text-[13px] max-[959px]:justify-center max-[959px]:px-0 max-[959px]:py-[9px]',
        p.on ? 'bg-btn text-ink' : 'text-soft hover:bg-hover',
      )}
    >
      <span className="h-[7px] w-[7px] flex-none rounded-full" style={{ background: info.color }} aria-hidden="true" />
      <span className={cx('min-w-0 flex-1 truncate', RAIL_HIDE)}>{info.name}</span>
      <span className={cx('whitespace-nowrap font-mono text-[11px] tabular-nums text-mute', RAIL_HIDE)}>{p.count}</span>
    </button>
  );
});

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
        'flex w-full flex-none items-center gap-[9px] rounded-lg px-[10px] py-[6px] text-left text-[13px]',
        p.on ? 'bg-side-sel text-ink' : 'text-soft hover:bg-side-hover',
      )}
    >
      <Icon name="folder" size={15} className={cx('flex-none', p.on ? 'text-ink' : 'text-mute')} />
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
      className="grid h-7 w-7 flex-none place-items-center rounded-md text-mute hover:bg-side-hover hover:text-soft disabled:pointer-events-none"
    >
      {p.children}
    </button>
  );
}
