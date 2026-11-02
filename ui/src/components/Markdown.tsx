import { memo } from 'react';
import ReactMarkdown, { type Components } from 'react-markdown';
import remarkGfm from 'remark-gfm';
import type { ElementContent, Root, RootContent } from 'hast';
import { api } from '../api';
import { highlightWords, type MatchMode } from '../model/format';
import { useApp } from '../state/store';

/** Wraps search matches in `<mark>`, inside every text node of the rendered message (code included). */
function rehypeMark(tokens: string[], mode: MatchMode | null) {
  const mark = (nodes: RootContent[]): RootContent[] =>
    nodes.flatMap((n): RootContent[] => {
      if (n.type === 'text') {
        const segs = highlightWords(n.value, tokens, mode);
        if (segs.length === 1 && !segs[0]!.hit) return [n];
        return segs.map((s): ElementContent =>
          s.hit
            ? { type: 'element', tagName: 'mark', properties: {}, children: [{ type: 'text', value: s.text }] }
            : { type: 'text', value: s.text },
        );
      }
      if (n.type === 'element') n.children = mark(n.children) as ElementContent[];
      return [n];
    });
  return () => (tree: Root) => {
    if (tokens.length > 0 && mode) tree.children = mark(tree.children);
  };
}

const components: Components = {
  // Links open in the browser; following one inside the app window would replace the app.
  a: ({ href, children }) => (
    <a
      href={href}
      title={href}
      onClick={(e) => {
        e.preventDefault();
        if (!href) return;
        api.openLink(href).catch((err: unknown) => useApp.getState().notify(String(err), 'error'));
      }}
    >
      {children}
    </a>
  ),
  img: ({ alt }) => <span className="text-mute">[image{alt ? `: ${alt}` : ''}]</span>,
};

/** A transcript message rendered as GitHub-flavoured markdown, with search matches marked. */
export const Markdown = memo(function Markdown({
  text,
  tokens,
  mode,
}: {
  text: string;
  tokens: string[];
  mode: MatchMode | null;
}) {
  return (
    <div className="chat-md">
      <ReactMarkdown remarkPlugins={[remarkGfm]} rehypePlugins={[rehypeMark(tokens, mode)]} components={components}>
        {text}
      </ReactMarkdown>
    </div>
  );
});
