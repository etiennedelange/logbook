import { useShallow } from 'zustand/react/shallow';
import { useApp } from '../state/store';
import { Icon } from './Icon';

/** In the toolbar once GitHub has a newer release; opens What's New. Nothing shows otherwise. */
export function UpdatePill() {
  const { update, updating, setWhatsNew } = useApp(
    useShallow((s) => ({ update: s.update, updating: s.updating, setWhatsNew: s.setWhatsNew })),
  );
  if (!update) return null;
  return (
    <button
      type="button"
      title={updating ? 'Logbook restarts when the update is installed' : `Logbook ${update.version} is available`}
      aria-label={updating ? 'Updating…' : `Update to ${update.version}`}
      aria-haspopup="dialog"
      disabled={updating}
      onClick={() => setWhatsNew({ ...update, mode: 'update' })}
      className="flex h-[26px] flex-none items-center gap-[6px] rounded-full border border-accent/40 bg-accent/12 px-[10px] text-[12px] font-medium text-accent-text hover:bg-accent/20 disabled:cursor-default disabled:opacity-70"
    >
      <Icon name="update" size={13} />
      {/* The session title shares this header; below 1400px the version moves to the tooltip. */}
      <span className="min-[1400px]:hidden">{updating ? 'Updating…' : 'Update'}</span>
      <span className="max-[1399px]:hidden">{updating ? 'Updating…' : `Update to ${update.version}`}</span>
    </button>
  );
}
