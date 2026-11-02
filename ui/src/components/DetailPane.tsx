import { memo, useDeferredValue, useEffect, useMemo, useRef, useState } from 'react';
import { useShallow } from 'zustand/react/shallow';
import { cx } from '../cx';
import {
  clock,
  dateTime,
  dayGroup,
  highlightWords,
  matchMode,
  plural,
  queryTokens,
  repoOf,
  sessionTitle,
  shortRepo,
  type MatchMode,
} from '../model/format';
import { sessionsById } from '../model/select';
import { useApp } from '../state/store';
import type { Session, TimelineEntry } from '../types';
import { AboutButton } from './About';
import { Icon, type IconName } from './Icon';
import { Markdown } from './Markdown';
import { PaneHeader } from './PaneHeader';
import { UpdatePill } from './UpdatePill';
import { WindowControls } from './WindowControls';
import { Shortcuts } from './Shortcuts';

/** Messages longer than this start folded, unless they contain a search match. */
const FOLD_CHARS = 1600;

export function DetailPane() {
  const { sessions, hits, detail, hitsQuery, resume, copyResume, openFolder } = useApp(
    useShallow((s) => ({
      sessions: s.sessions,
      hits: s.hits,
      detail: s.detail,
      hitsQuery: s.hitsQuery,
      resume: s.resume,
      copyResume: s.copyResume,
      openFolder: s.openFolder,
    })),
  );
  const id = detail?.id;
  const session = id ? sessionsById(sessions).get(id) : undefined;
  // Highlights follow the search results on screen; re-marking a long transcript waits
  // behind typing and list updates instead of holding them up.
  const searchKey = useDeferredValue(hits ? queryTokens(hitsQuery).join(' ') : '');

  if (!detail || !session) return <NothingSelected />;

  const title = sessionTitle(session);
  const repo = repoOf(session);
  const folder = session.cwd ?? session.gitRoot;
  const where = [repo && shortRepo(repo), session.branch].filter(Boolean).join(' · ');

  return (
    <section aria-label="Session" data-pane="detail" className="flex min-h-0 min-w-0 flex-col">
      <PaneHeader title={title.text} className="pl-5 pr-3">
        {where && (
          <span
            data-tauri-drag-region
            title={[repo, session.branch].filter(Boolean).join(' · ')}
            className="max-w-[40%] flex-none truncate rounded-full bg-raised px-[10px] py-[3px] font-mono text-[11px] text-soft"
          >
            {where}
          </span>
        )}
        <UpdatePill />
        <AboutButton />
        <WindowControls />
      </PaneHeader>
      <div className="min-h-0 flex-1 overflow-y-auto">
        <Facts session={session} />
        <Transcript key={session.id} entries={detail.transcript} error={detail.error} searchKey={searchKey} />
      </div>
      {/* The resume dock: what Resume will run and where, with the action beside it. */}
      <div className="flex-none px-5 pb-4 pt-2">
        <div className="mx-auto flex max-w-[760px] items-center gap-2 rounded-[16px] border border-line bg-raised py-[7px] pl-4 pr-[7px]">
          <code
            className="min-w-0 flex-1 truncate font-mono text-[11.5px] text-mute"
            title={`${folder ? `${folder} › ` : ''}copilot --resume=${session.id}`}
          >
            {folder && (
              <>
                {folder} <span aria-hidden="true">›</span>{' '}
              </>
            )}
            <span className="text-soft">copilot --resume={session.id}</span>
          </code>
          <IconAction icon="copy" label="Copy resume command" onClick={() => void copyResume(session.id)} />
          {folder && <IconAction icon="folder" label={`Open ${folder}`} onClick={() => void openFolder(folder)} />}
          <button
            type="button"
            title="Open a terminal here running copilot --resume (Enter on the row)"
            onClick={() => void resume(session.id)}
            className="flex h-8 flex-none items-center gap-[6px] rounded-full bg-accent px-[14px] text-[12.5px] font-semibold text-accent-ink hover:brightness-110 active:brightness-95"
          >
            <Icon name="terminal" size={13} />
            Resume
          </button>
        </div>
      </div>
    </section>
  );
}

function Facts({ session: s }: { session: Session }) {
  const top = [
    { label: 'Last active', value: dateTime(s.updatedAt) },
    { label: 'Started', value: dateTime(s.createdAt) },
    { label: 'Messages', value: String(s.messageCount) },
  ];
  const paths = [...(s.cwd ? [{ label: 'Folder', value: s.cwd }] : []), { label: 'Session', value: s.id }];

  return (
    <div className="flex flex-col gap-3 border-b border-line px-5 py-4">
      <dl className="m-0 grid grid-cols-3 gap-x-6">
        {top.map((f) => (
          <div key={f.label} className="flex min-w-0 flex-col gap-[3px]">
            <dt className="text-[11px] text-mute">{f.label}</dt>
            <dd className="m-0 truncate text-[12.5px] tabular-nums text-soft">{f.value}</dd>
          </div>
        ))}
      </dl>
      <dl className="m-0 grid grid-cols-[auto_minmax(0,1fr)] items-baseline gap-x-4 gap-y-[5px]">
        {paths.map((f) => (
          <div key={f.label} className="contents">
            <dt className="text-[11px] text-mute">{f.label}</dt>
            <dd className="m-0 select-all truncate font-mono text-[11.5px] text-soft" title={f.value}>
              {f.value}
            </dd>
          </div>
        ))}
      </dl>
    </div>
  );
}

const Transcript = memo(function Transcript({
  entries,
  error,
  searchKey,
}: {
  entries: TimelineEntry[] | null;
  error: string | null;
  /** The search tokens joined by spaces: a string, so it stays equal between renders. */
  searchKey: string;
}) {
  const ref = useRef<HTMLOListElement>(null);
  const tokens = useMemo(() => (searchKey ? searchKey.split(' ') : []), [searchKey]);
  // One mode for the whole conversation, so a phrase search marks the phrase, not every "what".
  const mode = useMemo(
    () => (entries && tokens.length ? matchMode(entries.map((e) => e.text).join('\n'), tokens) : null),
    [entries, tokens],
  );
  const hitAt = useMemo(
    () => (entries ?? []).map((e) => messageMatches(e.text, tokens, mode)),
    [entries, tokens, mode],
  );
  const matches = hitAt.filter(Boolean).length;

  // Land on the first search match rather than the top of a long conversation.
  useEffect(() => {
    if (!entries || !searchKey) return;
    ref.current?.querySelector('mark')?.scrollIntoView?.({ block: 'center' });
  }, [entries, searchKey]);

  if (error) {
    return (
      <p role="alert" className="px-5 py-6 text-[12px] text-[var(--c-fail)]">
        Couldn’t read this transcript: {error}
      </p>
    );
  }
  if (!entries) return <TranscriptSkeleton />;
  if (entries.length === 0) {
    return <p className="px-5 py-8 text-center text-[12px] text-mute">This session has no messages yet.</p>;
  }

  let lastDay = '';
  const now = new Date();
  return (
    <div className="px-5 pb-10 pt-4">
      {tokens.length > 0 && (
        <p className="mx-auto mb-3 max-w-[760px] text-[11px] text-mute">
          {matches > 0
            ? `${plural(matches, 'message matches', 'messages match')} your search`
            : 'No messages match your search; the match is in the session’s name or details.'}
        </p>
      )}
      <ol
        ref={ref}
        aria-label={`Transcript, ${plural(entries.length, 'message', 'messages')}`}
        className="mx-auto m-0 flex max-w-[760px] list-none flex-col gap-5 p-0"
      >
        {entries.map((e, i) => {
          const day = e.timestamp ? dayGroup(e.timestamp, now) : '';
          const showDay = day !== '' && day !== lastDay && i > 0;
          lastDay = day || lastDay;
          return (
            <li key={i} className="flex flex-col gap-[6px]">
              {showDay && (
                <div className="flex items-center gap-3 pb-1 pt-2 text-[11px] text-mute" aria-hidden="true">
                  <span className="h-px flex-1 bg-line-soft" />
                  {day}
                  <span className="h-px flex-1 bg-line-soft" />
                </div>
              )}
              <Message entry={e} tokens={tokens} mode={mode} hasHit={hitAt[i]!} />
            </li>
          );
        })}
      </ol>
    </div>
  );
});

const messageMatches = (text: string, tokens: string[], mode: MatchMode | null): boolean =>
  tokens.length > 0 && highlightWords(text, tokens, mode).some((s) => s.hit);

/** Memoized: only messages whose search marks change are rendered again. */
const Message = memo(function Message({
  entry: e,
  tokens,
  mode,
  hasHit,
}: {
  entry: TimelineEntry;
  tokens: string[];
  mode: MatchMode | null;
  hasHit: boolean;
}) {
  const isUser = e.role === 'user';
  // Follows the search (a match unfolds it) until the user folds or unfolds it themselves.
  const [override, setOverride] = useState<boolean | null>(null);
  const open = override ?? (e.text.length <= FOLD_CHARS || hasHit);
  const shown = open ? e.text : `${e.text.slice(0, FOLD_CHARS).replace(/\s+\S*$/, '')}…`;

  return (
    <>
      {/* Your messages are bubbles on the right, Copilot's plain text on the left, so "You" goes unsaid. */}
      <div className={cx('flex items-baseline gap-2 text-[11px]', isUser && 'justify-end')}>
        <span className={cx('font-semibold text-soft', isUser && 'sr-only')}>{isUser ? 'You' : 'Copilot'}</span>
        {e.timestamp && (
          <time dateTime={e.timestamp} className="font-mono tabular-nums text-mute" title={dateTime(e.timestamp)}>
            {clock(e.timestamp)}
          </time>
        )}
      </div>
      <div
        className={cx(
          'min-w-0 break-words text-[13.5px] leading-[1.6]',
          isUser ? 'max-w-[85%] self-end rounded-[18px] bg-sel px-4 py-[10px] text-ink' : 'text-soft',
        )}
      >
        {/* Messages without a match render unmarked, so a new search leaves them alone. */}
        <Markdown text={shown} tokens={hasHit ? tokens : NO_TOKENS} mode={hasHit ? mode : null} />
      </div>
      {e.text.length > FOLD_CHARS && (
        <button
          type="button"
          onClick={() => setOverride(!open)}
          className={cx(
            'rounded-md px-[6px] py-[2px] text-[12px] font-medium text-accent-text hover:bg-hover',
            isUser ? 'self-end' : 'self-start',
          )}
        >
          {open ? 'Show less' : `Show full message (${Math.round(e.text.length / 1000)}k characters)`}
        </button>
      )}
    </>
  );
});

const NO_TOKENS: string[] = [];

function TranscriptSkeleton() {
  return (
    <div aria-hidden="true" className="mx-auto flex max-w-[760px] flex-col gap-6 px-5 pt-5">
      {[0, 1, 2].map((i) => (
        <div key={i} className="flex flex-col gap-[8px]">
          <div className="skeleton h-[8px] w-[60px]" />
          <div className="skeleton h-[10px] w-[92%]" />
          <div className="skeleton h-[10px] w-[78%]" />
          <div className="skeleton h-[10px] w-[44%]" />
        </div>
      ))}
    </div>
  );
}

function IconAction(p: { icon: IconName; label: string; onClick: () => void }) {
  return (
    <button
      type="button"
      title={p.label}
      aria-label={p.label}
      onClick={p.onClick}
      className="grid h-8 w-8 flex-none place-items-center rounded-full border border-line text-soft hover:bg-btn-hover hover:text-ink"
    >
      <Icon name={p.icon} size={14} />
    </button>
  );
}

function NothingSelected() {
  return (
    <section aria-label="Session" data-pane="detail" className="flex min-h-0 min-w-0 flex-col">
      <PaneHeader title="No session selected" sub="Pick one to read its transcript" className="px-5">
        <UpdatePill />
        <AboutButton />
        <WindowControls />
      </PaneHeader>
      <div className="grid flex-1 place-items-center px-8">
        <div className="flex w-full max-w-[300px] flex-col gap-4">
          <p className="m-0 text-[13px] leading-[1.55] text-soft">
            Search across every Copilot CLI conversation, then resume it right where you left off.
          </p>
          <Shortcuts />
        </div>
      </div>
    </section>
  );
}
