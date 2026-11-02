import { snippetSegments } from '../model/format';

/** A search snippet with its matched words highlighted. */
export function Snippet({ text, className }: { text: string; className?: string }) {
  return (
    <span className={className}>
      {snippetSegments(text).map((s, i) => (s.hit ? <mark key={i}>{s.text}</mark> : <span key={i}>{s.text}</span>))}
    </span>
  );
}
