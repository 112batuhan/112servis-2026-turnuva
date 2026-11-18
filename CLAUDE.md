# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository. It holds the rules and invariants; how each feature works is in `docs/architecture/` (index below). Read the relevant file there before changing a feature, and keep it current when the behaviour changes.

## What this is

An osu! tournament site: a Rust (axum 0.8 + sqlx 0.8/Postgres) backend and a React + TypeScript (Vite) frontend. Users log in with osu! OAuth and can link Discord. Map poolers build per-stage map pools; hosts run qualifier lobbies and a double-elimination bracket whose osu! multiplayer rooms are run by the site's own Bancho bots; a Discord bot mirrors it all into the tournament server. There's a public match page, statistics, a pick'em, an OBS overlay and a hidden ARG.

## Commands

A single `.env` at the repo root (copy from `.env.example`) feeds the backend (dotenvy searches parent dirs), the frontend (`envDir: ".."` in `frontend/vite.config.ts`, so only `VITE_*` vars reach it) and docker-compose.

```bash
# Backend (from backend/) — needs a reachable Postgres at DATABASE_URL
cargo run                 # serves on 127.0.0.1:8080, runs migrations on startup
cargo test                # unit tests (Bancho parsing, commands, scoring, pacing, draft, pick'em, stats…)
cargo fmt / cargo clippy --all-targets -- -D warnings

# Frontend (from frontend/)
npm ci
npm run dev               # http://localhost:5173, calls the API at http://localhost:8080
npm run format            # Prettier (config in frontend/.prettierrc.json); CI runs format:check
npm run lint              # ESLint: rules of hooks, exhaustive-deps
npm run typecheck         # tsc (strict, noUnusedLocals/noUnusedParameters)
npm test                  # vitest: rules mirrored from the backend, error codes vs the backend's enums
npm run build             # tsc && vite build

# Everything in one container + Postgres (needs POSTGRES_PASSWORD in .env)
docker compose up --build # http://localhost:8080
```

CI (`.github/workflows/ci.yml`) runs fmt, clippy with `-D warnings`, `cargo test`, `npm ci`, Prettier's check, lint, typecheck and `npm test`; keep all of them passing.

The first host has to be set by hand in SQL: `UPDATE osu_users SET roles = '{host}' WHERE osu_id = <id>;`

`DEV_LOGIN=true` mounts `/api/dev/*` and bundles the account switcher (see `docs/architecture/dev-seed.md`). It works over https too, so testers can use it on the live server, but anyone can then sign in as anyone (hosts included): it must be off before launch. The seed in `backend/seed/` must be kept in step with schema changes.

## Backend layout (`backend/src`)

- `main.rs` builds `AppState` (oauth clients, `PgPool`, the HTTP clients with timeouts, `Arc<Config>`, the cached osu! app token, the `Bancho` handle, the live hub, the caches, the bot secret) and mounts `/auth/*` and `/api/*`, with CORS, compression (not on WebSocket upgrades), a request log and graceful shutdown. If `STATIC_DIR/index.html` exists it also serves the built frontend as a SPA fallback (how the Docker image serves both from one origin, with `VITE_API_URL` built empty).
- `config.rs`: clap `Config` from env vars (each also a CLI flag). Empty variables count as unset (compose passes unset ones as empty). Whether cookies are `Secure` comes from the scheme of `FRONTEND_URL`.
- `handlers/<area>.rs` expose `routes()` and are HTTP only: who's asking and what was sent (shared checks: `check_origin`, `require_runs`, `require_unconcluded` in `handlers/mod.rs`). What more than the routes does to the data is in `services/`, called by handlers, the room bots and the Discord bot alike. Nothing outside `handlers/` imports from it. Shared response shapes are in `views.rs`. `db/<area>.rs` hold the queries.
- `error.rs`: handlers return `Result<_, AppError>`. Every client error is a variant of a grouped enum (`TeamError`, `MatchError`, `LobbyError`…, gathered in `ErrorCode`): the variant carries its English message (`#[error]`) and its group's `status()` gives its HTTP status; `?`/`.into()` make it an `AppError` (`return Err(LobbyError::Full.into())`, `.ok_or(GeneralError::NotFound)?`). It's sent as `{ "group": "lobby", "code": "full", "message": "…" }`. AppError's other variants hold internal causes (database, osu!/Discord requests) and answer with a general code; 5xx causes go to the log only.
- `live.rs` (the WebSocket hub), `stats_cache.rs` (single-flight response cache for the statistics and the public bracket), `supervise.rs` (restarts background tasks that panic), `secret.rs` (bot password encryption), `bancho/` (the room bots), `discord/` (the Discord bot).

## Rules

- **SQL**: runtime `sqlx::query_as::<_, T>(...)`, never the compile-time macros (building needs no database). Every query is one static literal: no `format!`, no spliced column fragments; choose between columns in SQL.
- **Migrations** (`backend/migrations`, embedded with `sqlx::migrate!`, run on startup): add a new numbered file, never edit an applied one.
- **Errors**: a new client error is a new variant in the fitting group enum in `error.rs` (a new group is a new enum plus its `ErrorCode` variant, `status()` arm and `into_app_error!` entry), with its English message and its status in the group's `status()` match, plus its Turkish text under the same group in `ERROR_TEXT` (`frontend/src/api/errors.ts`; `errors.test.ts` checks they agree). The status belongs to the error, never to the call site. Never return raw status codes or error strings. All code, codes and messages are English; Turkish exists only in the frontend's UI text.
- **Sessions** are created and ended only through `auth::start_session` / `auth::end_session`. Roles are read on every request; sockets re-check via `Live::user_changed` (call it when a user's roles change or they log out).
- **Privacy splits**: `editor_notes` never reach `/api/public/*` (separate `Public*` structs; only published stages). `referee_notes` never reach the public qualifier or match routes. Players' Discord accounts are staff-only; staff Discord accounts are public on purpose. Map stats are fixed when a map is added, never recomputed on their own.
- **Discord**: handlers never call Discord. They queue `discord::jobs::Job`s with `state.discord.enqueue(&mut *tx, …)` in the same transaction as the change; anything that changes what a match card shows queues a `SyncMatch`. When a site action should reach Discord, queue the job next to the website notification.
- **Live updates**: after committing a bracket change call `state.live.bracket_changed()` (or `score`); a change to one match the bracket doesn't show calls `match_changed(id)`; one only the referee page shows calls `referee_changed(id)`. A write to `notifications` returns its recipients; pass them to `state.live.notifications_changed(...)` after the commit, so their bells update.
- **Potential matches**: every transaction that changes the bracket's teams takes `db::potential::lock` (before any row lock) and runs `db::potential::sync`.
- **Background tasks** that must keep running go through `supervise::spawn`; a channel they read is a `supervise::SharedReceiver`, so a restarted task reads on.
- **Bot passwords** are only stored encrypted (`BOT_SECRET_KEY`); never log or return them.
- **API types**: `frontend/src/api/types.ts` mirrors the Rust response structs by hand; update it whenever a serialized struct changes.
- **Frontend conventions**: each component keeps its CSS next to it; colours, spacing and font sizes come from `styles/tokens.css`. Icon-only buttons use `<IconButton label="…">`, new icons go in `ICONS` (`components/Icon.tsx`). Anything done by dragging also needs a button/select alternative. A page's changes go through `lib/useAction.ts`.
- **Bans**: every action a player takes part with (teams, lobbies, reschedules, pick'em, anything new like them) calls `user.require_not_banned()`; backing out of something stays open to banned users.
- **Roles** are a set, not a ladder (`role.rs`, mirrored in `frontend/src/auth/roles.ts`): `allows` (holds it, or is host) gates pages and actions; acting in a role (being offered a referee or streamer slot) needs the role itself (`has`).

## Feature docs (`docs/architecture/`)

- `auth-and-roles.md`: sessions, OAuth flows and their state cookie, roles, bans, rules page, staff page.
- `mappool.md`: stages, categories, slots, pool maps and their stats, best of, draft order, pack links.
- `teams.md`: 2v2 teams, invites, rank limit, banners, freezing.
- `matches-and-bracket.md`: matches, qualifier lobbies, the seeding and the bracket made from it, the bracket, forfeits, potential matches, live scores and the live socket, presence, reschedules, notifications.
- `pickem.md`: windows, scoring, privacy, the ballot.
- `statistics.md`: Z-scores, saved map results, accuracy from osu!, the response cache.
- `match-page.md`: the public match page.
- `referee-page.md`: the referee cockpit, talking through the room's bot, bans and picks, the score sheet.
- `bancho-bots.md`: the room bots, the manager, load balancing, pacing, password encryption, supervision, the auto referee.
- `discord-bot.md`: the job queue, match channels, cards, announcements, slash commands.
- `arg.md`: the hidden `/kupa` puzzle.
- `overlay.md`: the OBS overlay and its control dock.
- `frontend.md`: the frontend's structure, error handling, lazy pages, the bracket views.
- `dev-seed.md`: `DEV_LOGIN` and what the dummy seed contains.

`docs/AUTO_REFEREE.md` is the guide to the auto referee for the tournament committee, hosts, referees and players (procedure, commands, switches, a manual test checklist, and every message in Turkish with its English translation); keep it in step with `bancho/autoref/` and the messages it sends.

`docs/RANK_LIMIT_AND_SEEDING.md` explains the rank limit (the formula, where it comes from, what it allows) and, briefly, the Z-percentile seeding, for the committee to review. Keep its constants and table in step with `team_rank.rs`.
