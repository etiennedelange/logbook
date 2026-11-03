import { useEffect, useState } from 'react';
import { createPortal } from 'react-dom';
import { cx } from '../cx';
import { useApp } from '../state/store';

/** The open modal dialog, if any. It sits in the top layer, above anything outside it whatever its z-index. */
const modalDialog = () =>
  [...document.querySelectorAll('dialog')].filter((d) => d.open && d.matches(':modal')).at(-1) ?? null;

/**
 * Short-lived confirmation or error, pinned to the bottom of the window. While a modal dialog (About,
 * What's New) is open, it's rendered inside that dialog, or the dialog would cover it: About's "Check for
 * updates" result used to be hidden that way.
 */
export function Notice() {
  const notice = useApp((s) => s.notice);
  const [dialog, setDialog] = useState<HTMLDialogElement | null>(null);

  useEffect(() => {
    // showModal() and close() set and clear the open attribute, wherever and whenever they're called.
    const follow = () => setDialog(modalDialog());
    follow();
    const watch = new MutationObserver(follow);
    watch.observe(document.body, { subtree: true, attributes: true, attributeFilter: ['open'] });
    return () => watch.disconnect();
  }, []);

  const region = (
    <div aria-live="polite" className="pointer-events-none fixed inset-x-0 bottom-4 z-50 flex justify-center">
      {notice && (
        <p
          role={notice.tone === 'error' ? 'alert' : 'status'}
          className={cx(
            'm-0 rounded-md border bg-raised px-3 py-[7px] text-[12px] shadow-lg',
            notice.tone === 'error' ? 'border-[var(--c-fail)] text-[var(--c-fail)]' : 'border-line text-soft',
          )}
        >
          {notice.text}
        </p>
      )}
    </div>
  );
  return dialog ? createPortal(region, dialog) : region;
}
