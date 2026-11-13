import { memo, useEffect, useId, useMemo, useRef, useState } from 'react';
import { useShallow } from 'zustand/react/shallow';
import { cx } from '../cx';
import { onListboxKey, useListRow } from '../hooks/useListRow';
import { useNow } from '../hooks/useNow';
import { dayGroup, plural, relTime, sessionTitle, shortRepo } from '../model/format';
import { agentInfo, presentAgents } from '../model/agents';
import { RANGES, repoKey, visibleRows, type Row } from '../model/select';
import { useApp } from '../state/store';
import type { Session } from '../types';
import { Icon } from './Icon';
import { PaneHeader } from './PaneHeader';
import { SearchBox } from './SearchBox';
import { Snippet } from './Snippet';

/** Rows rendered at first; more are added as the list scrolls (see `useRowWindow`). */
const PAGE = 80;

export function ListPane() {
  const { sessions, hits, hitsQuery, query, setQuery, searching, range, setRange, repo, setRepo } = useApp(
    useShallow((s) => ({
      sessions: s.sessions,
      hits: s.hits,
      hitsQuery: s.hitsQuery,
      query: s.query,
      setQuery: s.setQuery,
      searching: s.searching,
      range: s.range,
      setRange: s.setRange,
      repo: s.repo,
      setRepo: s.setRepo,
    })),
  );
  const { agent, setAgent } = useApp(useShallow((s) => ({ agent: s.agent, setAgent: s.setAgent })));
  const { loaded, error, status, selId, select, resume } = useApp(
    useShallow((s) => ({
      loaded: s.loaded,
      error: s.error,
      status: s.status,
      selId: s.selId,
      select: s.select,
      resume: s.resume,
    })),
  );
  const now = useNow();
  // Ranges and day groups only change at midnight, not on every tick of the clock.
  const dayKey = now.toDateString();
  const today = useMemo(() => new Date(dayKey), [dayKey]);
  const rows = useMemo(
    () => visibleRows(sessions, hits, range, repo, today, agent),
    [sessions, hits, range, repo, today, agent],
  );
  // Rows name their agent only when there's more than one and none is picked.
  const showAgent = useMemo(() => agent === null && presentAgents(sessions).length > 1, [agent, sessions]);
  const selIndex = useMemo(() => rows.findIndex((r) => r.session.id === selId), [rows, selId]);
  const { shown, sentinel, scroller } = useRowWindow(`${range}|${repo}|${agent}|${hits ? hitsQuery : ''}`, selIndex);
  const groups = useMemo(() => groupByDay(rows.slice(0, shown), today), [rows, shown, today]);
  const rangeLabel = RANGES.find((r) => r.key === range)!.label;
  // The counts describe the results on screen, which trail the search box while a search runs.
  const q = hits ? hitsQuery.trim() : '';
  const agentName = agent ? agentInfo(agent).name : null;
  const title = repo ? shortRepo(repo) : (agentName ?? rangeLabel);
  const sub = [
    q ? `${plural(rows.length, 'match', 'matches')} for “${q}”` : plural(rows.length, 'session', 'sessions'),
    repo ? agentName : null,
    repo || agent ? (range === 'all' && agent ? null : rangeLabel.toLowerCase()) : null,
  ]
    .filter(Boolean)
    .join(', ');
  const firstSelectable = selIndex >= 0 ? selId : rows[0]?.session.id;

  return (
    <section aria-label="Sessions" className="flex min-h-0 min-w-0 flex-col border-r border-line bg-list">
      <PaneHeader title={title}>
        {repo && (
          <button
            type="button"
            onClick={() => setRepo(null)}
            title={`Stop filtering to ${repo}`}
            className="rounded-md bg-btn px-[9px] py-[4px] text-[12px] text-soft hover:bg-btn-hover"
          >
            All repositories
          </button>
        )}
      </PaneHeader>
      <div className="flex-none px-3 pb-[6px] pt-3">
        <SearchBox
          value={query}
          onChange={setQuery}
          label="Search sessions"
          placeholder="Search names, repos and transcripts"
        />
        {sub && (
          <p data-list-sub className="m-0 mt-2 truncate px-1 text-[12px] text-mute">
            {sub}
          </p>
        )}
      </div>

      <div
        ref={scroller}
        role="listbox"
        aria-label="Sessions"
        aria-busy={searching || !loaded}
        onKeyDown={onListboxKey}
        // Only dims when a search takes long enough to notice.
        className={cx(
          'relative min-h-0 flex-1 overflow-y-auto px-2 pb-2 transition-opacity duration-150',
          searching && 'opacity-60 delay-300',
        )}
      >
        {error ? (
          <Empty title="Couldn’t load sessions" body={error} />
        ) : !loaded ? (
          <ListSkeleton />
        ) : rows.length === 0 ? (
          q ? (
            <Empty
              title="No matches"
              body="Every word has to start a word somewhere in the session. Try fewer or shorter words."
              action={
                range !== 'all' || repo || agent
                  ? {
                      label: 'Search all sessions',
                      onClick: () => {
                        setRange('all');
                        setRepo(null);
                        setAgent(null);
                      },
                    }
                  : undefined
              }
            />
          ) : sessions.length === 0 ? (
            status?.indexing ? (
              <Empty
                title="Indexing your sessions…"
                body="The first scan reads every transcript once; after that only changed sessions are read again."
              />
            ) : (
              <Empty
                title="No sessions yet"
                body="Sessions from the GitHub Copilot CLI (~/.copilot/session-state) and Claude Code (~/.claude/projects) show up here once they’re indexed."
              />
            )
          ) : (
            <Empty
              title="Nothing in this range"
              body="No sessions were active in this period."
              action={{ label: 'Show all sessions', onClick: () => setRange('all') }}
            />
          )
        ) : (
          <>
            {groups.map((g) => (
              <DayGroup key={g.key} label={g.label}>
                {g.rows.map((r) => (
                  <SessionRow
                    key={r.session.id}
                    row={r}
                    now={now}
                    showRepo={repo === null}
                    showAgent={showAgent}
                    selected={r.session.id === selId}
                    tabbable={r.session.id === firstSelectable}
                    onSelect={select}
                    onOpen={resume}
                  />
                ))}
              </DayGroup>
            ))}
            {shown < rows.length && <div ref={sentinel} aria-hidden="true" className="h-px" />}
          </>
        )}
      </div>
    </section>
  );
}

/**
 * How many of `rows` to render. Mounting every row takes tens of milliseconds once there are
 * a thousand sessions, so a new list starts with one page and grows a page at a time as its
 * end scrolls into view (and always far enough to include the selected row).
 */
function useRowWindow(listKey: string, selIndex: number) {
  const [state, setState] = useState({ listKey, shown: PAGE });
  const scroller = useRef<HTMLDivElement>(null);
  const observer = useRef<IntersectionObserver | null>(null);
  // Another filter or search starts again at one page; a refresh of the same list keeps its length.
  let shown = state.listKey === listKey ? state.shown : PAGE;
  if (selIndex >= shown) shown = selIndex + PAGE;
  if (state.listKey !== listKey || state.shown !== shown) setState({ listKey, shown });

  useEffect(() => () => observer.current?.disconnect(), []);

  const sentinel = (el: HTMLDivElement | null) => {
    observer.current?.disconnect();
    if (!el || typeof IntersectionObserver === 'undefined') return;
    observer.current = new IntersectionObserver(
      (entries) => {
        if (entries.some((e) => e.isIntersecting)) setState((s) => ({ ...s, shown: s.shown + PAGE }));
      },
      { root: scroller.current, rootMargin: '800px 0px' },
    );
    observer.current.observe(el);
  };
  return { shown, sentinel, scroller };
}

function groupByDay(rows: Row[], now: Date): { key: string; label: string; rows: Row[] }[] {
  const out: { key: string; label: string; rows: Row[] }[] = [];
  for (const r of rows) {
    const label = dayGroup(r.session.updatedAt ?? r.session.createdAt, now);
    const last = out[out.length - 1];
    if (last?.label === label) last.rows.push(r);
    // Ranked search hits can come back to a day already shown, so labels alone aren't unique keys.
    else out.push({ key: `${out.length}:${label}`, label, rows: [r] });
  }
  return out;
}

function DayGroup({ label, children }: { label: string; children: React.ReactNode }) {
  const id = useId();
  return (
    <div role="group" aria-labelledby={id}>
      <div id={id} className="sticky top-0 z-10 bg-list px-3 pb-[5px] pt-[12px] text-[11px] font-medium text-mute">
        {label}
      </div>
      {children}
    </div>
  );
}

/** Memoized: a filter click or selection change only re-renders the rows it actually changes. */
const SessionRow = memo(function SessionRow(p: {
  row: Row;
  now: Date;
  showRepo: boolean;
  showAgent: boolean;
  selected: boolean;
  tabbable: boolean;
  onSelect: (id: string) => void;
  onOpen: (id: string) => Promise<void>;
}) {
  const { session: s, snippet, hit } = p.row;
  const props = useListRow<HTMLDivElement>(
    p.selected,
    p.tabbable,
    () => p.onSelect(s.id),
    () => void p.onOpen(s.id),
  );
  const repo = repoKey(s);
  const title = sessionTitle(s);
  return (
    <div
      {...props}
      className={cx(
        'mb-px flex cursor-default flex-col gap-[4px] rounded-[10px] px-3 py-[9px]',
        p.selected ? 'bg-sel' : 'hover:bg-hover',
      )}
    >
      <div className="flex items-baseline gap-3">
        <span
          title={title.text}
          className={cx(
            'min-w-0 flex-1 truncate text-[13px]',
            title.source === 'name' ? 'font-medium text-ink' : 'text-soft',
            title.source === 'none' && 'italic text-mute',
          )}
        >
          {title.text}
        </span>
        <time
          dateTime={s.updatedAt ?? undefined}
          className="flex-none font-mono text-[11px] tabular-nums text-mute"
          title={s.updatedAt ? new Date(s.updatedAt).toLocaleString() : undefined}
        >
          {relTime(s.updatedAt, p.now)}
        </time>
      </div>
      <div className="flex min-w-0 items-center gap-[6px] text-[12px] text-mute">
        {p.showRepo && repo && (
          <span className="truncate" title={repo}>
            {shortRepo(repo)}
          </span>
        )}
        {s.branch && (
          <>
            {p.showRepo && repo && <span aria-hidden="true">·</span>}
            <span className="min-w-0 truncate font-mono text-[11px]" title={s.branch}>
              {s.branch}
            </span>
          </>
        )}
        {s.sourceDeletedAt && <DeletedTag />}
        <span className="ml-auto flex flex-none items-center gap-[10px] font-mono text-[11px]">
          {p.showAgent && <AgentTag agent={s.agent} />}
          <span className="tabular-nums" title={plural(s.messageCount, 'message', 'messages')}>
            {s.messageCount}
            <span className="sr-only"> messages</span>
          </span>
        </span>
      </div>
      {snippet && hit?.matched === 'file' && hit.file ? (
        <span className="flex min-w-0 items-center gap-[6px] text-[12px] leading-[1.5] text-soft">
          <Icon name="file" size={12} className="flex-none text-mute" />
          <span className="flex-none">{hit.file.created ? 'Created' : 'Edited'}</span>
          <Snippet text={snippet} className="min-w-0 truncate font-mono text-[11.5px]" />
          {hit.file.edits > 1 && <span className="flex-none">{hit.file.edits} times</span>}
        </span>
      ) : snippet && hit?.matched === 'session' ? (
        <span className="flex min-w-0 items-center gap-[6px] text-[12px] leading-[1.5] text-soft">
          <span className="flex-none">Session</span>
          <Snippet text={snippet} className="min-w-0 truncate font-mono text-[11.5px]" />
        </span>
      ) : (
        snippet && <Snippet text={snippet} className="line-clamp-2 text-[12px] leading-[1.5] text-soft" />
      )}
    </div>
  );
});

/**
 * Which agent a row is from: its short name, muted, after the repository and branch. Text only, so
 * it reads the same however many agents there are.
 */
function AgentTag({ agent }: { agent: Session['agent'] }) {
  const info = agentInfo(agent);
  return (
    <span title={info.name}>
      <span aria-hidden="true">{info.short}</span>
      <span className="sr-only">{info.name}</span>
    </span>
  );
}

/** An agent's dot and name, for the detail pane. */
export function AgentLabel({ agent, className }: { agent: Session['agent']; className?: string }) {
  const info = agentInfo(agent);
  return (
    <span className={cx('flex flex-none items-center gap-[5px] text-soft', className)}>
      <span className="h-[6px] w-[6px] flex-none rounded-full" style={{ background: info.color }} aria-hidden="true" />
      {info.name}
    </span>
  );
}

function DeletedTag() {
  return (
    <span
      title="The agent deleted this session’s file. Logbook kept a copy; it can’t be resumed."
      className="flex-none whitespace-nowrap rounded border border-line px-[5px] text-[11px] leading-[16px] text-mute"
    >
      source deleted
    </span>
  );
}

function ListSkeleton() {
  return (
    <div aria-hidden="true">
      {[62, 48, 70, 54, 40, 66].map((w, i) => (
        <div key={i} className="flex flex-col gap-[9px] px-3 py-[13px]">
          <div className="skeleton h-[10px]" style={{ width: `${w}%` }} />
          <div className="skeleton h-[8px] w-[34%]" />
        </div>
      ))}
    </div>
  );
}

function Empty({
  title,
  body,
  action,
}: {
  title: string;
  body?: string;
  action?: { label: string; onClick: () => void };
}) {
  return (
    <div className="flex flex-col items-center gap-[6px] px-6 py-12 text-center">
      <p className="text-[13px] font-medium text-soft">{title}</p>
      {body && <p className="max-w-[300px] text-[12px] leading-[1.55] text-mute">{body}</p>}
      {action && (
        <button
          type="button"
          onClick={action.onClick}
          className="mt-2 rounded-md bg-btn px-[10px] py-[5px] text-[12px] font-medium text-soft hover:bg-btn-hover"
        >
          {action.label}
        </button>
      )}
    </div>
  );
}
