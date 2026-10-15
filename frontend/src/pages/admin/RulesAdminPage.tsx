import { useDeferredValue, useEffect, useMemo, useRef, useState, type KeyboardEvent } from "react";
import { Link } from "react-router-dom";
import { errorMessage } from "../../api/client";
import { fetchRules, saveRules } from "../../api/rules";
import type { Rules } from "../../api/types";
import { BBCode } from "../../components/BBCode";
import { ErrorAlert } from "../../components/ErrorAlert";
import { headingsOf, parseBBCode } from "../../lib/bbcode";
import { formatDateTime } from "../../lib/format";
import "./RulesAdminPage.css";

/** What replaces the selection: `before` + `middle` (the selection itself unless given) + `after`. */
type Edit = (selected: string) => { before: string; middle?: string; after: string };

const wrap =
  (open: string, close: string): Edit =>
  () => ({ before: open, after: close });

/** Each selected line becomes an item; with nothing selected, one empty item. */
const list =
  (open: string): Edit =>
  (selected) => ({
    before: `${open}\n`,
    middle: (selected || "").split("\n").map((line) => `[*]${line}`).join("\n"),
    after: "\n[/list]",
  });

const TOOLS: { label: string; title: string; edit: Edit; shortcut?: string }[] = [
  { label: "Kalın", title: "[b]…[/b] (Ctrl+B)", edit: wrap("[b]", "[/b]"), shortcut: "b" },
  { label: "İtalik", title: "[i]…[/i] (Ctrl+I)", edit: wrap("[i]", "[/i]"), shortcut: "i" },
  { label: "Altı çizili", title: "[u]…[/u] (Ctrl+U)", edit: wrap("[u]", "[/u]"), shortcut: "u" },
  { label: "Başlık", title: "[heading]…[/heading]: bölüm başlığı, içindekilerde görünür", edit: wrap("[heading]", "[/heading]") },
  { label: "Numaralı liste", title: "[list=1] ve her satır için [*]", edit: list("[list=1]") },
  { label: "Liste", title: "[list] ve her satır için [*]", edit: list("[list]") },
  { label: "Link", title: "[url=adres]…[/url]", edit: wrap("[url=https://]", "[/url]") },
  { label: "Ortala", title: "[centre]…[/centre]", edit: wrap("[centre]", "[/centre]") },
];

export function RulesAdminPage() {
  const [saved, setSaved] = useState<Rules | null>(null);
  const [draft, setDraft] = useState("");
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const textRef = useRef<HTMLTextAreaElement>(null);

  useEffect(() => {
    fetchRules()
      .then((r) => {
        setSaved(r);
        setDraft(r.bbcode);
      })
      .catch((e) => setError(errorMessage(e)));
  }, []);

  const dirty = saved !== null && draft !== saved.bbcode;

  // The browser asks before leaving or reloading with unsaved changes.
  useEffect(() => {
    if (!dirty) return;
    const warn = (e: BeforeUnloadEvent) => e.preventDefault();
    window.addEventListener("beforeunload", warn);
    return () => window.removeEventListener("beforeunload", warn);
  }, [dirty]);

  // The preview may lag a keystroke behind on a long text rather than slow down typing.
  const previewSource = useDeferredValue(draft);
  const nodes = useMemo(() => parseBBCode(previewSource), [previewSource]);
  const headings = useMemo(() => headingsOf(nodes), [nodes]);

  async function save() {
    if (!saved || !dirty || saving) return;
    setSaving(true);
    try {
      setSaved(await saveRules(draft, saved.updated_at));
      setError(null);
    } catch (e) {
      setError(errorMessage(e));
    } finally {
      setSaving(false);
    }
  }

  /** Applies a toolbar edit to the selection, through the browser so Ctrl+Z can undo it. */
  function apply(edit: Edit) {
    const el = textRef.current;
    if (!el) return;
    const { selectionStart: start, selectionEnd: end, value } = el;
    const selected = value.slice(start, end);
    const { before, middle = selected, after } = edit(selected);
    el.focus();
    const inserted = before + middle + after;
    if (!document.execCommand("insertText", false, inserted)) {
      el.setRangeText(inserted, start, end, "end");
      setDraft(el.value);
    }
    // Leave the original text (or the empty spot between the tags) selected.
    el.setSelectionRange(start + before.length, start + before.length + middle.length);
  }

  function onKeyDown(e: KeyboardEvent<HTMLTextAreaElement>) {
    if (!(e.ctrlKey || e.metaKey) || e.altKey) return;
    const key = e.key.toLowerCase();
    if (key === "s") {
      e.preventDefault();
      void save();
      return;
    }
    const tool = TOOLS.find((t) => t.shortcut === key);
    if (tool && !e.shiftKey) {
      e.preventDefault();
      apply(tool.edit);
    }
  }

  return (
    <section className="rules-admin">
      <div className="page-header">
        <h1>Kurallar</h1>
        <div className="page-header-aside">
          {saved && (
            <span className={`rules-admin-status ${dirty ? "is-dirty" : ""}`} role="status">
              {dirty
                ? "Kaydedilmemiş değişiklikler var"
                : `Son kayıt: ${formatDateTime(saved.updated_at)}${saved.updated_by ? ` · ${saved.updated_by}` : ""}`}
            </span>
          )}
          <Link to="/rules" className="btn btn-secondary" target="_blank" rel="noopener noreferrer">
            Sayfayı aç
          </Link>
          {dirty && (
            <button type="button" className="btn btn-secondary" onClick={() => saved && setDraft(saved.bbcode)}>
              Değişiklikleri geri al
            </button>
          )}
          <button type="button" className="btn btn-primary" disabled={!dirty || saving} onClick={() => void save()}>
            {saving ? "Kaydediliyor…" : "Kaydet"}
          </button>
        </div>
      </div>
      <ErrorAlert message={error} onDismiss={() => setError(null)} />
      {!saved && !error && <p className="status">Yükleniyor…</p>}

      {saved && (
        <div className="rules-admin-panes">
          <div className="rules-admin-pane">
            <div className="rules-admin-bar" role="toolbar" aria-label="Biçimlendirme">
              {TOOLS.map((tool) => (
                <button
                  key={tool.label}
                  type="button"
                  className="btn btn-secondary btn-sm"
                  title={tool.title}
                  onClick={() => apply(tool.edit)}
                >
                  {tool.label}
                </button>
              ))}
            </div>
            <textarea
              ref={textRef}
              className="rules-admin-source"
              value={draft}
              onChange={(e) => setDraft(e.target.value)}
              onKeyDown={onKeyDown}
              spellCheck={false}
              aria-label="Kurallar (BBCode)"
            />
          </div>
          <div className="rules-admin-pane">
            <div className="rules-admin-bar">
              <h2 className="rules-admin-preview-title">Önizleme</h2>
              <span className="rules-admin-hint">osu! forumundaki BBCode etiketleri çalışır.</span>
            </div>
            <div className="rules-admin-preview">
              {draft.trim() ? (
                <BBCode nodes={nodes} headings={headings} />
              ) : (
                <p className="status">Boş bir kural sayfası “Kurallar henüz yazılmadı.” der.</p>
              )}
            </div>
          </div>
        </div>
      )}
    </section>
  );
}
