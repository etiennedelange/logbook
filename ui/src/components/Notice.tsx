import { cx } from '../cx';
import { useApp } from '../state/store';

/** Short-lived confirmation or error, pinned to the bottom of the window. */
export function Notice() {
  const notice = useApp((s) => s.notice);
  return (
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
}
