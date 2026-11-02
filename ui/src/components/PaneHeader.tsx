import { cx } from '../cx';

/** Pane top bar. Every pane uses the same height so their bottom borders line up across the window. */
export const HEADER_H = 'h-[60px]';

export function PaneHeader({
  title,
  sub,
  subTitle,
  className,
  children,
}: {
  title: string;
  /** A second line under the title; without it the title sits alone on one line. */
  sub?: string;
  /** Tooltip for the subtitle, when it is a shortened form of something longer. */
  subTitle?: string;
  className?: string;
  children?: React.ReactNode;
}) {
  return (
    // The header doubles as the title bar: drag it to move the window, double-click to maximise.
    <header
      data-tauri-drag-region
      className={cx('flex flex-none items-center gap-[10px] border-b border-line', HEADER_H, className ?? 'px-4')}
    >
      <div data-tauri-drag-region className="flex min-w-0 flex-1 flex-col gap-px">
        <h1 data-tauri-drag-region className="truncate text-[15px] font-semibold" title={title}>
          {title}
        </h1>
        {sub && (
          <span data-tauri-drag-region className="truncate text-[12px] text-mute" title={subTitle}>
            {sub}
          </span>
        )}
      </div>
      {children}
    </header>
  );
}
