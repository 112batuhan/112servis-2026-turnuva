import { describe, expect, it } from "vitest";
import { headingsOf, parseBBCode, safeUrl, textOf } from "./bbcode";

// The rules page renders this tree as React elements; what's checked here is what keeps the
// text from getting anything but plain links and images through.

describe("safeUrl", () => {
  it("lets web addresses through", () => {
    expect(safeUrl("https://osu.ppy.sh/")).toBe("https://osu.ppy.sh/");
    expect(safeUrl("  http://example.com/a?b=c ")).toBe("http://example.com/a?b=c");
  });

  it("refuses scripts, data and anything relative", () => {
    expect(safeUrl("javascript:alert(1)")).toBeNull();
    expect(safeUrl("JaVaScRiPt:alert(1)")).toBeNull();
    expect(safeUrl(" javascript:alert(1)", true)).toBeNull();
    expect(safeUrl("data:image/png;base64,AAAA")).toBeNull();
    expect(safeUrl("data:text/html,<script>alert(1)</script>")).toBeNull();
    expect(safeUrl("vbscript:msgbox")).toBeNull();
    expect(safeUrl("/relative/path")).toBeNull();
    expect(safeUrl("//evil.example")).toBeNull();
  });

  it("allows mail only where asked (links, not images)", () => {
    expect(safeUrl("mailto:a@b.c")).toBeNull();
    expect(safeUrl("mailto:a@b.c", true)).toBe("mailto:a@b.c");
  });
});

describe("parseBBCode", () => {
  it("nests known tags", () => {
    expect(parseBBCode("[b]bold [i]both[/i][/b]")).toEqual([
      {
        type: "tag",
        name: "b",
        arg: null,
        children: [
          { type: "text", text: "bold " },
          { type: "tag", name: "i", arg: null, children: [{ type: "text", text: "both" }] },
        ],
      },
    ]);
  });

  it("keeps a tag's argument, a link's target included, for the renderer to check", () => {
    const [node] = parseBBCode("[url=javascript:alert(1)]click[/url]");
    expect(node).toMatchObject({ type: "tag", name: "url", arg: "javascript:alert(1)" });
  });

  it("takes an image's address as it is, unparsed", () => {
    const nodes = parseBBCode("[img]data:image/png;base64,[b]x[/b][/img]");
    expect(nodes[0]).toMatchObject({ type: "tag", name: "img" });
    expect(textOf(nodes)).toBe("data:image/png;base64,[b]x[/b]");
  });

  it("leaves unknown tags and stray closing tags as text, and markup as text", () => {
    expect(textOf(parseBBCode("[script]x[/script] [/b] <img onerror=alert(1)>"))).toBe(
      "[script]x[/script] [/b] <img onerror=alert(1)>",
    );
    for (const node of parseBBCode("<b>hi</b>")) expect(node.type).toBe("text");
  });
});

describe("headingsOf", () => {
  it("gives each heading an ASCII anchor, numbering repeats", () => {
    expect(
      headingsOf(
        parseBBCode(
          "[heading]Genel Kurallar[/heading][heading]Şikâyet & İtiraz[/heading][heading]Genel Kurallar[/heading]",
        ),
      ),
    ).toEqual([
      { id: "genel-kurallar", title: "Genel Kurallar" },
      { id: "sikayet-itiraz", title: "Şikâyet & İtiraz" },
      { id: "genel-kurallar-2", title: "Genel Kurallar" },
    ]);
  });
});
