import { useMemo } from 'react';
import { presentAgents } from '../model/agents';
import { useApp } from '../state/store';

/** Keys, what they do, and whether the keys are the two ends of a run (`1`–`4`). */
type Shortcut = [keys: string[], what: string, run?: boolean];

const SHORTCUTS: Shortcut[] = [
  [['/'], 'Search names and transcripts'],
  [['↑', '↓'], 'Move through sessions'],
  [['Enter'], 'Resume the selected session'],
  [['1', '4'], 'Pick a time range', true],
  [['r'], 'Re-index sessions'],
  [['Esc'], 'Close the session'],
  [['?'], 'About Logbook'],
];

/** Only listed when there's more than one agent to pick, like the sidebar's Agents group. */
const AGENT_SHORTCUT: Shortcut = [['Alt+1', 'Alt+2'], 'Pick an agent'];

/** The keyboard shortcuts handled in `App.tsx`, as a key / description list. */
export function Shortcuts() {
  const sessions = useApp((s) => s.sessions);
  const multi = useMemo(() => presentAgents(sessions).length > 1, [sessions]);
  const list = multi ? [...SHORTCUTS.slice(0, 4), AGENT_SHORTCUT, ...SHORTCUTS.slice(4)] : SHORTCUTS;
  return (
    <dl className="m-0 grid grid-cols-[auto_1fr] items-center gap-x-4 gap-y-[9px] text-[12px]">
      {list.map(([keys, what, run]) => (
        <div key={what} className="contents">
          <dt className="flex items-center gap-1">
            {keys.map((k, i) => (
              <span key={k} className="flex items-center gap-1">
                {i > 0 && <span className="text-mute">{run ? '–' : ''}</span>}
                <kbd className="min-w-[20px] rounded border border-line bg-raised px-[5px] py-px text-center font-mono text-[11px] text-soft">
                  {k}
                </kbd>
              </span>
            ))}
          </dt>
          <dd className="m-0 text-mute">{what}</dd>
        </div>
      ))}
    </dl>
  );
}
