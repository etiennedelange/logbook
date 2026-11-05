import { memo, useCallback, useDeferredValue, useEffect, useMemo, useRef, useState, type RefObject } from 'react';
import { useShallow } from 'zustand/react/shallow';
import { cx } from '../cx';
import {
  clock,
  dateOnly,
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
import { agentInfo, presentAgents } from '../model/agents';
import { sessionsById } from '../model/select';
import { useApp } from '../state/store';
import type { Session, TimelineEntry } from '../types';
import { AboutButton } from './About';
import { Icon, type IconName } from './Icon';
import { Markdown } from './Markdown';
import { PaneHeader } from './PaneHeader';
import { UpdatePill } from './UpdatePill';
import { WindowControls } from './WindowControls';
import { AgentLabel } from './ListPane';
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

  const multiAgent = useMemo(() => presentAgents(sessions).length > 1, [sessions]);

  if (!detail || !session) return <NothingSelected />;

  const info = agentInfo(session.agent);
  const deleted = session.sourceDeletedAt !== null;
  const title = sessionTitle(session);
  const repo = repoOf(session);
  const folder = session.cwd ?? session.gitRoot;
  // The branch alone: the repository is already in the list row and the folder below.
  const command = info.resume.replace('<id>', session.nativeId);
  const where = session.branch ?? (repo ? shortRepo(repo) : '');

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
      <div data-transcript-scroll className="relative min-h-0 flex-1 overflow-y-auto">
        <Facts session={session} showAgent={multiAgent} />
        {deleted && <DeletedNote session={session} />}
        <Transcript
          key={session.id}
          entries={detail.transcript}
          error={detail.error}
          searchKey={searchKey}
          speaker={info.speaker}
        />
      </div>
      {/* The resume dock: what Resume will run and where, with the action beside it. */}
      <div className="flex-none px-5 pb-4 pt-2">
        <div className="mx-auto flex max-w-[760px] items-center gap-2 rounded-[16px] border border-line bg-raised py-[7px] pl-4 pr-[7px]">
          <code
            className="min-w-0 flex-1 truncate font-mono text-[11.5px] text-mute"
            title={`${folder ? `${folder} › ` : ''}${command}`}
          >
            {folder && (
              <>
                {folder} <span aria-hidden="true">›</span>{' '}
              </>
            )}
            <span className="text-soft">{command}</span>
          </code>
          <IconAction icon="copy" label="Copy resume command" onClick={() => void copyResume(session.id)} />
          {folder && <IconAction icon="folder" label={`Open ${folder}`} onClick={() => void openFolder(folder)} />}
          <button
            type="button"
            title={
              deleted
                ? `${info.name} deleted this session, so it can’t be resumed`
                : `Open a terminal here running ${info.resume} (Enter on the row)`
            }
            onClick={() => void resume(session.id)}
            disabled={deleted}
            className="flex h-8 flex-none items-center gap-[6px] rounded-full bg-accent px-[14px] text-[12.5px] font-semibold text-accent-ink hover:brightness-110 active:brightness-95 disabled:bg-btn disabled:text-mute disabled:hover:brightness-100"
          >
            <Icon name="terminal" size={13} />
            Resume
          </button>
        </div>
      </div>
    </section>
  );
}

function Facts({ session: s, showAgent }: { session: Session; showAgent: boolean }) {
  const top = [
    { label: 'Last active', value: dateTime(s.updatedAt) },
    { label: 'Started', value: dateTime(s.createdAt) },
    { label: 'Messages', value: String(s.messageCount) },
  ];
  const folder = s.cwd ?? s.gitRoot;
  const paths = [...(folder ? [{ label: 'Folder', value: folder }] : []), { label: 'Session', value: s.nativeId }];

  return (
    <div className="flex flex-col gap-3 border-b border-line px-5 py-4">
      {/* Wraps to a second row in a narrow pane rather than cutting the dates short. */}
      <dl className="m-0 flex flex-wrap gap-x-10 gap-y-3">
        {top.map((f) => (
          <div key={f.label} className="flex min-w-0 flex-col gap-[3px]">
            <dt className="text-[11px] text-mute">{f.label}</dt>
            <dd className="m-0 truncate text-[12.5px] tabular-nums text-soft">{f.value}</dd>
          </div>
        ))}
      </dl>
      <dl className="m-0 grid grid-cols-[auto_minmax(0,1fr)] items-baseline gap-x-4 gap-y-[5px]">
        {showAgent && (
          <div className="contents">
            <dt className="text-[11px] text-mute">Agent</dt>
            <dd className="m-0 flex text-[12px]">
              <AgentLabel agent={s.agent} />
            </dd>
          </div>
        )}
        {paths.map((f) => (
          <div key={f.label} className="contents">
            <dt className="text-[11px] text-mute">{f.label}</dt>
            <dd className="m-0 flex min-w-0 items-center">
              <span className="min-w-0 select-all truncate font-mono text-[11.5px] text-soft" title={f.value}>
                {f.value}
              </span>
            </dd>
          </div>
        ))}
      </dl>
    </div>
  );
}

/** Why a session whose file is gone can still be read but not resumed. */
function DeletedNote({ session: s }: { session: Session }) {
  const info = agentInfo(s.agent);
  const when = s.sourceDeletedAt ? dateOnly(s.sourceDeletedAt) : null;
  return (
    <div
      role="note"
      className="mx-5 mt-4 rounded-md border border-line bg-raised px-3 py-[10px] text-[12.5px] leading-[1.55] text-soft"
    >
      <strong className="font-medium text-ink">
        {info.name} deleted this session’s file{when ? ` by ${when}` : ''}.
      </strong>{' '}
      Logbook kept the messages, so you can still find and read them, but the session can’t be resumed.
      {info.retention && (
        <>
          {' '}
          {info.name} keeps sessions for {info.retention.days} days unless{' '}
          <code className="font-mono text-[11.5px]">{info.retention.setting}</code> in its settings says otherwise.
        </>
      )}
    </div>
  );
}

const Transcript = memo(function Transcript({
  entries,
  error,
  searchKey,
  speaker,
}: {
  entries: TimelineEntry[] | null;
  error: string | null;
  /** How the agent's replies are labelled. */
  speaker: string;
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
  const pinned = usePinnedPrompt(ref, entries);

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
      {pinned !== null && entries[pinned] && (
        <PinnedPrompt entry={entries[pinned]} onJump={() => jumpTo(ref.current, pinned)} />
      )}
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
            <li key={i} data-entry={i} className="flex scroll-mt-20 flex-col gap-[6px]">
              {showDay && (
                <div className="flex items-center gap-3 pb-1 pt-2 text-[11px] text-mute" aria-hidden="true">
                  <span className="h-px flex-1 bg-line-soft" />
                  {day}
                  <span className="h-px flex-1 bg-line-soft" />
                </div>
              )}
              <Message entry={e} tokens={tokens} mode={mode} hasHit={hitAt[i]!} speaker={speaker} />
            </li>
          );
        })}
      </ol>
    </div>
  );
});

/**
 * The index of your latest message that has scrolled up out of view, so it can stay pinned
 * above the replies to it; null while the latest one above is still (partly) on screen.
 */
function usePinnedPrompt(list: RefObject<HTMLOListElement | null>, entries: TimelineEntry[] | null) {
  const [pinned, setPinned] = useState<number | null>(null);
  const update = useCallback(() => {
    const ol = list.current;
    const scroller = ol?.closest<HTMLElement>('[data-transcript-scroll]');
    if (!ol || !scroller) return setPinned(null);
    const top = scroller.getBoundingClientRect().top;
    const prompts = ol.querySelectorAll<HTMLElement>('[data-prompt]');
    // In document order, so a binary search finds the last bubble wholly above the top edge.
    let lo = 0;
    let hi = prompts.length;
    while (lo < hi) {
      const mid = (lo + hi) >> 1;
      if (prompts[mid]!.getBoundingClientRect().bottom <= top) lo = mid + 1;
      else hi = mid;
    }
    const li = lo > 0 ? prompts[lo - 1]!.closest<HTMLElement>('[data-entry]') : null;
    setPinned(li ? Number(li.dataset.entry) : null);
  }, [list]);

  useEffect(() => {
    const scroller = list.current?.closest<HTMLElement>('[data-transcript-scroll]');
    if (!scroller) return;
    let frame = 0;
    const onScroll = () => {
      cancelAnimationFrame(frame);
      frame = requestAnimationFrame(update);
    };
    update();
    scroller.addEventListener('scroll', onScroll, { passive: true });
    window.addEventListener('resize', onScroll);
    return () => {
      cancelAnimationFrame(frame);
      scroller.removeEventListener('scroll', onScroll);
      window.removeEventListener('resize', onScroll);
    };
  }, [list, entries, update]);

  return pinned;
}

function jumpTo(list: HTMLOListElement | null, index: number) {
  const still = window.matchMedia?.('(prefers-reduced-motion: reduce)').matches;
  list
    ?.querySelector(`[data-entry="${index}"]`)
    ?.scrollIntoView?.({ block: 'start', behavior: still ? 'auto' : 'smooth' });
}

/** Your last message above the fold, held at the top of the pane while you read the replies. */
function PinnedPrompt({ entry: e, onJump }: { entry: TimelineEntry; onJump: () => void }) {
  return (
    // A zero-height sticky anchor: the bar floats over the transcript instead of pushing it down.
    <div className="sticky top-0 z-10 h-0">
      <div className="-mx-5 -mt-4 border-b border-line-soft bg-bg/95 px-5 py-2 backdrop-blur-sm">
        <button
          type="button"
          onClick={onJump}
          title="Your last message above: jump back to it"
          aria-label={`Your last message: ${e.text.slice(0, 120)}. Jump to it`}
          className="mx-auto flex w-full max-w-[760px] justify-end"
        >
          <span className="line-clamp-2 max-w-[85%] whitespace-pre-line break-words rounded-[14px] bg-sel px-3 py-[6px] text-left text-[12.5px] leading-[1.5] text-ink hover:brightness-110">
            {e.text}
          </span>
        </button>
      </div>
    </div>
  );
}

const messageMatches = (text: string, tokens: string[], mode: MatchMode | null): boolean =>
  tokens.length > 0 && highlightWords(text, tokens, mode).some((s) => s.hit);

/** Memoized: only messages whose search marks change are rendered again. */
const Message = memo(function Message({
  entry: e,
  tokens,
  mode,
  hasHit,
  speaker,
}: {
  speaker: string;
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
      {/* Your messages are bubbles on the right, the agent's plain text on the left, so "You" goes unsaid. */}
      <div className={cx('flex items-baseline gap-2 text-[11px]', isUser && 'justify-end')}>
        <span className={cx('font-semibold text-soft', isUser && 'sr-only')}>{isUser ? 'You' : speaker}</span>
        {e.timestamp && (
          <time dateTime={e.timestamp} className="font-mono tabular-nums text-mute" title={dateTime(e.timestamp)}>
            {clock(e.timestamp)}
          </time>
        )}
      </div>
      <div
        data-prompt={isUser ? '' : undefined}
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
            Search across every Copilot CLI and Claude Code conversation, then resume it right where you left off.
          </p>
          <Shortcuts />
        </div>
      </div>
    </section>
  );
}
