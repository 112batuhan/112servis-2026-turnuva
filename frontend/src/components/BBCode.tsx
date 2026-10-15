import { Fragment, type CSSProperties, type ReactNode } from "react";
import { type BBHeading, type BBNode, safeUrl, textOf } from "../lib/bbcode";
import "./BBCode.css";

interface Props {
  nodes: BBNode[];
  /** From headingsOf(nodes): the anchors to give the [heading]s, in order. */
  headings: BBHeading[];
}

/** Renders parsed osu! BBCode the way the forum lays it out, in the site's type. */
export function BBCode({ nodes, headings }: Props) {
  const ids = headings.map((h) => h.id);
  return <div className="bbcode">{renderList(nodes, { ids, inLink: false })}</div>;
}

interface Ctx {
  /** Heading anchors still to hand out, consumed in document order. */
  ids: string[];
  inLink: boolean;
}

/** Blocks swallow the line break right before and after them, like on the forum. */
const BLOCKS = new Set(["list", "*", "heading", "notice", "quote", "code", "centre", "center", "box", "spoilerbox"]);
const isBlock = (node: BBNode | undefined) => node?.type === "tag" && BLOCKS.has(node.name);

function renderList(nodes: BBNode[], ctx: Ctx): ReactNode[] {
  return nodes.map((node, i) => {
    if (node.type === "tag") return <Fragment key={i}>{renderTag(node, ctx)}</Fragment>;
    let text = node.text;
    if (isBlock(nodes[i - 1])) text = text.replace(/^\n/, "");
    if (isBlock(nodes[i + 1])) text = text.replace(/\n$/, "");
    return <Fragment key={i}>{renderText(text, ctx)}</Fragment>;
  });
}

const BARE_URL = /https?:\/\/[^\s[\]<>"]+[^\s[\]<>".,;:!?)]/g;

/** Line breaks become <br>, and bare web addresses become links (outside other links). */
function renderText(text: string, ctx: Ctx): ReactNode {
  const lines = text.split("\n");
  return lines.map((line, i) => (
    <Fragment key={i}>
      {i > 0 && <br />}
      {ctx.inLink ? line : autolink(line)}
    </Fragment>
  ));
}

function autolink(line: string): ReactNode {
  const parts: ReactNode[] = [];
  let pos = 0;
  for (const match of line.matchAll(BARE_URL)) {
    const href = safeUrl(match[0]);
    if (!href || match.index === undefined) continue;
    parts.push(line.slice(pos, match.index));
    parts.push(
      <a key={match.index} href={href} target="_blank" rel="noopener noreferrer">
        {match[0]}
      </a>,
    );
    pos = match.index + match[0].length;
  }
  if (!parts.length) return line;
  parts.push(line.slice(pos));
  return parts;
}

/** Inside a list only its items count, and an item's own leading and trailing breaks go. */
function trimEdges(nodes: BBNode[]): BBNode[] {
  return nodes.map((node, i) => {
    if (node.type !== "text") return node;
    let text = node.text;
    if (i === 0) text = text.replace(/^\s+/, "");
    if (i === nodes.length - 1) text = text.replace(/\s+$/, "");
    return { ...node, text };
  });
}

const COLOR = /^(#[0-9a-f]{3}|#[0-9a-f]{6}|[a-z]{3,20})$/i;

function renderTag(node: Extract<BBNode, { type: "tag" }>, ctx: Ctx): ReactNode {
  const children = () => renderList(node.children, ctx);
  const raw = () => textOf(node.children);

  switch (node.name) {
    case "b":
      return <strong>{children()}</strong>;
    case "i":
      return <em>{children()}</em>;
    case "u":
      return <u>{children()}</u>;
    case "s":
    case "strike":
      return <s>{children()}</s>;
    case "color": {
      const color = node.arg?.trim() ?? "";
      return COLOR.test(color) ? <span style={{ color }}>{children()}</span> : children();
    }
    case "size": {
      // The forum's sizes are percentages (50, 85, 100, 150); keep anything else in that range.
      const size = Math.min(200, Math.max(50, Number(node.arg) || 100));
      return <span style={{ fontSize: `${size}%` } as CSSProperties}>{children()}</span>;
    }
    case "url": {
      const href = safeUrl(node.arg ?? raw(), true);
      if (!href) return children();
      return (
        <a href={href} target="_blank" rel="noopener noreferrer">
          {renderList(node.children, { ...ctx, inLink: true })}
        </a>
      );
    }
    case "email": {
      const address = raw().trim();
      return <a href={`mailto:${address}`}>{address}</a>;
    }
    case "profile": {
      const name = raw().trim();
      const href = node.arg ? `https://osu.ppy.sh/users/${encodeURIComponent(node.arg)}` : `https://osu.ppy.sh/users/${encodeURIComponent(name)}`;
      return (
        <a href={href} target="_blank" rel="noopener noreferrer">
          {name}
        </a>
      );
    }
    case "img": {
      const src = safeUrl(raw());
      return src ? <img className="bbcode-img" src={src} alt="" loading="lazy" /> : null;
    }
    case "youtube": {
      const id = raw().trim();
      if (!/^[\w-]{6,20}$/.test(id)) return null;
      return (
        <a href={`https://www.youtube.com/watch?v=${id}`} target="_blank" rel="noopener noreferrer">
          YouTube videosu
        </a>
      );
    }
    case "list": {
      const items = node.children.filter((n) => n.type === "tag" && n.name === "*");
      const rendered = renderList(items, ctx);
      // On the forum any argument makes the list numbered.
      return node.arg === null ? <ul className="bbcode-list">{rendered}</ul> : <ol className="bbcode-list">{rendered}</ol>;
    }
    case "*":
      return <li>{renderList(trimEdges(node.children), ctx)}</li>;
    case "centre":
    case "center":
      return <div className="bbcode-centre">{renderList(trimEdges(node.children), ctx)}</div>;
    case "heading": {
      const id = ctx.ids.shift();
      return (
        <h2 id={id} className="bbcode-heading">
          {renderList(trimEdges(node.children), ctx)}
        </h2>
      );
    }
    case "notice":
      return <div className="bbcode-notice">{renderList(trimEdges(node.children), ctx)}</div>;
    case "quote":
      return (
        <blockquote className="bbcode-quote">
          {node.arg && <p className="bbcode-quote-author">{node.arg} yazdı:</p>}
          {renderList(trimEdges(node.children), ctx)}
        </blockquote>
      );
    case "code":
      return <pre className="bbcode-code">{raw().replace(/^\n|\n$/g, "")}</pre>;
    case "c":
      return <code className="bbcode-inline-code">{raw()}</code>;
    case "spoiler":
      return (
        <span className="bbcode-spoiler" tabIndex={0}>
          {children()}
        </span>
      );
    case "box":
    case "spoilerbox":
      return (
        <details className="bbcode-box">
          <summary>{node.arg || "Spoiler"}</summary>
          <div className="bbcode-box-body">{renderList(trimEdges(node.children), ctx)}</div>
        </details>
      );
  }
}
