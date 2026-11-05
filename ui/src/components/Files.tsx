import { useMemo } from 'react';
import { cx } from '../cx';
import { clock, dateTime, highlightWords, matchMode, plural } from '../model/format';
import { agentInfo } from '../model/agents';
import type { Detail } from '../state/store';
import type { FileChange, FileEdit, Hunk, Session } from '../types';
import { Icon } from './Icon';

/** How many files a session changed, as the detail pane's facts say it: "2 created, 4 edited". */
export function filesSummary(s: Session): string | null {
  const parts = [
    s.filesCreated > 0 ? `${s.filesCreated} created` : '',
    s.filesEdited > 0 ? `${s.filesEdited} edited` : '',
  ].filter(Boolean);
  return parts.length > 0 ? parts.join(', ') : null;
}

export const filesCount = (s: Session): number => s.filesCreated + s.filesEdited;

/** Transcript | Files, under the session's facts. Shown only for a session that changed files. */
export function DetailTabs({
  session,
  tab,
  onTab,
}: {
  session: Session;
  tab: Detail['tab'];
  onTab: (tab: Detail['tab']) => void;
}) {
  const n = filesCount(session);
  const item = (key: Detail['tab'], label: string, count?: number) => (
    <button
      type="button"
      aria-current={tab === key ? 'page' : undefined}
      onClick={() => onTab(key)}
      className={cx(
        'flex h-[38px] items-center gap-[6px] border-b-2 px-[10px] text-[12.5px]',
        tab === key ? 'border-accent font-medium text-ink' : 'border-transparent text-mute hover:text-soft',
      )}
    >
      {label}
      {count !== undefined && (
        <span className="rounded-full bg-btn px-[6px] text-[11px] tabular-nums text-soft">{count}</span>
      )}
    </button>
  );
  return (
    <nav aria-label="Session view" className="flex gap-1 border-b border-line px-4">
      {item('transcript', 'Transcript')}
      {item('files', 'Files', n)}
    </nav>
  );
}

/** The Files tab: what the session created and edited, or one file's edits. */
export function FilesView({
  session,
  detail,
  tokens,
  onOpen,
}: {
  session: Session;
  detail: Detail;
  /** The search, so matching paths are marked. */
  tokens: string[];
  onOpen: (path: string | null) => void;
}) {
  if (detail.filesError) {
    return (
      <p role="alert" className="px-5 py-6 text-[12px] text-[var(--c-fail)]">
        Couldn’t read this session’s files: {detail.filesError}
      </p>
    );
  }
  if (!detail.files) return <ListSkeleton />;
  const file = detail.file ? detail.files.files.find((f) => f.path === detail.file) : undefined;
  if (file) return <FileEdits session={session} file={file} detail={detail} onBack={() => onOpen(null)} />;

  const { files, commands } = detail.files;
  const created = files.filter((f) => f.created);
  const edited = files.filter((f) => !f.created);
  const info = agentInfo(session.agent);
  return (
    <div className="mx-auto flex max-w-[800px] flex-col gap-5 px-5 pb-10 pt-4">
      {[
        { label: 'Created', list: created },
        { label: 'Edited', list: edited },
      ].map(
        (g) =>
          g.list.length > 0 && (
            <section key={g.label} aria-label={g.label} className="flex flex-col gap-[2px]">
              <h2 className="m-0 mb-[6px] ml-[10px] text-[11px] font-medium text-mute">{g.label}</h2>
              <ul className="m-0 flex list-none flex-col gap-[2px] p-0">
                {g.list.map((f) => (
                  <li key={f.path}>
                    <FileRow file={f} tokens={tokens} onOpen={() => onOpen(f.path)} />
                  </li>
                ))}
              </ul>
            </section>
          ),
      )}
      {files.length === 0 && (
        <p className="m-0 px-[10px] text-[12.5px] text-mute">
          {info.name} didn’t change any files with its file tools.
        </p>
      )}
      {commands.length > 0 && (
        <section aria-label="Commands that may have changed files" className="flex flex-col gap-[6px]">
          <h2 className="m-0 ml-[10px] text-[11px] font-medium text-mute">Commands that may have changed files</h2>
          <div className="mx-[10px] flex flex-col gap-[6px] rounded-lg border border-line bg-raised px-3 py-[10px]">
            {commands.map((c) => (
              <code key={c} className="select-all break-all font-mono text-[11.5px] text-soft">
                {c}
              </code>
            ))}
            <p className="m-0 mt-1 text-[11.5px] text-mute">
              Logbook sees these commands, but not which files they changed.
            </p>
          </div>
        </section>
      )}
      <p className="m-0 mx-[10px] text-[11.5px] leading-[1.55] text-mute">
        From {info.name}’s edit records. This is what changed during the session, not the files as they are now.
      </p>
    </div>
  );
}

function FileRow({ file: f, tokens, onOpen }: { file: FileChange; tokens: string[]; onOpen: () => void }) {
  const cut = Math.max(f.path.lastIndexOf('/'), f.path.lastIndexOf('\\')) + 1;
  const marks = useMemo(() => pathMarks(f.path, tokens), [f.path, tokens]);
  return (
    <button
      type="button"
      onClick={onOpen}
      title={`Show ${f.path}’s edits`}
      className="flex w-full items-center gap-3 rounded-lg px-[10px] py-2 text-left hover:bg-hover"
    >
      <Icon name="file" size={14} className="flex-none text-mute" />
      <span className="min-w-0 flex-1 truncate font-mono text-[12px]">
        {marks ? (
          <Marked segments={marks} />
        ) : (
          <>
            <span className="text-mute">{f.path.slice(0, cut)}</span>
            <span className="text-ink">{f.path.slice(cut)}</span>
          </>
        )}
      </span>
      <span className="flex-none whitespace-nowrap text-[11px] text-mute">{editCount(f)}</span>
      <Counts added={f.added} removed={f.removed} />
    </button>
  );
}

/** "4 edits"; nothing for a file written once. */
const editCount = (f: FileChange): string => (f.edits > 1 ? plural(f.edits, 'edit', 'edits') : '');

/** The path with the search marked, or null when it doesn't match. */
function pathMarks(path: string, tokens: string[]) {
  const mode = tokens.length > 0 ? matchMode(path, tokens) : null;
  if (!mode) return null;
  const segs = highlightWords(path, tokens, mode);
  return segs.some((s) => s.hit) ? segs : null;
}

function Marked({ segments }: { segments: { text: string; hit: boolean }[] }) {
  return (
    <>
      {segments.map((s, i) =>
        s.hit ? (
          <mark key={i}>{s.text}</mark>
        ) : (
          <span key={i} className="text-soft">
            {s.text}
          </span>
        ),
      )}
    </>
  );
}

function Counts({ added, removed }: { added: number; removed: number }) {
  return (
    <>
      <span className="w-11 flex-none text-right font-mono text-[11.5px] tabular-nums text-ok">
        {added > 0 ? `+${added}` : ''}
        <span className="sr-only"> lines added</span>
      </span>
      <span className="w-11 flex-none text-right font-mono text-[11.5px] tabular-nums text-[var(--c-fail)]">
        {removed > 0 ? `−${removed}` : ''}
        <span className="sr-only"> lines removed</span>
      </span>
    </>
  );
}

/** One file's edits, oldest first, each as a diff. */
function FileEdits({
  session,
  file: f,
  detail,
  onBack,
}: {
  session: Session;
  file: FileChange;
  detail: Detail;
  onBack: () => void;
}) {
  const info = agentInfo(session.agent);
  const edits = detail.edits;
  return (
    <div className="mx-auto flex max-w-[840px] flex-col gap-[14px] px-5 pb-10 pt-[14px]">
      <div className="flex flex-wrap items-center gap-[10px]">
        <button
          type="button"
          onClick={onBack}
          className="flex h-8 items-center gap-1 rounded-lg bg-btn pl-[6px] pr-[10px] text-[12.5px] text-soft hover:bg-btn-hover"
        >
          <Icon name="chevron-left" size={14} />
          Files
        </button>
        <h2 className="m-0 min-w-0 flex-1 break-all font-mono text-[12.5px] font-normal text-ink">{f.path}</h2>
        <Counts added={f.added} removed={f.removed} />
      </div>
      {detail.editsError ? (
        <p role="alert" className="m-0 text-[12px] text-[var(--c-fail)]">
          Couldn’t read the edits: {detail.editsError}
        </p>
      ) : !edits ? (
        <ListSkeleton />
      ) : edits.length === 0 ? (
        <p className="m-0 text-[12.5px] leading-[1.55] text-mute">
          {session.sourceDeletedAt
            ? `${info.name} deleted this session’s file, so its edits can’t be shown. Logbook kept the list of files.`
            : 'No edits to this file were found in the session’s file.'}
        </p>
      ) : (
        edits.map((e, i) => <EditView key={i} edit={e} n={i + 1} of={edits.length} />)
      )}
    </div>
  );
}

function EditView({ edit: e, n, of }: { edit: FileEdit; n: number; of: number }) {
  const label = e.created ? 'Created' : of > 1 ? `Edit ${n} of ${of}` : 'Edit';
  return (
    <article aria-label={label} className="overflow-hidden rounded-[10px] border border-line">
      <header className="flex items-center gap-[10px] border-b border-line bg-raised px-3 py-2">
        <span className="text-[12px] font-medium text-ink">{label}</span>
        {e.timestamp && (
          <time
            dateTime={e.timestamp}
            title={dateTime(e.timestamp)}
            className="font-mono text-[11px] tabular-nums text-mute"
          >
            {clock(e.timestamp)}
          </time>
        )}
      </header>
      <div className="overflow-x-auto py-[6px]">
        {e.hunks.map((h, i) => (
          <HunkView key={i} hunk={h} first={i === 0} />
        ))}
        {e.hunks.length === 0 && <p className="m-0 px-3 py-1 text-[12px] text-mute">An empty file.</p>}
        {e.truncated && <p className="m-0 px-3 pb-1 pt-2 text-[11.5px] text-mute">Long edit: the rest isn’t shown.</p>}
      </div>
    </article>
  );
}

/** A hunk's lines, each numbered in the file it belongs to: removed lines in the old file, the rest in the new. */
function HunkView({ hunk: h, first }: { hunk: Hunk; first: boolean }) {
  let oldNo = h.oldStart;
  let newNo = h.newStart;
  return (
    <div data-hunk="">
      {!first && (
        <div aria-hidden="true" className="px-3 font-mono text-[11px] leading-[1.7] text-mute">
          ⋯
        </div>
      )}
      {h.lines.map((line, i) => {
        const op = line[0] === '+' || line[0] === '-' ? line[0] : ' ';
        const no = op === '-' ? oldNo++ : newNo++;
        if (op === ' ') oldNo++;
        return (
          <div
            key={i}
            data-op={op === '+' ? 'add' : op === '-' ? 'del' : undefined}
            className={cx(
              'grid grid-cols-[44px_18px_minmax(0,1fr)] whitespace-pre font-mono text-[11.5px] leading-[1.7]',
              op === '+' && 'diff-add',
              op === '-' && 'diff-del',
            )}
          >
            <span className="select-none pr-2 text-right tabular-nums text-mute">{no > 0 ? no : ''}</span>
            <span className="diff-op select-none" aria-hidden="true">
              {op === ' ' ? '' : op === '-' ? '−' : '+'}
            </span>
            <span className="text-soft">
              <span className="sr-only">{op === '+' ? 'Added: ' : op === '-' ? 'Removed: ' : ''}</span>
              {line.slice(1)}
            </span>
          </div>
        );
      })}
    </div>
  );
}

function ListSkeleton() {
  return (
    <div aria-hidden="true" className="mx-auto flex max-w-[800px] flex-col gap-3 px-5 pt-5">
      {[0, 1, 2, 3].map((i) => (
        <div key={i} className="skeleton h-[10px]" style={{ width: `${80 - i * 12}%` }} />
      ))}
    </div>
  );
}
