import { useEffect, useRef, useState } from 'react';
import { useShallow } from 'zustand/react/shallow';
import { api } from '../api';
import { AGENTS, agentInfo } from '../model/agents';
import { useApp } from '../state/store';
import type { AppInfo } from '../types';
import { Icon } from './Icon';
import { Shortcuts } from './Shortcuts';

/** The `?` button in the detail pane's header. */
export function AboutButton() {
  const setAbout = useApp((s) => s.setAbout);
  return (
    <button
      type="button"
      title="About Logbook (?)"
      aria-label="About Logbook"
      aria-keyshortcuts="?"
      aria-haspopup="dialog"
      onClick={() => setAbout(true)}
      className="grid h-[28px] w-[28px] flex-none place-items-center rounded-md text-mute hover:bg-btn hover:text-soft"
    >
      <Icon name="help" size={15} />
    </button>
  );
}

/** What Logbook is and how it works, in a modal over the window. */
export function About() {
  const { open, setAbout, openFolder, checkUpdate } = useApp(
    useShallow((s) => ({ open: s.about, setAbout: s.setAbout, openFolder: s.openFolder, checkUpdate: s.checkUpdate })),
  );
  const [checking, setChecking] = useState(false);
  const ref = useRef<HTMLDialogElement>(null);
  const [info, setInfo] = useState<AppInfo | null>(null);

  useEffect(() => {
    const d = ref.current;
    if (!d) return;
    if (open && !d.open) d.showModal?.();
    if (!open && d.open) d.close?.();
  }, [open]);

  useEffect(() => {
    if (!open || info) return;
    api.appInfo().then(setInfo, () => undefined);
  }, [open, info]);

  return (
    <dialog
      ref={ref}
      aria-labelledby="about-title"
      onClose={() => setAbout(false)}
      // A click on the dimmed backdrop lands on the dialog itself, outside the panel.
      onClick={(e) => e.target === e.currentTarget && setAbout(false)}
      className="m-auto max-h-[min(720px,calc(100vh-48px))] w-[min(540px,calc(100vw-48px))] overflow-hidden rounded-xl border border-line bg-bg p-0 text-ink shadow-2xl backdrop:bg-black/50 open:flex open:flex-col"
    >
      <header className="flex h-[48px] flex-none items-center gap-[10px] border-b border-line pl-5 pr-3">
        <span className="grid h-[26px] w-[26px] flex-none place-items-center rounded-[7px] bg-accent text-accent-ink">
          <Icon name="logbook" size={15} />
        </span>
        <div className="flex min-w-0 flex-1 flex-col gap-px">
          <h1 id="about-title" className="text-[15px] font-semibold">
            About Logbook
          </h1>
          <span className="flex items-center gap-2 text-[12px] text-mute">
            {info ? `Version ${info.version}` : 'Your coding-agent sessions'}
            {info && (
              <button
                type="button"
                disabled={checking}
                onClick={() => {
                  setChecking(true);
                  void checkUpdate(true).finally(() => setChecking(false));
                }}
                className="rounded px-1 text-accent-text underline-offset-2 hover:underline disabled:text-mute disabled:no-underline"
              >
                {checking ? 'Checking…' : 'Check for updates'}
              </button>
            )}
          </span>
        </div>
        <button
          type="button"
          aria-label="Close"
          title="Close (Esc)"
          onClick={() => setAbout(false)}
          className="grid h-[28px] w-[28px] flex-none place-items-center rounded-md text-mute hover:bg-btn hover:text-soft"
        >
          <Icon name="close" size={14} />
        </button>
      </header>

      <div className="relative min-h-0 flex-1 overflow-y-auto px-5 pb-6 pt-5">
        <p className="m-0 text-[13.5px] leading-[1.6] text-ink">
          Logbook keeps a searchable record of your coding-agent sessions from the GitHub Copilot CLI and Claude Code,
          so you can find an old conversation and pick it up right where you left off.
        </p>

        <Section title="Where sessions come from">
          <p className="m-0">
            Each agent saves every session on your machine: the folder and repository you were in, and the whole
            conversation. Logbook reads those folders. It never changes or deletes anything in them, and nothing leaves
            your machine.
          </p>
          {(info?.sources ?? AGENTS.map((a) => ({ agent: a.key, dir: a.home }))).map((s) => (
            <Path
              key={s.agent}
              label={agentInfo(s.agent).name}
              value={s.dir}
              onOpen={info ? () => void openFolder(s.dir) : undefined}
            />
          ))}
          <p className="m-0">
            Claude Code deletes sessions after 30 days (its <Code>cleanupPeriodDays</Code> setting). Logbook keeps its
            own copy of each conversation, so a deleted session stays searchable and readable here, marked “source
            deleted”. It just can’t be resumed.
          </p>
        </Section>

        <Section title="Staying up to date">
          <p className="m-0">
            To open instantly, Logbook keeps a small index of its own. Whenever you come back to the window, or press{' '}
            <Key>r</Key>, it looks for new and changed sessions and reads only those. The index is safe to delete:
            Logbook rebuilds it from your sessions.
          </p>
          <Path
            label="Index"
            value={info?.cacheFile ?? 'Logbook/index-v6.bin in your local app data folder'}
            onOpen={info ? () => void openFolder(parentOf(info.cacheFile)) : undefined}
          />
        </Section>

        <Section title="Searching">
          <p className="m-0">
            Search looks through session names, repositories and full transcripts. A session has to contain every word
            you type, and words match from their start, so <Code>flak</Code> finds “flaky”. Matches are highlighted in
            the list and in the transcript. Narrow things down further with a time range, an agent or a repository from
            the sidebar.
          </p>
        </Section>

        <Section title="Resuming">
          <p className="m-0">
            <strong className="font-semibold text-ink">Resume</strong> opens Windows Terminal in the session’s folder
            running the agent’s own resume command (<Code>copilot --resume=&lt;id&gt;</Code> or{' '}
            <Code>claude --resume &lt;id&gt;</Code>), so the conversation carries on with all its context.{' '}
            <strong className="font-semibold text-ink">Copy</strong> puts that command on your clipboard instead.
          </p>
        </Section>

        <Section title="Keyboard">
          <Shortcuts />
        </Section>
      </div>
    </dialog>
  );
}

function Section(p: { title: string; children: React.ReactNode }) {
  return (
    <section className="mt-6 flex flex-col gap-[10px] text-[13px] leading-[1.6] text-soft">
      <h2 className="m-0 text-[12px] font-medium text-mute">{p.title}</h2>
      {p.children}
    </section>
  );
}

function Path(p: { label: string; value: string; onOpen?: () => void }) {
  return (
    <div className="flex items-center gap-3 rounded-md border border-line-soft bg-inset py-[6px] pl-3 pr-[6px]">
      <span className="flex-none text-[11px] text-mute">{p.label}</span>
      <span className="min-w-0 flex-1 select-all truncate font-mono text-[11.5px] text-soft" title={p.value}>
        {p.value}
      </span>
      {p.onOpen && (
        <button
          type="button"
          title="Open folder"
          aria-label={`Open the ${p.label.toLowerCase()} folder`}
          onClick={p.onOpen}
          className="grid h-6 w-6 flex-none place-items-center rounded-md text-mute hover:bg-btn hover:text-soft"
        >
          <Icon name="folder" size={13} />
        </button>
      )}
    </div>
  );
}

const Key = ({ children }: { children: React.ReactNode }) => (
  <kbd className="rounded border border-line bg-raised px-[5px] py-px font-mono text-[11px] text-soft">{children}</kbd>
);

const Code = ({ children }: { children: React.ReactNode }) => (
  <code className="rounded bg-raised px-[5px] py-px font-mono text-[0.89em] text-ink">{children}</code>
);

/** The folder a file is in, for either path separator. */
const parentOf = (path: string): string => path.replace(/[\\/][^\\/]*$/, '');
