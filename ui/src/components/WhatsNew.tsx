import { useEffect, useRef } from 'react';
import { useShallow } from 'zustand/react/shallow';
import { useApp } from '../state/store';
import { Icon } from './Icon';
import { Markdown } from './Markdown';

const NO_TOKENS: string[] = [];

/**
 * A release's notes (its CHANGELOG section) in a modal: before updating, with Update now, or once on the
 * first start after an update. The notes come off the network; react-markdown renders no raw HTML.
 */
export function WhatsNew() {
  const { whatsNew, setWhatsNew, updating, installUpdate, openReleasePage } = useApp(
    useShallow((s) => ({
      whatsNew: s.whatsNew,
      setWhatsNew: s.setWhatsNew,
      updating: s.updating,
      installUpdate: s.installUpdate,
      openReleasePage: s.openReleasePage,
    })),
  );
  const ref = useRef<HTMLDialogElement>(null);
  const open = whatsNew !== null;

  useEffect(() => {
    const d = ref.current;
    if (!d) return;
    if (open && !d.open) {
      d.showModal?.();
      // showModal focuses the first button (Close); start on the main action instead.
      d.querySelector<HTMLElement>('[data-autofocus]')?.focus();
    }
    if (!open && d.open) d.close?.();
  }, [open]);

  // While an update installs the dialog stays put: it says what's happening until Logbook restarts.
  const close = () => {
    if (!updating) setWhatsNew(null);
  };
  const offer = whatsNew?.mode === 'update';

  return (
    <dialog
      ref={ref}
      aria-labelledby="whats-new-title"
      aria-describedby="whats-new-sub"
      onCancel={(e) => updating && e.preventDefault()}
      onClose={() => {
        // A browser may close the dialog on Esc despite onCancel; reopen it while updating.
        if (updating) ref.current?.showModal?.();
        else close();
      }}
      onClick={(e) => e.target === e.currentTarget && close()}
      className="m-auto max-h-[min(640px,calc(100vh-48px))] w-[min(520px,calc(100vw-48px))] overflow-hidden rounded-xl border border-line bg-bg p-0 text-ink shadow-2xl backdrop:bg-black/50 open:flex open:flex-col"
    >
      {whatsNew && (
        <>
          <header className="flex h-[60px] flex-none items-center gap-[10px] border-b border-line pl-5 pr-3">
            <span className="grid h-[26px] w-[26px] flex-none place-items-center rounded-[7px] bg-accent text-accent-ink">
              <Icon name="update" size={15} />
            </span>
            <div className="flex min-w-0 flex-1 flex-col gap-px">
              <h1 id="whats-new-title" className="text-[15px] font-semibold">
                What’s New
              </h1>
              <span id="whats-new-sub" className="text-[12px] text-mute">
                {!offer
                  ? `You’re now on Logbook ${whatsNew.version}`
                  : updating
                    ? `Installing ${whatsNew.version}; Logbook restarts when it’s done`
                    : `Logbook ${whatsNew.version} is available`}
              </span>
            </div>
            <button
              type="button"
              aria-label="Close"
              title="Close (Esc)"
              onClick={close}
              disabled={updating}
              className="grid h-[28px] w-[28px] flex-none place-items-center rounded-md text-mute hover:bg-btn hover:text-soft disabled:pointer-events-none disabled:opacity-40"
            >
              <Icon name="close" size={14} />
            </button>
          </header>

          <div className="relative min-h-0 flex-1 overflow-y-auto px-5 py-5 text-[13px] leading-[1.6] text-soft">
            {whatsNew.notes ? (
              <Markdown text={whatsNew.notes} tokens={NO_TOKENS} mode={null} />
            ) : (
              <p className="m-0">This release has no notes here. The release page lists what changed.</p>
            )}
          </div>

          <footer className="flex flex-none flex-wrap items-center gap-2 border-t border-line px-5 py-3">
            <button
              type="button"
              onClick={() => void openReleasePage(whatsNew.version)}
              className="mr-auto flex items-center gap-[6px] rounded-md px-[6px] py-[5px] text-[12px] text-mute hover:bg-btn hover:text-soft"
            >
              <Icon name="external" size={13} />
              Release page
            </button>
            {offer ? (
              <>
                <button
                  type="button"
                  onClick={close}
                  disabled={updating}
                  className="rounded-md bg-btn px-[11px] py-[5px] text-[12px] text-soft hover:bg-btn-hover hover:text-ink disabled:opacity-60"
                >
                  Later
                </button>
                <button
                  type="button"
                  data-autofocus
                  title="Logbook restarts on the new version once it’s installed"
                  onClick={() => void installUpdate()}
                  disabled={updating}
                  className="flex items-center gap-[6px] rounded-md bg-accent px-[11px] py-[5px] text-[12px] font-semibold text-accent-ink hover:brightness-110 active:brightness-95 disabled:opacity-70"
                >
                  <Icon name="update" size={13} />
                  {updating ? 'Updating…' : 'Update now'}
                </button>
              </>
            ) : (
              <button
                type="button"
                data-autofocus
                onClick={close}
                className="rounded-md bg-accent px-[11px] py-[5px] text-[12px] font-semibold text-accent-ink hover:brightness-110 active:brightness-95"
              >
                Got it
              </button>
            )}
          </footer>
        </>
      )}
    </dialog>
  );
}
