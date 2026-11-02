import { useEffect, useRef } from 'react';

/**
 * One option in a pane's listbox. Only the `tabbable` row is a Tab stop (roving tabindex); the
 * listbox moves between rows with the arrow keys (see `onListboxKey`). Space selects; Enter selects,
 * or resumes the session when the row is already selected.
 */
export function useListRow<T extends HTMLElement>(
  selected: boolean,
  tabbable: boolean,
  onSelect: () => void,
  onOpen: () => void,
) {
  const ref = useRef<T>(null);
  useEffect(() => {
    if (selected) ref.current?.scrollIntoView?.({ block: 'nearest' });
  }, [selected]);
  return {
    ref,
    role: 'option' as const,
    tabIndex: tabbable ? 0 : -1,
    'aria-selected': selected,
    onClick: onSelect,
    onKeyDown: (e: React.KeyboardEvent) => {
      if (e.key === ' ') {
        e.preventDefault();
        onSelect();
      } else if (e.key === 'Enter') {
        e.preventDefault();
        if (selected) onOpen();
        else onSelect();
      }
    },
  };
}

/** Arrow keys, Home and End move focus and selection between a listbox's options. */
export function onListboxKey(e: React.KeyboardEvent<HTMLElement>) {
  if (e.altKey || e.ctrlKey || e.metaKey) return;
  if (!['ArrowDown', 'ArrowUp', 'Home', 'End'].includes(e.key)) return;
  const opts = [...e.currentTarget.querySelectorAll<HTMLElement>('[role="option"]')];
  if (opts.length === 0) return;
  e.preventDefault();
  const i = opts.indexOf(document.activeElement as HTMLElement);
  const last = opts.length - 1;
  const next =
    e.key === 'Home' ? 0 : e.key === 'End' ? last : e.key === 'ArrowDown' ? Math.min(last, i + 1) : Math.max(0, i - 1);
  if (next === i) return;
  opts[next]!.focus();
  opts[next]!.click();
}
