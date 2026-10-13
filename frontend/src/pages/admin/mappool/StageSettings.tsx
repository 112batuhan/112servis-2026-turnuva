import { useState, type FormEvent } from "react";
import type { Category, Stage, StageDetail } from "../../../api/types";
import { Icon } from "../../../components/Icon";
import { IconButton } from "../../../components/IconButton";
import { InlineEdit } from "../../../components/InlineEdit";
import { useDragReorder } from "../../../lib/dragAndDrop";
import { plural } from "../../../lib/format";
import type { StageEditorActions } from "./useStageEditor";

interface Props {
  detail: StageDetail;
  stages: Stage[];
  busy: boolean;
  actions: StageEditorActions;
}

export function StageSettings({ detail, stages, busy, actions }: Props) {
  const index = stages.findIndex((s) => s.id === detail.id);

  function deleteStage() {
    if (confirm(`Delete stage "${detail.name}"? Its categories and slots are removed; maps return to the generic pool.`)) {
      actions.deleteStage();
    }
  }

  return (
    <section className="panel stage-settings" aria-label="Stage settings">
      <div className="stage-settings-head">
        <h2 className="stage-settings-title">
          <InlineEdit value={detail.name} label="Rename stage" onSave={actions.renameStage} />
        </h2>
        <span className={`badge ${detail.published ? "badge-success" : "badge-warning"}`}>
          {detail.published ? "Published" : "Draft"}
        </span>

        <div className="stage-settings-actions">
          <IconButton
            icon="arrowLeft"
            label="Move stage earlier"
            variant="outlined"
            disabled={index <= 0}
            onClick={() => actions.shiftStage(detail.id, -1)}
          />
          <IconButton
            icon="arrowRight"
            label="Move stage later"
            variant="outlined"
            disabled={index < 0 || index >= stages.length - 1}
            onClick={() => actions.shiftStage(detail.id, 1)}
          />
          <button
            type="button"
            className={`btn ${detail.published ? "btn-secondary" : "btn-primary"}`}
            disabled={busy}
            onClick={actions.togglePublished}
          >
            {detail.published ? "Unpublish" : "Publish"}
          </button>
          <button type="button" className="btn btn-danger" disabled={busy} onClick={deleteStage}>
            <Icon name="trash" />
            Delete stage
          </button>
        </div>
      </div>

      <h3 className="stage-settings-subtitle">Categories</h3>
      <NewCategoryForm busy={busy} onCreate={actions.createCategory} />
      <CategoryList detail={detail} actions={actions} />
    </section>
  );
}

function NewCategoryForm({ busy, onCreate }: { busy: boolean; onCreate: (name: string) => Promise<boolean> }) {
  const [name, setName] = useState("");

  async function submit(e: FormEvent) {
    e.preventDefault();
    const trimmed = name.trim();
    if (trimmed && (await onCreate(trimmed))) setName("");
  }

  return (
    <form className="inline-form new-category-form" onSubmit={submit}>
      <label className="sr-only" htmlFor="new-category-name">
        New category name
      </label>
      <input
        id="new-category-name"
        className="input"
        value={name}
        onChange={(e) => setName(e.target.value)}
        placeholder="Category name, e.g. DoubleTime"
      />
      <button type="submit" className="btn btn-primary" disabled={busy || !name.trim()}>
        <Icon name="plus" />
        Add category
      </button>
    </form>
  );
}

function CategoryList({ detail, actions }: { detail: StageDetail; actions: StageEditorActions }) {
  const reorderProps = useDragReorder(actions.dragCategory);

  if (detail.categories.length === 0) return <p className="pool-empty">No categories yet.</p>;

  return (
    <ol className="category-list">
      {detail.categories.map((category, i) => (
        <li key={category.id} className="category-row" {...reorderProps(category.id)}>
          <span className="category-grip" title="Drag to reorder">
            <Icon name="grip" />
          </span>
          <CategoryColor category={category} actions={actions} />
          <span className="category-name">
            <InlineEdit
              value={category.name}
              label={`Rename ${category.name}`}
              onSave={(name) => actions.renameCategory(category.id, name)}
            />
          </span>
          <span className="category-meta">
            {plural(detail.slots.filter((s) => s.category_id === category.id).length, "slot")}
          </span>
          <IconButton
            icon="arrowUp"
            label={`Move ${category.name} up`}
            disabled={i === 0}
            onClick={() => actions.shiftCategory(category.id, -1)}
          />
          <IconButton
            icon="arrowDown"
            label={`Move ${category.name} down`}
            disabled={i === detail.categories.length - 1}
            onClick={() => actions.shiftCategory(category.id, 1)}
          />
          <IconButton
            icon="trash"
            label={`Delete ${category.name}`}
            tone="danger"
            onClick={() => {
              if (confirm(`Delete category "${category.name}"? Its slots and notes are removed; maps return to the generic pool.`)) {
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
      aria-label={`Colour for ${category.name}`}
      title="Category colour"
      onChange={(e) => actions.previewCategoryColor(category.id, e.target.value)}
      onBlur={(e) => actions.saveCategoryColor(category.id, e.target.value)}
    />
  );
}
