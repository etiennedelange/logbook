import type { Session } from '../types';

const MIN = 60_000;
const HOUR = 60 * MIN;
const DAY = 24 * HOUR;

/** List time: "now", "2m", "1h", the weekday within a week, otherwise "Oct 7". */
export function relTime(iso: string | null, now: Date): string {
  if (!iso) return '';
  const d = new Date(iso);
  const diff = now.getTime() - d.getTime();
  if (diff < MIN) return 'now';
  if (diff < HOUR) return `${Math.floor(diff / MIN)}m`;
  if (diff < DAY) return `${Math.floor(diff / HOUR)}h`;
  if (diff < 7 * DAY) return d.toLocaleDateString('en-US', { weekday: 'short' });
  const sameYear = d.getFullYear() === now.getFullYear();
  return d.toLocaleDateString('en-US', { month: 'short', day: 'numeric', ...(sameYear ? {} : { year: 'numeric' }) });
}

/** Full timestamp: "Oct 7, 2026, 14:55". */
/** A day: "Oct 28, 2026". */
export const dateOnly = (iso: string): string =>
  new Date(iso).toLocaleDateString('en-US', { month: 'short', day: 'numeric', year: 'numeric' });

export function dateTime(iso: string | null): string {
  if (!iso) return '—';
  const d = new Date(iso);
  const date = dateOnly(iso);
  const time = d.toLocaleTimeString('en-GB', { hour: '2-digit', minute: '2-digit' });
  return `${date}, ${time}`;
}

/** Message time inside a transcript: "14:55". */
export function clock(iso: string | null): string {
  if (!iso) return '';
  return new Date(iso).toLocaleTimeString('en-GB', { hour: '2-digit', minute: '2-digit' });
}

export const basename = (path: string): string =>
  path
    .replace(/[\\/]+$/, '')
    .split(/[\\/]/)
    .pop() ?? path;

export type TitleSource = 'name' | 'prompt' | 'none';

/** What a session is called: its name, else what it was first asked, else "Untitled session". */
export function sessionTitle(s: Session): { text: string; source: TitleSource } {
  if (s.name?.trim()) return { text: s.name.trim(), source: 'name' };
  const prompt = s.firstPrompt
    ?.replace(/`+|\*\*|^[#>\s]+/g, '')
    .replace(/\s+/g, ' ')
    .trim();
  if (prompt) return { text: prompt, source: 'prompt' };
  return { text: 'Untitled session', source: 'none' };
}

export const displayName = (s: Session): string => sessionTitle(s).text;

/** Grouping key for the sidebar: the repository, else the git root or working folder's name. */
export function repoOf(s: Session): string | null {
  if (s.repository?.trim()) return s.repository;
  const dir = s.gitRoot ?? s.cwd;
  return dir ? basename(dir) : null;
}

/** "Org/Project/Repo" shows as "Repo"; the full name goes in the tooltip. */
export const shortRepo = (repo: string): string => repo.split('/').pop() || repo;

/** List section for a session: "Today", "Yesterday", the weekday within a week, then month and year. */
export function dayGroup(iso: string | null, now: Date): string {
  if (!iso) return 'Undated';
  const d = new Date(iso);
  const day = (x: Date) => new Date(x.getFullYear(), x.getMonth(), x.getDate()).getTime();
  const days = Math.round((day(now) - day(d)) / DAY);
  if (days <= 0) return 'Today';
  if (days === 1) return 'Yesterday';
  if (days < 7) return d.toLocaleDateString('en-US', { weekday: 'long' });
  return d.toLocaleDateString('en-US', { month: 'long', year: 'numeric' });
}

/** Search words as typed (same split as the Rust `fts_query::build`). */
export const queryTokens = (q: string): string[] =>
  q
    .toLowerCase()
    .split(/[^\p{L}\p{N}_-]+/u)
    .filter(Boolean);

/**
 * How a search matched, best first (mirrors `Rank` in core/src/index.rs): the words in order
 * as typed, every word whole somewhere, or only as word prefixes ("gets" in "getSpecification").
 */
export type MatchMode = 'phrase' | 'words' | 'prefix';

const WORD = /([\p{L}\p{N}_-]+)/u;

/** Alternating separator/word parts; odd indexes are words. */
const splitWords = (text: string): string[] => text.split(WORD);

const isPhraseAt = (lower: string[], i: number, tokens: string[]): boolean =>
  tokens.every((t, j) => {
    const w = lower[i + 2 * j];
    return w !== undefined && (j === tokens.length - 1 ? w.startsWith(t) : w === t);
  });

/** The best way `tokens` match `text`, or null when they don't all match. */
export function matchMode(text: string, tokens: string[]): MatchMode | null {
  if (tokens.length === 0) return null;
  const lower = splitWords(text).map((p) => p.toLowerCase());
  const words = lower.filter((_, i) => i % 2 === 1);
  if (tokens.length > 1 && lower.some((_, i) => i % 2 === 1 && isPhraseAt(lower, i, tokens))) return 'phrase';
  if (tokens.every((t) => words.includes(t))) return 'words';
  if (tokens.every((t) => words.some((w) => w.startsWith(t)))) return 'prefix';
  return null;
}

/**
 * Splits text into runs with the search matches marked: whole phrases (separators included)
 * in `phrase` mode, whole words in `words` mode, word prefixes in `prefix` mode.
 */
export function highlightWords(text: string, tokens: string[], mode: MatchMode | null = 'prefix'): Segment[] {
  if (tokens.length === 0 || mode === null) return [{ text, hit: false }];
  const parts = splitWords(text);
  const lower = parts.map((p) => p.toLowerCase());
  const hit = parts.map(() => false);
  for (let i = 1; i < parts.length; i += 2) {
    if (mode === 'phrase') {
      if (isPhraseAt(lower, i, tokens)) for (let k = i; k <= i + 2 * (tokens.length - 1); k++) hit[k] = true;
    } else if (mode === 'words' ? tokens.includes(lower[i]!) : tokens.some((t) => lower[i]!.startsWith(t))) {
      hit[i] = true;
    }
  }
  const out: Segment[] = [];
  parts.forEach((text, i) => {
    if (!text) return;
    const last = out[out.length - 1];
    if (last && last.hit === hit[i]) last.text += text;
    else out.push({ text, hit: hit[i]! });
  });
  return out;
}

export interface Segment {
  text: string;
  hit: boolean;
}

/** Markdown syntax that only adds noise to a one-line snippet. */
const plainSnippet = (s: string): string => s.replace(/\*\*|__|`+|^#+\s/gm, '').replace(/\s+/g, ' ');

/**
 * Splits a search snippet on its STX/ETX markers into plain and highlighted runs, dropping
 * markdown syntax and joining marked words that are only separated by spaces into one run.
 */
export function snippetSegments(snippet: string): Segment[] {
  const out: Segment[] = [];
  const push = (text: string, hit: boolean) => {
    if (!text) return;
    const last = out[out.length - 1];
    if (last && last.hit === hit) last.text += text;
    else out.push({ text, hit });
  };
  for (const part of snippet.split('\u0002')) {
    const end = part.indexOf('\u0003');
    if (end === -1) {
      push(plainSnippet(part), false);
      continue;
    }
    push(part.slice(0, end), true);
    push(plainSnippet(part.slice(end + 1)), false);
  }
  // "what" " " "gets" reads as one highlighted phrase.
  return out
    .flatMap((s, i) =>
      !s.hit && s.text === ' ' && out[i - 1]?.hit && out[i + 1]?.hit ? [{ text: ' ', hit: true }] : [s],
    )
    .reduce<Segment[]>((acc, s) => {
      const last = acc[acc.length - 1];
      if (last && last.hit === s.hit) last.text += s.text;
      else acc.push({ ...s });
      return acc;
    }, []);
}

export const plural = (n: number, one: string, many: string) => `${n} ${n === 1 ? one : many}`;
