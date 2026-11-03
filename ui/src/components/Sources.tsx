import { useEffect, useId, useMemo, useRef } from 'react';
import { useShallow } from 'zustand/react/shallow';
import { cx } from '../cx';
import { useNow } from '../hooks/useNow';
import { agentInfo, sourceRows, type SourceRow } from '../model/agents';
import { plural } from '../model/format';
import { useApp } from '../state/store';
import { Icon } from './Icon';

/** Hidden (but still read out) in the narrow icon rail, as in the rest of the sidebar. */
const RAIL_HIDE = 'max-[959px]:sr-only';

/** Dots on the trigger before they'd stop reading as a cluster. */
const MAX_DOTS = 4;

/**
 * "2 sources" in the sidebar footer: the folders Logbook reads, one per agent. It opens a small
 * popover listing each agent's folder, its sessions, and when Logbook last read them.
 */
export function Sources() {
  const { sources, sessions, open, setOpen } = useApp(
    useShallow((s) => ({ sources: s.sources, sessions: s.sessions, open: s.sourcesOpen, setOpen: s.setSourcesOpen })),
  );
  const rows = useMemo(() => sourceRows(sources ?? [], sessions), [sources, sessions]);
  const found = rows.filter((r) => r.found);
  const id = useId();
  const wrap = useRef<HTMLDivElement>(null);
  const trigger = useRef<HTMLButtonElement>(null);

  // A click anywhere else closes it, like any popover.
  useEffect(() => {
    if (!open) return;
    const onDown = (e: MouseEvent) => {
      if (!wrap.current?.contains(e.target as Node)) setOpen(false);
    };
    document.addEventListener('mousedown', onDown);
    return () => document.removeEventListener('mousedown', onDown);
  }, [open, setOpen]);

  if (!sources) return null;
  const label = found.length ? plural(found.length, 'source', 'sources') : 'No sources';

  return (
    <div
      ref={wrap}
      className="relative mr-auto max-[959px]:mr-0"
      onKeyDown={(e) => {
        if (e.key !== 'Escape' || !open) return;
        // Esc closes the popover only: it shouldn't also close the open session behind it.
        e.stopPropagation();
        setOpen(false);
        trigger.current?.focus();
      }}
    >
      <button
        ref={trigger}
        type="button"
        aria-expanded={open}
        aria-controls={id}
        title="Folders Logbook reads"
        onClick={() => setOpen(!open)}
        className={cx(
          'flex h-7 items-center gap-2 rounded-md pl-2 pr-[10px] text-[12px] max-[959px]:w-7 max-[959px]:justify-center max-[959px]:p-0',
          // The dots' rings take the button's own background, so they stay crisp on every state.
          open
            ? 'bg-side-sel text-ink [--ring:var(--side-sel)]'
            : 'text-mute [--ring:var(--side)] hover:bg-side-hover hover:text-soft hover:[--ring:var(--side-hover)]',
        )}
      >
        <Dots rows={found} />
        <span className={RAIL_HIDE}>{label}</span>
      </button>
      {open && <Popover id={id} rows={rows} />}
    </div>
  );
}

/** The agents' colours, overlapping a little, each ringed in the button's colour so they stay apart. */
function Dots({ rows }: { rows: SourceRow[] }) {
  if (!rows.length) {
    return <span aria-hidden="true" className="h-[9px] w-[9px] rounded-full border-[1.5px] border-mute" />;
  }
  return (
    <span aria-hidden="true" className="flex items-center">
      {rows.slice(0, MAX_DOTS).map((r, i) => (
        <span
          key={r.agent}
          className={cx('h-[9px] w-[9px] rounded-full shadow-[0_0_0_2px_var(--ring)]', i > 0 && '-ml-[3px]')}
          style={{ background: agentInfo(r.agent).color }}
        />
      ))}
    </span>
  );
}

function Popover({ id, rows }: { id: string; rows: SourceRow[] }) {
  const { openFolder, indexing, readAt } = useApp(
    useShallow((s) => ({ openFolder: s.openFolder, indexing: s.status?.indexing ?? false, readAt: s.readAt })),
  );
  const now = useNow();
  return (
    <div
      id={id}
      role="dialog"
      aria-label="Sources"
      className="absolute bottom-[calc(100%+8px)] left-0 z-30 flex max-h-[calc(100vh-72px)] w-[320px] flex-col overflow-hidden rounded-xl border border-line bg-raised text-left shadow-[0_16px_40px_rgb(0_0_0/0.35),0_2px_6px_rgb(0_0_0/0.2)]"
    >
      <div className="flex flex-none flex-col gap-[3px] px-4 pb-[10px] pt-[14px]">
        <h2 className="m-0 text-[13px] font-semibold text-ink">Sources</h2>
        <p className="m-0 text-[12px] leading-[1.45] text-mute">Logbook reads these folders and never changes them.</p>
      </div>
      {/* However many agents there are, the list scrolls and the header and status line stay put. */}
      <ul className="m-0 flex min-h-0 list-none flex-col overflow-y-auto p-0 px-[6px] pb-[6px]">
        {rows.map((r) => (
          <SourceItem key={r.agent} row={r} onOpen={(dir) => void openFolder(dir)} />
        ))}
      </ul>
      <div className="flex flex-none items-center gap-2 border-t border-line px-4 py-[10px] text-[11.5px] text-mute">
        {indexing ? (
          <>
            <Icon name="refresh" size={11} className="flex-none animate-spin text-accent-text" />
            <span>Reading…</span>
          </>
        ) : (
          <>
            <span aria-hidden="true" className="h-[6px] w-[6px] flex-none rounded-full bg-ok" />
            <span>Up to date{readAt !== null && ` · read ${ago(now.getTime() - readAt)}`}</span>
          </>
        )}
      </div>
    </div>
  );
}

function SourceItem({ row, onOpen }: { row: SourceRow; onOpen: (dir: string) => void }) {
  const info = agentInfo(row.agent);
  return (
    <li className="flex items-start gap-[10px] rounded-lg p-[10px] hover:bg-hover">
      <span
        aria-hidden="true"
        className={cx('mt-[6px] h-[7px] w-[7px] flex-none rounded-full', !row.found && 'border-[1.5px] border-mute')}
        style={row.found ? { background: info.color } : undefined}
      />
      <div className="flex min-w-0 flex-1 flex-col gap-[3px]">
        <div className={cx('flex items-baseline gap-2 text-[13px]', row.found ? 'text-ink' : 'text-mute')}>
          <span className="min-w-0 flex-1 truncate">{info.name}</span>
          {row.found ? (
            <span className="whitespace-nowrap font-mono text-[11px] tabular-nums text-mute">
              {row.count ? plural(row.count, 'session', 'sessions') : 'No sessions yet'}
            </span>
          ) : (
            <span className="whitespace-nowrap text-[11.5px]">Not found</span>
          )}
        </div>
        {row.dirs.map((d) => (
          <div key={d.dir} className="flex min-w-0 items-center gap-1">
            {/* A long path loses its start, not its end: the last folders are the ones that tell them apart. */}
            <span
              dir="rtl"
              title={d.dir}
              className={cx(
                'min-w-0 flex-1 truncate text-left font-mono text-[11.5px]',
                d.found ? 'text-soft' : 'text-mute',
              )}
            >
              <bdi dir="ltr">{d.short}</bdi>
            </span>
            {d.found && (
              <button
                type="button"
                title="Open folder"
                aria-label={`Open the ${info.name} folder`}
                onClick={() => onOpen(d.dir)}
                className="grid h-6 w-6 flex-none place-items-center rounded-md text-mute hover:bg-btn hover:text-soft"
              >
                <Icon name="folder" size={13} />
              </button>
            )}
          </div>
        ))}
        {row.kept > 0 && (
          <span className="text-[11.5px] text-mute">
            {row.kept} kept after {info.name} deleted {row.kept === 1 ? 'it' : 'them'}
          </span>
        )}
      </div>
    </li>
  );
}

const MIN = 60_000;

/** "just now", "4 min ago", "2 h ago". */
function ago(ms: number): string {
  if (ms < MIN) return 'just now';
  if (ms < 60 * MIN) return `${Math.floor(ms / MIN)} min ago`;
  return `${Math.floor(ms / (60 * MIN))} h ago`;
}
