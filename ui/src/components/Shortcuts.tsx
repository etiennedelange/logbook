const SHORTCUTS: [string[], string][] = [
  [['/'], 'Search names and transcripts'],
  [['↑', '↓'], 'Move through sessions'],
  [['Enter'], 'Resume the selected session'],
  [['1', '4'], 'Pick a time range'],
  [['r'], 'Re-index sessions'],
  [['Esc'], 'Close the session'],
  [['?'], 'About Logbook'],
];

/** The single-key shortcuts handled in `App.tsx`, as a key / description list. */
export function Shortcuts() {
  return (
    <dl className="m-0 grid grid-cols-[auto_1fr] items-center gap-x-4 gap-y-[9px] text-[12px]">
      {SHORTCUTS.map(([keys, what]) => (
        <div key={what} className="contents">
          <dt className="flex items-center gap-1">
            {keys.map((k, i) => (
              <span key={k} className="flex items-center gap-1">
                {i > 0 && <span className="text-mute">{keys.length === 2 && k === '4' ? '–' : ''}</span>}
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
