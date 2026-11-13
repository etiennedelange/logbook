import { useEffect, useMemo, useRef, useState, type CSSProperties } from 'react';
import { useShallow } from 'zustand/react/shallow';
import { cx } from '../cx';
import { useNow } from '../hooks/useNow';
import { agentInfo } from '../model/agents';
import { plural, sessionTitle, shortRepo } from '../model/format';
import { timeOf } from '../model/select';
import { MAP_WEEKS, PERIODS, computeStats, facts, greeting, hh, level, type Day, type Stats } from '../model/stats';
import { useApp } from '../state/store';
import type { Session } from '../types';
import { AboutButton } from './About';
import { Icon, type IconName } from './Icon';
import { PaneHeader } from './PaneHeader';
import { UpdatePill } from './UpdatePill';
import { WindowControls } from './WindowControls';

const MONTHS = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'];
const WEEKDAYS = ['Mondays', 'Tuesdays', 'Wednesdays', 'Thursdays', 'Fridays', 'Saturdays', 'Sundays'];
const dayLabel = (d: Date) => d.toLocaleDateString('en-US', { weekday: 'short', month: 'short', day: 'numeric' });
/** A new fact this often, unless the pointer or focus is on the strip. */
const FACT_MS = 7000;
/** How long the "+1" chip for a new session stays. */
const CHIP_MS = 4000;

function reducedMotion(): boolean {
  try {
    return window.matchMedia('(prefers-reduced-motion: reduce)').matches;
  } catch {
    return false;
  }
}

/** Counts up to `target` (from 0 at first, then from wherever it was), easing out. */
function useCountUp(target: number): number {
  const [shown, setShown] = useState(() => (reducedMotion() ? target : 0));
  const from = useRef(shown);
  useEffect(() => {
    if (reducedMotion()) {
      from.current = target;
      setShown(target);
      return;
    }
    const start = performance.now();
    const begin = from.current;
    let raf = 0;
    const tick = (t: number) => {
      const k = Math.min(1, (t - start) / 900);
      const v = Math.round(begin + (target - begin) * (1 - (1 - k) ** 3));
      from.current = v;
      setShown(v);
      if (k < 1) raf = requestAnimationFrame(tick);
    };
    raf = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(raf);
  }, [target]);
  return shown;
}

/**
 * Sessions that appear while Stats is open and were active today: the newest one, shown as a chip
 * in the header, and a count that re-keys today's square so it flashes.
 */
function useArrivals(sessions: Session[], today: Date) {
  const seen = useRef<Set<string> | null>(null);
  const [arrival, setArrival] = useState<{ session: Session | null; n: number }>({ session: null, n: 0 });
  useEffect(() => {
    const ids = new Set(sessions.map((s) => s.id));
    const before = seen.current;
    seen.current = ids;
    // The first list is what was there already; only later ones bring news.
    if (!before || before.size === 0) return;
    const start = today.getTime();
    const fresh = sessions.find((s) => !before.has(s.id) && (timeOf(s) ?? 0) >= start);
    if (fresh) setArrival((a) => ({ session: fresh, n: a.n + 1 }));
  }, [sessions, today]);
  useEffect(() => {
    if (!arrival.session) return;
    const t = setTimeout(() => setArrival((a) => ({ ...a, session: null })), CHIP_MS);
    return () => clearTimeout(t);
  }, [arrival]);
  return arrival;
}

export function StatsPane() {
  const { sessions, period, setPeriod, sources, status, loaded } = useApp(
    useShallow((s) => ({
      sessions: s.sessions,
      period: s.statsPeriod,
      setPeriod: s.setStatsPeriod,
      sources: s.sources,
      status: s.status,
      loaded: s.loaded,
    })),
  );
  const now = useNow();
  const dayKey = now.toDateString();
  const today = useMemo(() => new Date(dayKey), [dayKey]);
  const stats = useMemo(() => computeStats(sessions, period, today), [sessions, period, today]);
  const arrival = useArrivals(sessions, today);
  const days = PERIODS.find((p) => p.key === period)!.days;
  const periodLabel = PERIODS.find((p) => p.key === period)!.label;
  const sub = `${greeting(now)} · ${plural(stats.sessions, 'session', 'sessions')}${
    stats.repoCount ? ` across ${plural(stats.repoCount, 'repository', 'repositories')}` : ''
  }, ${periodLabel.toLowerCase()}`;

  return (
    <section aria-label="Stats" className="col-span-2 flex min-h-0 min-w-0 flex-col bg-bg">
      <PaneHeader title="Stats" className="pl-6 pr-3">
        <UpdatePill />
        <AboutButton />
        <WindowControls />
      </PaneHeader>

      <div className="flex min-h-0 flex-1 flex-col gap-3 overflow-y-auto px-6 pb-6 pt-4">
        <p className="m-0 text-[13px] text-mute">{sub}</p>
        <div className={cx('flex flex-wrap items-center gap-x-4 gap-y-2', sessions.length === 0 && 'hidden')}>
          <div
            role="group"
            aria-label="Period"
            className="flex flex-none gap-px rounded-lg border border-line bg-inset p-[2px]"
          >
            {PERIODS.map((p) => (
              <button
                key={p.key}
                type="button"
                aria-pressed={period === p.key}
                onClick={() => setPeriod(p.key)}
                className={cx(
                  'rounded-md px-[10px] py-[4px] text-[12px] font-medium transition-colors',
                  period === p.key ? 'bg-sel text-ink' : 'text-mute hover:text-soft',
                )}
              >
                {p.label}
              </button>
            ))}
          </div>
          <Live
            sources={sources?.filter((s) => s.found).length ?? null}
            indexing={status?.indexing ?? false}
            arrival={arrival}
          />
        </div>
        {loaded && sessions.length === 0 ? (
          <p className="m-0 py-10 text-center text-[13px] text-mute">
            No sessions yet. Once Copilot CLI or Claude Code has run, your stats show up here.
          </p>
        ) : (
          <>
            <FactStrip stats={stats} />
            <Totals stats={stats} days={days} />
            <ActivityMap stats={stats} days={days} flash={arrival.n} />
            <div className="grid grid-cols-[repeat(auto-fit,minmax(300px,1fr))] gap-3">
              <Hours stats={stats} now={now} />
              <Agents stats={stats} />
            </div>
            <div className="grid grid-cols-[repeat(auto-fit,minmax(300px,1fr))] gap-3">
              <Repos stats={stats} />
              <Highlights stats={stats} />
            </div>
          </>
        )}
      </div>
    </section>
  );
}

/** "Watching 2 sources", with a pulsing dot, and the chip for a session that just arrived. */
function Live(p: { sources: number | null; indexing: boolean; arrival: ReturnType<typeof useArrivals> }) {
  const s = p.arrival.session;
  return (
    <div className="ml-auto flex min-w-0 items-center gap-3">
      {/* A polite status: a screen reader hears each arrival once. */}
      <span role="status" className="flex min-w-0 items-center">
        {s && (
          <span
            key={p.arrival.n}
            className="st-chip inline-flex min-w-0 items-center gap-2 rounded-full border border-line bg-raised py-[3px] pl-[4px] pr-[10px] text-[12px]"
          >
            <span className="rounded-full bg-accent px-[6px] font-mono text-[10px] font-semibold text-accent-ink">
              +1
            </span>
            <span
              className="h-[7px] w-[7px] flex-none rounded-full"
              style={{ background: agentInfo(s.agent).color }}
              aria-hidden="true"
            />
            <span className="max-w-[180px] truncate">
              <span className="sr-only">New session: </span>
              {sessionTitle(s).text}
            </span>
          </span>
        )}
      </span>
      {p.sources !== null && p.sources > 0 && (
        <span className="flex items-center gap-2 whitespace-nowrap text-[12px] text-mute">
          <span className="st-live relative h-[7px] w-[7px] rounded-full bg-ok" aria-hidden="true" />
          {p.indexing ? 'Reading sessions…' : `Watching ${plural(p.sources, 'source', 'sources')}`}
        </span>
      )}
    </div>
  );
}

function Card(p: { title: string; aside?: string; className?: string; children: React.ReactNode }) {
  return (
    <section
      aria-label={p.title}
      className={cx('flex min-w-0 flex-col gap-3 rounded-xl border border-line bg-list px-5 py-4', p.className)}
    >
      <div className="flex flex-wrap items-baseline gap-x-4 gap-y-1">
        <h2 className="m-0 flex-1 text-[13px] font-semibold">{p.title}</h2>
        {p.aside && <span className="text-[12px] text-mute">{p.aside}</span>}
      </div>
      {p.children}
    </section>
  );
}

const Empty = () => <p className="m-0 text-[12px] text-mute">No sessions in this period.</p>;

/** One line at a time, worked out from the stats. Click for the next; it moves on by itself unless you're on it. */
function FactStrip({ stats }: { stats: Stats }) {
  const list = useMemo(() => facts(stats, (a) => agentInfo(a).name), [stats]);
  const [i, setI] = useState(0);
  const [held, setHeld] = useState(false);
  useEffect(() => {
    if (held || list.length < 2 || reducedMotion()) return;
    const t = setInterval(() => setI((n) => n + 1), FACT_MS);
    return () => clearInterval(t);
  }, [held, list.length]);
  if (list.length === 0) return null;
  const at = i % list.length;
  return (
    <button
      type="button"
      onClick={() => setI((n) => n + 1)}
      onMouseEnter={() => setHeld(true)}
      onMouseLeave={() => setHeld(false)}
      onFocus={() => setHeld(true)}
      onBlur={() => setHeld(false)}
      title="Show another"
      className="flex min-h-[40px] flex-none items-center gap-3 rounded-lg border border-line-soft bg-list px-4 py-2 text-left text-[13px] text-soft transition-colors hover:bg-hover"
    >
      <Icon name="sparkle" size={15} className="st-twirl flex-none text-accent-text" />
      <span className="min-w-0 flex-1">
        <span key={i} className="st-fade">
          {list[at]}
        </span>
      </span>
      <span className="flex-none font-mono text-[11px] text-mute" aria-hidden="true">
        {at + 1}/{list.length}
      </span>
    </button>
  );
}

function Totals({ stats, days }: { stats: Stats; days: number }) {
  const sessions = useCountUp(stats.sessions);
  const messages = useCountUp(stats.messages);
  const active = useCountUp(stats.activeDays);
  const streak = useCountUp(stats.streak);
  const tiles = [
    { label: 'Sessions', value: String(sessions), sub: `${(stats.sessions / days).toFixed(1)} a day on average` },
    {
      label: 'Messages',
      value: messages.toLocaleString('en-US'),
      sub: stats.sessions ? `${Math.round(stats.messages / stats.sessions)} per session` : 'None yet',
    },
    { label: 'Active days', value: String(active), sub: `of ${days}` },
  ];
  return (
    <div role="group" aria-label="Totals" className="grid grid-cols-4 gap-3 max-[1099px]:grid-cols-2">
      {tiles.map((t, i) => (
        <div
          key={t.label}
          className="st-rise st-tile flex flex-col gap-[2px] rounded-xl border border-line bg-list px-4 py-3"
          style={{ animationDelay: `${i * 70}ms` }}
        >
          <span className="text-[12px] text-mute">{t.label}</span>
          <span className="text-[22px] font-semibold tabular-nums tracking-[-0.01em]" data-stat={t.label}>
            {t.value}
          </span>
          <span className="text-[11px] text-mute">{t.sub}</span>
        </div>
      ))}
      <StreakTile streak={streak} longest={stats.longestStreak} />
    </div>
  );
}

/** Twelve directions for the confetti, a little upward. */
const SPARKS = Array.from({ length: 12 }, (_, i) => {
  const a = (i * Math.PI) / 6;
  const r = i % 2 ? 36 : 52;
  return {
    '--dx': `${Math.round(Math.cos(a) * r)}px`,
    '--dy': `${Math.round(Math.sin(a) * r) - 10}px`,
    '--rot': `${i % 2 ? -90 : 120}deg`,
    width: i % 3 ? 5 : 8,
    height: i % 3 ? 5 : 8,
    background: ['var(--accent)', 'var(--cat-3)', 'var(--cat-1)', 'var(--cat-4)'][i % 4],
  } as CSSProperties;
});

function StreakTile({ streak, longest }: { streak: number; longest: number }) {
  const [burst, setBurst] = useState(0);
  useEffect(() => {
    if (!burst) return;
    const t = setTimeout(() => setBurst(0), 1000);
    return () => clearTimeout(t);
  }, [burst]);
  return (
    <button
      type="button"
      onClick={() => setBurst((b) => b + 1)}
      title="Celebrate"
      className="st-rise st-tile relative flex flex-col gap-[2px] rounded-xl border border-line bg-list px-4 py-3 text-left"
      style={{ animationDelay: '210ms' }}
    >
      <span className="text-[12px] text-mute">Current streak</span>
      <span className="flex items-center gap-2 text-[22px] font-semibold tabular-nums tracking-[-0.01em]">
        <span className="flex-1" data-stat="Current streak">
          {plural(streak, 'day', 'days')}
        </span>
        <svg
          width="22"
          height="22"
          viewBox="0 0 24 24"
          className="st-flicker flex-none text-accent-text"
          fill="color-mix(in srgb, var(--accent) 25%, transparent)"
          stroke="currentColor"
          strokeWidth="1.8"
          strokeLinejoin="round"
          aria-hidden="true"
        >
          <path d="M12 3c1 3 4 5 4 9a4 4 0 0 1-8 0c0-2 1-3 2-4 0 2 1 3 2 3 0-3-1-5 0-8z" />
        </svg>
      </span>
      <span className="text-[11px] text-mute">Longest in period: {plural(longest, 'day', 'days')}</span>
      {burst > 0 && (
        <span key={burst} aria-hidden="true" className="absolute right-[27px] top-[46px] h-0 w-0">
          {SPARKS.map((style, i) => (
            <span key={i} className="st-spark" style={style} />
          ))}
        </span>
      )}
    </button>
  );
}

function ActivityMap({ stats, days, flash }: { stats: Stats; days: number; flash: number }) {
  const byWeekday = stats.weekdays.indexOf(Math.max(...stats.weekdays));
  const months = stats.weeks.map((w, i) => {
    const first = w.find((d) => d.date.getDate() === 1);
    if (first) return MONTHS[first.date.getMonth()]!;
    // The first column names its month too, unless a new month starts within two weeks.
    if (i === 0 && !stats.weeks.slice(1, 3).some((x) => x.some((d) => d.date.getDate() === 1))) {
      return MONTHS[w[0]!.date.getMonth()]!;
    }
    return '';
  });
  const active = stats.weeks.flat().filter((d) => d.count > 0).length;
  return (
    <Card title="Activity" aside="Each square is a day; the highlighted ones are in the period">
      <div className="flex flex-wrap items-start gap-x-8 gap-y-4">
        <div
          role="img"
          aria-label={`Sessions per day over ${MAP_WEEKS} weeks: sessions on ${plural(active, 'day', 'days')}`}
          className="st-sheen max-w-full overflow-x-auto"
        >
          <div className="flex gap-[3px]">
            <div className="mr-1 flex flex-col gap-[3px] pt-[17px] font-mono text-[11px] leading-[13px] text-mute">
              {['Mon', '', 'Wed', '', 'Fri', '', 'Sun'].map((d, i) => (
                <span key={i} className="h-[13px]">
                  {d}
                </span>
              ))}
            </div>
            {stats.weeks.map((w, wi) => (
              <div key={wi} className="flex flex-col gap-[3px]">
                <span className="h-[14px] w-[13px] overflow-visible whitespace-nowrap text-[11px] text-mute">
                  {months[wi]}
                </span>
                {w.map((d, di) => (
                  <MapSquare
                    key={d.ago === 0 ? `today-${flash}` : di}
                    day={d}
                    inPeriod={d.ago < days}
                    delay={180 + wi * 14 + di * 10}
                    flash={flash}
                  />
                ))}
              </div>
            ))}
          </div>
          <div className="mt-3 flex items-center gap-[3px] text-[11px] text-mute">
            <span className="mr-1">Less</span>
            {[0, 1, 2, 3, 4].map((l) => (
              <span key={l} className="h-[11px] w-[11px] rounded-[3px]" style={{ background: `var(--heat-${l})` }} />
            ))}
            <span className="ml-1">More</span>
          </div>
        </div>
        <dl className="m-0 flex min-w-[160px] flex-1 flex-col gap-3">
          <Fact
            label="Busiest day"
            value={stats.busiestDay ? `${dayLabel(stats.busiestDay.date)} · ${stats.busiestDay.count}` : '–'}
          />
          <Fact label="Favourite day" value={stats.sessions ? WEEKDAYS[byWeekday]! : '–'} />
          <Fact label="Days off" value={plural(days - stats.activeDays, 'day', 'days')} />
        </dl>
      </div>
    </Card>
  );
}

function Fact({ label, value }: { label: string; value: string }) {
  return (
    <div>
      <dt className="text-[11px] text-mute">{label}</dt>
      <dd className="m-0 text-[14px] font-semibold">{value}</dd>
    </div>
  );
}

function MapSquare({ day, inPeriod, delay, flash }: { day: Day; inPeriod: boolean; delay: number; flash: number }) {
  if (day.ago < 0) return <span className="h-[13px] w-[13px]" />;
  const today = day.ago === 0;
  return (
    <span
      title={`${dayLabel(day.date)}: ${plural(day.count, 'session', 'sessions')}`}
      className={cx('h-[13px] w-[13px] rounded-[3px]', today ? (flash ? 'st-flash' : 'st-today') : 'st-pop')}
      style={{
        background: `var(--heat-${level(day.count)})`,
        opacity: inPeriod ? 1 : 0.35,
        animationDelay: today && flash ? undefined : `${delay}ms`,
      }}
    />
  );
}

function Hours({ stats, now }: { stats: Stats; now: Date }) {
  const max = Math.max(1, ...stats.hours);
  const h = now.getHours();
  return (
    <Card
      title="When you start sessions"
      aside={`${stats.peakHour !== null ? `Peak around ${hh(stats.peakHour)} · ` : ''}now ${hh(h)}`}
    >
      <div
        role="img"
        aria-label={
          stats.peakHour !== null
            ? `Sessions by the hour they started, most around ${hh(stats.peakHour)}`
            : 'No sessions in this period'
        }
      >
        <div className="flex h-[110px] items-end gap-[3px] border-b border-line">
          {stats.hours.map((c, i) => (
            <span
              key={i}
              title={`${hh(i)}: ${plural(c, 'session', 'sessions')}`}
              className={cx('st-grow-y flex-1 rounded-t-[3px]', i === h && 'st-now')}
              style={{
                height: `${Math.max(3, Math.round((104 * c) / max))}px`,
                background:
                  i === stats.peakHour ? 'var(--accent)' : 'color-mix(in oklch, var(--accent) 50%, var(--raised))',
                animationDelay: `${300 + i * 22}ms`,
              }}
            />
          ))}
        </div>
        <div className="mt-[5px] flex gap-[3px] font-mono text-[11px] text-mute">
          {stats.hours.map((_, i) => (
            <span key={i} className={cx('flex-1', i === h && 'text-accent-text')}>
              {i === h ? 'now' : i % 6 === 0 && Math.abs(i - h) > 1 ? String(i).padStart(2, '0') : ''}
            </span>
          ))}
        </div>
      </div>
    </Card>
  );
}

function Agents({ stats }: { stats: Stats }) {
  const setAgent = useApp((s) => s.setAgent);
  const total = stats.agents.reduce((n, a) => n + a.count, 0);
  const share = (n: number) => Math.round((n * 100) / Math.max(1, total));
  return (
    <Card title="Agents">
      {total === 0 ? (
        <Empty />
      ) : (
        <>
          <div className="flex h-[12px] gap-[2px] overflow-hidden rounded-full" aria-hidden="true">
            {stats.agents
              .filter((a) => a.count > 0)
              .map((a, i) => (
                <span
                  key={a.agent}
                  className="st-grow-x"
                  style={{
                    width: `${(a.count * 100) / total}%`,
                    background: agentInfo(a.agent).color,
                    animationDelay: `${450 + i * 150}ms`,
                  }}
                />
              ))}
          </div>
          <ul className="m-0 flex list-none flex-col gap-px p-0">
            {stats.agents.map((a) => (
              <li key={a.agent}>
                <button
                  type="button"
                  onClick={() => setAgent(a.agent)}
                  title={`Show ${agentInfo(a.agent).name} sessions`}
                  className="flex w-full items-center gap-[10px] rounded-md px-2 py-[6px] text-left text-[13px] hover:bg-hover"
                >
                  <span
                    className="h-[9px] w-[9px] flex-none rounded-full"
                    style={{ background: agentInfo(a.agent).color }}
                    aria-hidden="true"
                  />
                  <span className="flex-1">{agentInfo(a.agent).name}</span>
                  <span className="font-mono text-[12px] tabular-nums text-soft">{a.count}</span>
                  <span className="w-[40px] text-right font-mono text-[12px] tabular-nums text-mute">
                    {share(a.count)}%
                  </span>
                </button>
              </li>
            ))}
          </ul>
        </>
      )}
    </Card>
  );
}

function Repos({ stats }: { stats: Stats }) {
  const setRepo = useApp((s) => s.setRepo);
  const max = Math.max(1, stats.repos[0]?.count ?? 0);
  return (
    <Card title="Top repositories">
      {stats.repos.length === 0 ? (
        <Empty />
      ) : (
        <ul className="m-0 flex list-none flex-col gap-px p-0">
          {stats.repos.map((r, i) => (
            <li key={r.name}>
              <button
                type="button"
                onClick={() => setRepo(r.name)}
                title={`Show sessions in ${r.name}`}
                className="st-row grid w-full grid-cols-[110px_minmax(0,1fr)_36px] items-center gap-3 rounded-md px-2 py-[6px] text-left text-[13px] hover:bg-hover"
              >
                <span className="truncate text-soft">{shortRepo(r.name)}</span>
                <span className="h-[8px] rounded-full bg-raised">
                  <span
                    className="st-grow-x block h-full rounded-full"
                    style={{
                      width: `${(r.count * 100) / max}%`,
                      background: i === 0 ? 'var(--accent)' : 'color-mix(in oklch, var(--accent) 55%, var(--raised))',
                      animationDelay: `${550 + i * 80}ms`,
                    }}
                  />
                </span>
                <span className="text-right font-mono text-[12px] tabular-nums text-mute">{r.count}</span>
              </button>
            </li>
          ))}
        </ul>
      )}
    </Card>
  );
}

function Highlights({ stats }: { stats: Stats }) {
  const { select, setView } = useApp(useShallow((s) => ({ select: s.select, setView: s.setView })));
  const c = stats.chattiest;
  const late = Math.round((stats.lateNight * 100) / Math.max(1, stats.sessions));
  const rows: { icon: IconName; label: string; value: string; detail: string; open?: () => void }[] = [
    {
      icon: 'flame',
      label: 'Longest streak',
      value: `${plural(stats.longestStreak, 'day', 'days')} in a row`,
      detail: `Current run: ${plural(stats.streak, 'day', 'days')}`,
    },
    ...(c
      ? [
          {
            icon: 'chat' as const,
            label: 'Chattiest session',
            value: sessionTitle(c).text,
            detail: [plural(c.messageCount, 'message', 'messages'), c.repository && shortRepo(c.repository), c.branch]
              .filter(Boolean)
              .join(' · '),
            open: () => {
              setView('sessions');
              select(c.id);
            },
          },
        ]
      : []),
    {
      icon: 'moon',
      label: 'Night owl',
      value: `${late}% after 22:00`,
      detail: `${plural(stats.lateNight, 'session', 'sessions')} started late`,
    },
  ];
  return (
    <Card title="Highlights">
      <ul className="m-0 flex list-none flex-col p-0">
        {rows.map((r, i) => {
          const body = (
            <>
              <span className="st-ico grid h-[32px] w-[32px] flex-none place-items-center rounded-lg bg-[color-mix(in_srgb,var(--accent)_16%,transparent)] text-accent-text">
                <Icon name={r.icon} size={15} />
              </span>
              <span className="flex min-w-0 flex-1 flex-col">
                <span className="text-[11px] text-mute">{r.label}</span>
                <span className="truncate text-[13px] font-semibold">{r.value}</span>
                <span className="truncate text-[12px] text-mute">{r.detail}</span>
              </span>
            </>
          );
          const cls = 'st-hl flex w-full items-start gap-3 rounded-md px-2 py-[8px] text-left';
          return (
            <li
              key={r.label}
              className="st-rise border-t border-line-soft first:border-t-0"
              style={{ animationDelay: `${600 + i * 80}ms` }}
            >
              {r.open ? (
                <button type="button" onClick={r.open} title="Open this session" className={cx(cls, 'hover:bg-hover')}>
                  {body}
                </button>
              ) : (
                <div className={cls}>{body}</div>
              )}
            </li>
          );
        })}
      </ul>
    </Card>
  );
}
