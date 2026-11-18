import { useEffect, useState, type FormEvent } from "react";
import type { NewCategory } from "../../../api/mappool";
import type { Category, CategoryKind, StageDetail } from "../../../api/types";
import { Icon } from "../../../components/Icon";
import { IconButton } from "../../../components/IconButton";
import { InlineEdit } from "../../../components/InlineEdit";
import { CATEGORY_KINDS, cleanAbbreviation, isAbbreviation, kindOf } from "../../../lib/category";
import { useDragReorder } from "../../../lib/dragAndDrop";
import type { StageEditorActions } from "./useStageEditor";

interface Props {
  detail: StageDetail;
  busy: boolean;
  actions: StageEditorActions;
}

export function StageSettings({ detail, busy, actions }: Props) {
  return (
    <section className="panel stage-settings" aria-label="Stage ayarları">
      <div className="stage-settings-head">
        <h2 className="stage-settings-title">{detail.name}</h2>
        {detail.is_qualifier && <span className="badge badge-info">Eleme mappool'u</span>}
        <span className={`badge ${detail.published ? "badge-success" : "badge-warning"}`}>
          {detail.published ? "Yayında" : "Taslak"}
        </span>

        <div className="stage-settings-actions">
          <button
            type="button"
            className={`btn ${detail.published ? "btn-secondary" : "btn-primary"}`}
            disabled={busy}
            onClick={actions.togglePublished}
          >
            {detail.published ? "Yayından kaldır" : "Yayınla"}
          </button>
        </div>
      </div>

      <h3 className="stage-settings-subtitle">Beatmap pack</h3>
      <PackUrlForm detail={detail} busy={busy} onSave={actions.savePackUrl} />

      <h3 className="stage-settings-subtitle">Kategoriler</h3>
      <NewCategoryForm busy={busy} onCreate={actions.createCategory} />
      <CategoryList detail={detail} actions={actions} />
    </section>
  );
}

/**
 * The link to the stage's beatmap pack, which is uploaded elsewhere. Emptying the field turns "Kaydet"
 * into "Kaldır"; the public mappool page shows "Toplu indir" while there's a link.
 */
function PackUrlForm({
  detail,
  busy,
  onSave,
}: {
  detail: StageDetail;
  busy: boolean;
  onSave: (url: string) => Promise<boolean>;
}) {
  const saved = detail.pack_url ?? "";
  const [url, setUrl] = useState(saved);
  useEffect(() => setUrl(saved), [saved]);
  const trimmed = url.trim();
  const removing = !trimmed && saved !== "";
  const hintId = `stage-pack-hint-${detail.id}`;

  function submit(e: FormEvent) {
    e.preventDefault();
    if (trimmed !== saved) onSave(trimmed);
  }

  return (
    <div className="stage-pack">
      <form className="inline-form stage-pack-form" onSubmit={submit}>
        <label className="sr-only" htmlFor="stage-pack-url">
          Beatmap pack linki
        </label>
        <input
          id="stage-pack-url"
          className="input"
          type="url"
          inputMode="url"
          spellCheck={false}
          autoComplete="off"
          value={url}
          onChange={(e) => setUrl(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Escape") setUrl(saved);
          }}
          placeholder="https://…"
          aria-describedby={saved ? hintId : undefined}
        />
        <button
          type="submit"
          className={`btn ${removing ? "btn-danger" : "btn-secondary"}`}
          disabled={busy || trimmed === saved}
        >
          {removing ? "Kaldır" : "Kaydet"}
        </button>
        {saved && (
          <a
            className="icon-btn"
            href={saved}
            target="_blank"
            rel="noopener noreferrer"
            aria-label="Kayıtlı pack linkini aç"
            title="Kayıtlı pack linkini aç"
          >
            <Icon name="download" size={16} />
          </a>
        )}
      </form>
      {saved && (
        <p className="stage-pack-hint" id={hintId}>
          {detail.published
            ? "Mappool sayfasındaki “Toplu indir” butonu bu linke gidiyor."
            : "Stage yayınlanınca Mappool sayfasında “Toplu indir” butonu bu linke gidecek."}
        </p>
      )}
    </div>
  );
}

/**
 * A new category: its short name (its slots' labels, what players type to ban and pick), its name
 * and its kind. Typing NM, FM or TB picks the kind they mean, until the kind is chosen by hand.
 */
function NewCategoryForm({
  busy,
  onCreate,
}: {
  busy: boolean;
  onCreate: (category: NewCategory) => Promise<boolean>;
}) {
  const [abbreviation, setAbbreviation] = useState("");
  const [name, setName] = useState("");
  const [kind, setKind] = useState<CategoryKind>("modded");
  const [kindChosen, setKindChosen] = useState(false);
  const abbr = cleanAbbreviation(abbreviation);
  const badAbbr = abbr !== "" && !isAbbreviation(abbr);
  const ready = isAbbreviation(abbr) && name.trim() !== "";

  function typeAbbreviation(value: string) {
    setAbbreviation(value);
    const guess = kindOf(value);
    if (!kindChosen) setKind(guess ?? "modded");
  }

  async function submit(e: FormEvent) {
    e.preventDefault();
    if (!ready) return;
    if (await onCreate({ name: name.trim(), abbreviation: abbr, kind })) {
      setAbbreviation("");
      setName("");
      setKind("modded");
      setKindChosen(false);
    }
  }

  return (
    <form className="new-category-form" onSubmit={submit}>
      <label className="new-category-field">
        <span className="new-category-label">Kısaltma</span>
        <input
          className="input category-abbr"
          value={abbreviation}
          onChange={(e) => typeAbbreviation(e.target.value)}
          placeholder="NM"
          maxLength={6}
          spellCheck={false}
          autoComplete="off"
          aria-invalid={badAbbr}
          aria-describedby="new-category-hint"
        />
      </label>
      <label className="new-category-field new-category-name">
        <span className="new-category-label">Ad</span>
        <input
          className="input"
          value={name}
          onChange={(e) => setName(e.target.value)}
          placeholder="ör. NoMod"
        />
      </label>
      <label className="new-category-field">
        <span className="new-category-label">Tür</span>
        <KindSelect
          value={kind}
          onChange={(next) => {
            setKind(next);
            setKindChosen(true);
          }}
        />
      </label>
      <button type="submit" className="btn btn-primary" disabled={busy || !ready}>
        <Icon name="plus" />
        Kategori ekle
      </button>
      <p
        id="new-category-hint"
        className={`new-category-hint${badAbbr ? " is-error" : ""}`}
        aria-live="polite"
      >
        {badAbbr
          ? "Kısaltma yalnızca harf olabilir, en fazla 6 tane."
          : `Slotlar kısaltmayla anılır (${abbr || "NM"}1, ${abbr || "NM"}2…); oyuncular ban ve pick için bunu yazar.`}
      </p>
    </form>
  );
}

/** A category's kind: the rules its maps follow (lib/category.ts). */
function KindSelect({
  value,
  onChange,
  label,
}: {
  value: CategoryKind;
  onChange: (kind: CategoryKind) => void;
  label?: string;
}) {
  const hint = CATEGORY_KINDS.find((k) => k.kind === value)?.hint;
  return (
    <select
      className="select category-kind"
      value={value}
      aria-label={label}
      title={hint}
      onChange={(e) => onChange(e.target.value as CategoryKind)}
    >
      {CATEGORY_KINDS.map((k) => (
        <option key={k.kind} value={k.kind} title={k.hint}>
          {k.label}
        </option>
      ))}
    </select>
  );
}

/**
 * A category's short name. Enter or leaving the field saves it; Escape, or leaving it empty, puts
 * back the saved one.
 */
function CategoryAbbreviation({
  category,
  onSave,
}: {
  category: Category;
  onSave: (abbreviation: string) => void;
}) {
  const saved = category.abbreviation;
  const [text, setText] = useState(saved);
  useEffect(() => setText(saved), [saved]);

  function commit() {
    const next = cleanAbbreviation(text);
    if (!next) return setText(saved);
    setText(next);
    if (next !== saved) onSave(next);
  }

  return (
    <input
      className="input input-sm category-abbr"
      value={text}
      maxLength={6}
      spellCheck={false}
      autoComplete="off"
      aria-label={`${category.name}: kısaltma`}
      title="Slotların etiketi (NM1, NM2…): oyuncular ban ve pick için bunu yazar"
      onChange={(e) => setText(e.target.value)}
      onBlur={commit}
      onKeyDown={(e) => {
        if (e.key === "Enter") e.currentTarget.blur();
        if (e.key === "Escape") {
          setText(saved);
          e.currentTarget.blur();
        }
      }}
    />
  );
}

function CategoryList({ detail, actions }: { detail: StageDetail; actions: StageEditorActions }) {
  const reorderProps = useDragReorder(actions.dragCategory);

  if (detail.categories.length === 0) return <p className="pool-empty">Henüz kategori yok.</p>;

  return (
    <ol className="category-list">
      {detail.categories.map((category, i) => (
        <li key={category.id} className="category-row" {...reorderProps(category.id)}>
          <span className="category-grip" title="Sıralamak için sürükle">
            <Icon name="grip" />
          </span>
          <CategoryColor category={category} actions={actions} />
          <CategoryAbbreviation
            category={category}
            onSave={(abbr) => actions.setCategoryAbbreviation(category.id, abbr)}
          />
          <span className="category-name">
            <InlineEdit
              value={category.name}
              label={`${category.name} kategorisini yeniden adlandır`}
              onSave={(name) => actions.renameCategory(category.id, name)}
            />
          </span>
          <KindSelect
            value={category.kind}
            label={`${category.name}: tür`}
            onChange={(kind) => actions.setCategoryKind(category.id, kind)}
          />
          <span className="category-meta">
            {detail.slots.filter((s) => s.category_id === category.id).length} slot
          </span>
          <IconButton
            icon="arrowUp"
            label={`${category.name} kategorisini yukarı taşı`}
            disabled={i === 0}
            onClick={() => actions.shiftCategory(category.id, -1)}
          />
          <IconButton
            icon="arrowDown"
            label={`${category.name} kategorisini aşağı taşı`}
            disabled={i === detail.categories.length - 1}
            onClick={() => actions.shiftCategory(category.id, 1)}
          />
          <IconButton
            icon="trash"
            label={`${category.name} kategorisini sil`}
            tone="danger"
            onClick={() => {
              if (
                confirm(
                  `"${category.name}" kategorisi silinsin mi? Slot'ları ve notları kaldırılır; map'ler ortak havuza döner.`,
                )
              ) {
                actions.deleteCategory(category.id);
              }
            }}
          />
        </li>
      ))}
    </ol>
  );
}

function CategoryColor({ category, actions }: { category: Category; actions: StageEditorActions }) {
  return (
    <input
      type="color"
      className="category-color"
      value={category.color}
      aria-label={`${category.name} kategorisinin rengi`}
      title="Kategori rengi"
      onChange={(e) => actions.previewCategoryColor(category.id, e.target.value)}
      onBlur={(e) => actions.saveCategoryColor(category.id, e.target.value)}
    />
  );
}
