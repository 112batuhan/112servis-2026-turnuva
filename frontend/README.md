# Frontend

React + TypeScript, built with Vite. `npm run dev` to develop, `npm run build` to typecheck and build.

## Where things live

```
src/
  api/          Backend calls. types.ts mirrors the Rust response structs.
  auth/         Signed-in user context, role checks, route guard.
  player/       App-wide song preview player state.
  lib/          Small helpers: localStorage, formatting, reordering, drag & drop.
  components/   Reusable UI (MapCard, PoolSection, StageTabs, IconButton, Icon…).
  pages/        One file per route. The map pool editor is in pages/admin/mappool/:
                useStageEditor.ts holds its data + API actions, the .tsx files only render.
  styles/       tokens.css (all colours/sizes), base, controls (buttons, inputs), layout.
```

Each component keeps its CSS next to it (`MapCard.tsx` + `MapCard.css`).

## Conventions

- Colours, spacing and font sizes come from the variables in `styles/tokens.css`.
- Icon-only buttons use `<IconButton label="…">` so they always have an accessible name.
- New icons go in the `ICONS` map in `components/Icon.tsx`.
- Anything done by dragging also needs a button/select alternative (see "Move to…" on map cards).
