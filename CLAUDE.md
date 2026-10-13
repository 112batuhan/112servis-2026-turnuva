# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

An osu! tournament site: a Rust (axum + sqlx/Postgres) backend and a React + TypeScript (Vite) frontend. Users log in with osu! OAuth, can optionally link Discord, and map poolers build per-stage map pools that are published to a public page.

## Commands

A single `.env` at the repo root (copy from `.env.example`) feeds the backend (dotenvy searches parent dirs), the frontend (`envDir: ".."` in `frontend/vite.config.ts`, so only `VITE_*` vars reach it) and docker-compose.

```bash
# Backend (from backend/) — needs a reachable Postgres at DATABASE_URL
cargo run                 # serves on 127.0.0.1:8080, runs migrations on startup
cargo check / cargo clippy / cargo fmt

# Frontend (from frontend/)
npm install
npm run dev               # http://localhost:5173, calls the API at http://localhost:8080
npm run typecheck         # tsc only
npm run build             # tsc && vite build

# Everything in one container + Postgres
docker compose up --build # http://localhost:8080
```

There are no automated tests and no linter config for the frontend. `tsc` runs in strict mode with `noUnusedLocals`/`noUnusedParameters`, so unused code fails the typecheck.

For testing, set `DEV_LOGIN=true` in `.env`: the backend then mounts `/api/dev/*` (`handlers/dev.rs`) and the frontend bundles a floating account switcher (`components/DevSwitcher.tsx`, behind the build-time `__DEV_LOGIN__` constant from `vite.config.ts`) with buttons to reseed or clear dummy data. Dummy accounts have negative `osu_id`s; the seed lives in `backend/seed/` (also runnable with `psql "$DATABASE_URL" -f backend/seed/clear.sql -f backend/seed/seed.sql`) and must be kept in step with schema changes. It defaults to off and must stay off in production; the backend refuses to start with it over https.

The first host has to be set by hand in SQL: `UPDATE osu_users SET role = 'host' WHERE osu_id = <id>;`

## Architecture

### Backend (`backend/src`)

- `main.rs` builds `AppState` (oauth clients, `PgPool`, reqwest client, `Arc<Config>`, cached osu! app token) and mounts `/auth/*` (OAuth flows) and `/api/*` (JSON). If `STATIC_DIR/index.html` exists, it also serves the built frontend as a SPA fallback. This is how the Docker image serves both from one origin, with `VITE_API_URL` built empty.
- `config.rs`: clap `Config` from env vars, each also settable as a CLI flag. Whether cookies are `Secure` comes from the scheme of `FRONTEND_URL`.
- `handlers/<area>.rs` each expose `routes()`; `db/<area>.rs` hold the queries. Queries use runtime `sqlx::query_as::<_, T>(...)`, not the compile-time `query!` macros, so building needs no database.
- `error.rs`: handlers return `Result<_, AppError>`. Clients only get a generic message; 5xx causes go to the log. Add a new variant there instead of returning raw status codes.
- Migrations in `backend/migrations` are embedded with `sqlx::migrate!` and run on startup. Add a new numbered file and never edit an applied one.

### Auth and roles

- Server-side sessions (`auth.rs`, `db/session.rs`). The httpOnly `session` cookie holds a random 32-byte hex token, and the `sessions` table stores only its SHA-256, with a fixed 7-day expiry and no sliding renewal. The `AuthUser` extractor does one query per request that joins the session, `osu_users.role` and whether a Discord account is linked. Use `Option<AuthUser>` for routes that also work signed out.
- Create and end sessions only through `auth::start_session` / `auth::end_session`; they keep the cookie attributes and the stored hash consistent.
- `auth::spawn_session_cleanup` (started in `main.rs`) is a background tokio task that deletes expired sessions once at startup and then hourly.
- Roles form a ladder: `basic < map_pooler < host` (`role.rs`, mirrored in `frontend/src/auth/roles.ts`). Gate handlers with `user.require_role(Role::X)?`. All `/api/stages|categories|slots|pool|maps` routes go through `guard()` in `handlers/mappool.rs` (MapPooler+). User admin is Host-only.
- Because the role is read on every request, role changes and logouts take effect immediately. No cookie needs re-issuing.

### Map pool domain (the core of the app)

The schema has changed a lot across migrations; the current model is:

- `stages` (ordered by `position`, with a `published` flag) → `stage_categories` (ordered, with a `color` `#rrggbb`) → `category_slots` (ordered, with `editor_notes`).
- `pool_maps` holds each map **self-contained**: the beatmap metadata plus the mod-adjusted stats are copied in when the map is added, and there is no beatmap cache table. `slot_id IS NULL` means the map is in the global **generic pool** shared by all stages. Otherwise it fills exactly one slot (`UNIQUE(slot_id)`). Deleting a slot sends its map back to the generic pool. The same beatmap under different mods is a separate row.
- Mods are a concatenated 2-letter acronym string (`"HDDT"`) stored on the map. Stats are fixed when the map is added (`resolve_stats` in `handlers/mappool.rs`): SR/AR/OD come from the osu! difficulty-attributes API (skipped for nomod), and CS/BPM/length are computed in `mods.rs`. They are never recomputed later. HP is not stored.
- Public vs editor views: `editor_notes` (on slots and maps) must never reach the `/api/public/*` routes. These routes use separate `Public*` row structs and only return published stages. Keep that split when adding fields.
- osu! API calls (`osu_api/`) use an app token from the client-credentials flow, cached in memory in `AppState.osu_app_token`.

### Teams (2v2)

- `teams` / `team_members` (a player's `user_id` is the primary key, so one team per player) / `team_invites`. A team is created with both players when an invite is accepted, and doing so drops every other invite involving either of them.
- Eligibility is `team_rank.rs`: each rank maps to a value and the pair's sum must stay at or below `LIMIT`. It's checked when inviting and again when accepting (`handlers/team.rs`).
- Only `basic` users play; staff are filtered out of the search and can't invite or accept. Inviting and accepting also need a linked Discord account.

### Frontend (`frontend/src`)

- `api/client.ts` wraps `fetch` with `credentials: "include"` and throws the server's error text. `api/types.ts` mirrors the Rust response structs by hand, so update it whenever a serialized struct changes.
- `auth/AuthContext.tsx` loads `/api/me` on startup. `RequireAuth minRole=…` guards routes; `/admin/*` needs map_pooler and `/admin/users` needs host.
- The map pool editor lives in `pages/admin/mappool/`. `useStageEditor.ts` owns all of its data and API calls, and the `.tsx` files only render. Structural changes go through `run()`, which shows a busy state and then reloads the stage. Small edits (notes, colours, ordering) update local state first and save in the background.
- `player/PlayerContext.tsx` and `components/MiniPlayer.tsx` provide an app-wide song preview player.

Frontend conventions (from `frontend/README.md`):
- Each component keeps its CSS next to it (`MapCard.tsx` + `MapCard.css`). Colours, spacing and font sizes come from the variables in `styles/tokens.css`.
- Icon-only buttons use `<IconButton label="…">`; new icons go in the `ICONS` map in `components/Icon.tsx`.
- Anything done by dragging also needs a button/select alternative (e.g. "Move to…" on map cards).
