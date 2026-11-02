import { memo, useEffect, useId, useMemo, useRef, useState } from 'react';
import { useShallow } from 'zustand/react/shallow';
import { cx } from '../cx';
import { onListboxKey, useListRow } from '../hooks/useListRow';
import { useNow } from '../hooks/useNow';
import { dayGroup, plural, relTime, sessionTitle, shortRepo } from '../model/format';
import { RANGES, repoKey, visibleRows, type Row } from '../model/select';
import { useApp } from '../state/store';
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
  const rows = useMemo(() => visibleRows(sessions, hits, range, repo, today), [sessions, hits, range, repo, today]);
  const selIndex = useMemo(() => rows.findIndex((r) => r.session.id === selId), [rows, selId]);
  const { shown, sentinel, scroller } = useRowWindow(`${range}|${repo}|${hits ? hitsQuery : ''}`, selIndex);
  const groups = useMemo(() => groupByDay(rows.slice(0, shown), today), [rows, shown, today]);
  const rangeLabel = RANGES.find((r) => r.key === range)!.label;
  // The counts describe the results on screen, which trail the search box while a search runs.
  const q = hits ? hitsQuery.trim() : '';
  const title = repo ? shortRepo(repo) : rangeLabel;
  const sub = [
    q ? `${plural(rows.length, 'match', 'matches')} for “${q}”` : plural(rows.length, 'session', 'sessions'),
    repo ? rangeLabel.toLowerCase() : null,
  ]
    .filter(Boolean)
    .join(', ');
  const firstSelectable = selIndex >= 0 ? selId : rows[0]?.session.id;

  return (
    <section aria-label="Sessions" className="flex min-h-0 min-w-0 flex-col border-r border-line">
      <PaneHeader title={title} sub={sub}>
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
      <div className="flex-none border-b border-line px-3 py-[10px]">
        <SearchBox
          value={query}
          onChange={setQuery}
          label="Search sessions"
          placeholder="Search names, repos and transcripts"
        />
      </div>

      <div
        ref={scroller}
        role="listbox"
        aria-label="Sessions"
        aria-busy={searching || !loaded}
        onKeyDown={onListboxKey}
        // Only dims when a search takes long enough to notice.
        className={cx(
          'min-h-0 flex-1 overflow-y-auto transition-opacity duration-150',
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
                range !== 'all' || repo
                  ? {
                      label: 'Search all sessions',
                      onClick: () => {
                        setRange('all');
                        setRepo(null);
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
                body="Sessions from the GitHub Copilot CLI (~/.copilot/session-state) show up here once they’re indexed."
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
      <div
        id={id}
        className="sticky top-0 z-10 border-b border-line-soft bg-bg px-4 pb-[5px] pt-[10px] text-[11px] font-medium text-mute"
      >
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
  selected: boolean;
  tabbable: boolean;
  onSelect: (id: string) => void;
  onOpen: (id: string) => Promise<void>;
}) {
  const { session: s, snippet } = p.row;
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
        'relative flex cursor-default flex-col gap-[4px] border-b border-line-soft px-4 py-[10px]',
        p.selected ? 'bg-sel' : 'hover:bg-hover',
      )}
    >
      {p.selected && <span aria-hidden="true" className="absolute inset-y-0 left-0 w-[2px] bg-accent" />}
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
        <span
          className="ml-auto flex-none font-mono text-[11px] tabular-nums"
          title={plural(s.messageCount, 'message', 'messages')}
        >
          {s.messageCount}
          <span className="sr-only"> messages</span>
        </span>
      </div>
      {snippet && <Snippet text={snippet} className="line-clamp-2 text-[12px] leading-[1.5] text-soft" />}
    </div>
  );
});

function ListSkeleton() {
  return (
    <div aria-hidden="true">
      {[62, 48, 70, 54, 40, 66].map((w, i) => (
        <div key={i} className="flex flex-col gap-[9px] border-b border-line-soft px-4 py-[13px]">
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
