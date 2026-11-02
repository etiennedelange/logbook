import { cx } from '../cx';

/** Filter input: Esc or × clears it; the `searchable` one is what `/` focuses from anywhere. */
export function SearchBox({
  value,
  onChange,
  label,
  placeholder,
  className,
  searchable = true,
  autoFocus,
}: {
  value: string;
  onChange: (v: string) => void;
  label: string;
  placeholder: string;
  className?: string;
  searchable?: boolean;
  autoFocus?: boolean;
}) {
  return (
    <div className={cx('relative min-w-0', className ?? 'w-full')}>
      <input
        type="search"
        data-search={searchable || undefined}
        autoFocus={autoFocus}
        value={value}
        onChange={(e) => onChange(e.target.value)}
        onKeyDown={(e) => {
          if (e.key !== 'Escape') return;
          e.stopPropagation();
          if (value) onChange('');
          else e.currentTarget.blur();
        }}
        aria-label={label}
        placeholder={placeholder}
        title={searchable ? `${label} (/)` : label}
        spellCheck={false}
        autoComplete="off"
        className="w-full rounded-md border border-line bg-transparent py-[4px] pl-[9px] pr-[24px] text-[12px] text-ink outline-none placeholder:text-mute focus:border-accent [&::-webkit-search-cancel-button]:hidden"
      />
      {value && (
        <button
          type="button"
          aria-label="Clear filter"
          onClick={() => onChange('')}
          className="absolute right-[3px] top-1/2 grid h-5 w-5 -translate-y-1/2 place-items-center rounded text-mute hover:bg-btn hover:text-soft"
        >
          ×
        </button>
      )}
    </div>
  );
}
