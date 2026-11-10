/**
 * A parser for the osu! forum flavour of BBCode, for the rules page. It builds a tree that
 * components/BBCode.tsx renders as React elements (never as HTML strings), so nothing in the text
 * can inject markup. Tags it doesn't know, and closing tags with nothing to close, stay as text,
 * like on the forum.
 */

export type BBNode =
  | { type: "text"; text: string }
  | { type: "tag"; name: TagName; arg: string | null; children: BBNode[] };

const TAGS = [
  "b",
  "i",
  "u",
  "s",
  "strike",
  "color",
  "size",
  "url",
  "email",
  "img",
  "list",
  "*",
  "centre",
  "center",
  "heading",
  "notice",
  "quote",
  "code",
  "c",
  "spoiler",
  "spoilerbox",
  "box",
  "youtube",
  "profile",
] as const;

export type TagName = (typeof TAGS)[number];

const isTag = (name: string): name is TagName => (TAGS as readonly string[]).includes(name);

/** Tags whose content is taken as it is, not parsed: a link or image address, or code. */
const RAW = new Set<TagName>(["img", "email", "youtube", "code", "c", "profile"]);

const TOKEN = /\[(\/?)([a-z]+|\*)(?:=([^\]]*))?\]/gi;

export function parseBBCode(source: string): BBNode[] {
  const text = source.replace(/\r\n?/g, "\n");
  const root: BBNode = { type: "tag", name: "b", arg: null, children: [] };
  const stack: Extract<BBNode, { type: "tag" }>[] = [root];
  const top = () => stack[stack.length - 1];
  const pushText = (value: string) => {
    if (!value) return;
    const children = top().children;
    const last = children[children.length - 1];
    if (last?.type === "text") last.text += value;
    else children.push({ type: "text", text: value });
  };
  /** Closes everything above the innermost open `name`; false when none is open. */
  const closeTo = (name: TagName | TagName[]) => {
    const names = Array.isArray(name) ? name : [name];
    for (let i = stack.length - 1; i > 0; i--) {
      if (names.includes(stack[i].name)) {
        stack.length = i;
        return true;
      }
    }
    return false;
  };

  let pos = 0;
  TOKEN.lastIndex = 0;
  for (let match = TOKEN.exec(text); match; match = TOKEN.exec(text)) {
    const [whole, slash, rawName, rawArg] = match;
    const name = rawName.toLowerCase();
    pushText(text.slice(pos, match.index));
    pos = match.index + whole.length;

    if (!isTag(name) || (slash && name === "*")) {
      pushText(whole);
      continue;
    }

    if (slash) {
      // [/url] also closes a [url] opened with an address; [/center] closes [centre] and so on.
      const names: TagName[] =
        name === "centre" || name === "center"
          ? ["centre", "center"]
          : name === "s" || name === "strike"
            ? ["s", "strike"]
            : [name];
      if (name === "list") closeTo("*");
      if (!closeTo(names)) pushText(whole);
      continue;
    }

    if (name === "*") {
      // An item ends at the next one, or at the end of its list.
      let listIndex = stack.length - 1;
      while (listIndex > 0 && stack[listIndex].name !== "list") listIndex--;
      if (listIndex === 0) {
        pushText(whole);
        continue;
      }
      stack.length = listIndex + 1;
      const item: BBNode = { type: "tag", name: "*", arg: null, children: [] };
      top().children.push(item);
      stack.push(item);
      continue;
    }

    const arg = rawArg === undefined ? null : rawArg.replace(/^"(.*)"$/, "$1");
    const node: Extract<BBNode, { type: "tag" }> = { type: "tag", name, arg, children: [] };

    if (RAW.has(name) || (name === "url" && arg === null)) {
      const close = text.toLowerCase().indexOf(`[/${name}]`, pos);
      if (close === -1) {
        pushText(whole);
        continue;
      }
      node.children.push({ type: "text", text: text.slice(pos, close) });
      top().children.push(node);
      pos = close + name.length + 3;
      TOKEN.lastIndex = pos;
      continue;
    }

    top().children.push(node);
    stack.push(node);
  }
  pushText(text.slice(pos));
  return root.children;
}

/** The plain text inside a node, for heading anchors and the contents list. */
export function textOf(nodes: BBNode[]): string {
  return nodes.map((n) => (n.type === "text" ? n.text : textOf(n.children))).join("");
}

export interface BBHeading {
  id: string;
  title: string;
}

/** Every [heading], in order, with the anchor the renderer gives it. */
export function headingsOf(nodes: BBNode[]): BBHeading[] {
  const out: BBHeading[] = [];
  const seen = new Map<string, number>();
  const walk = (list: BBNode[]) => {
    for (const node of list) {
      if (node.type !== "tag") continue;
      if (node.name === "heading") {
        const title = textOf(node.children).trim();
        out.push({ id: anchorFor(title, seen), title });
      } else {
        walk(node.children);
      }
    }
  };
  walk(nodes);
  return out;
}

function anchorFor(title: string, seen: Map<string, number>): string {
  const base =
    title
      .toLocaleLowerCase("tr-TR")
      .replace(/ı/g, "i")
      .replace(/ğ/g, "g")
      .replace(/ü/g, "u")
      .replace(/ş/g, "s")
      .replace(/ö/g, "o")
      .replace(/ç/g, "c")
      .normalize("NFKD")
      // The accents NFKD splits off (â, î, û), so they don't turn into dashes.
      .replace(/[\u0300-\u036f]/g, "")
      .replace(/[^a-z0-9]+/g, "-")
      .replace(/^-|-$/g, "") || "bolum";
  const count = seen.get(base) ?? 0;
  seen.set(base, count + 1);
  return count ? `${base}-${count + 1}` : base;
}

/** Links and images may only point at the web (or mail for links). */
export function safeUrl(raw: string, allowMail = false): string | null {
  const value = raw.trim();
  try {
    const url = new URL(value);
    if (url.protocol === "http:" || url.protocol === "https:") return url.href;
    if (allowMail && url.protocol === "mailto:") return url.href;
  } catch {
    // not an absolute address
  }
  return null;
}
