import { useEffect, useState, type FormEvent } from "react";
import type { Category, StageDetail } from "../../../api/types";
import { Icon } from "../../../components/Icon";
import { IconButton } from "../../../components/IconButton";
import { InlineEdit } from "../../../components/InlineEdit";
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

function NewCategoryForm({
  busy,
  onCreate,
}: {
  busy: boolean;
  onCreate: (name: string) => Promise<boolean>;
}) {
  const [name, setName] = useState("");

  async function submit(e: FormEvent) {
    e.preventDefault();
    const trimmed = name.trim();
    if (trimmed && (await onCreate(trimmed))) setName("");
  }

  return (
    <form className="inline-form new-category-form" onSubmit={submit}>
      <label className="sr-only" htmlFor="new-category-name">
        Yeni kategori adı
      </label>
      <input
        id="new-category-name"
        className="input"
        value={name}
        onChange={(e) => setName(e.target.value)}
        placeholder="Kategori adı, ör. DoubleTime"
      />
      <button type="submit" className="btn btn-primary" disabled={busy || !name.trim()}>
        <Icon name="plus" />
        Kategori ekle
      </button>
    </form>
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
          <span className="category-name">
            <InlineEdit
              value={category.name}
              label={`${category.name} kategorisini yeniden adlandır`}
              onSave={(name) => actions.renameCategory(category.id, name)}
            />
          </span>
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
