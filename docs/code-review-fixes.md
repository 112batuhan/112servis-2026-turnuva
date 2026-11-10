# Code review: fixes to make

A whole-codebase review, rechecked against the code after the refactor in commit
`88c6a4c` ("refactor: major refactor"). Line numbers are as of that commit; each item
also names the function or the text to search for, so it can be found if lines move.

At review time, `cargo test`, `cargo fmt --check`, `npm run typecheck` and clippy (one
lint, see 19) all passed. Every authenticated route had a role check that fits it.

Status after the refactor:

- **Already fixed:** out-of-order reloads on the referee page. `lib/useMatchFeed.ts`
  now drops answers that come back out of order, so that item is gone from this list.
- **Partly done:** frontend tests. Vitest now covers draft order, best of, pick'em and
  roles (17).
- **Everything else below is still there.**

---

## Fix before launch

### 1. OAuth `state` isn't tied to the browser

- `backend/src/handlers/auth.rs`: `osu_login` (line 39), `osu_callback` (54),
  `discord_link` (96), `discord_callback` (118)
- `backend/src/main.rs:46`: `csrf_tokens: Arc<Mutex<HashMap<String, ()>>>`

The callbacks only check that `params.state` is somewhere in one global map. The
attack: someone signs in, starts `/auth/discord/link` with their own Discord, stops
before the redirect back, and sends that callback URL to a signed-in referee or host.
The victim's osu! account is then linked to the attacker's Discord. That gives the
attacker the victim's match channels and staff DMs, including the confirm/decline
buttons.

**Fix:** when the flow starts, also set the state in a short-lived cookie (httpOnly,
`SameSite=Lax`, about a 10-minute max-age, `Secure` from `config.secure_cookies()`).
In the callback, require that the cookie matches `params.state`, then clear the cookie.
For the Discord flow, also store the starting user's id next to the state and check it
against the `AuthUser` in the callback.

The map also never expires entries, so every hit on `GET /auth/osu` grows it. Store an
`Instant` with each entry and drop old ones, or drop the map once the cookie does the
job.

### 2. Postgres is published with default credentials

- `docker-compose.yml:7-12`: `POSTGRES_PASSWORD: postgres`, `ports: - "5432:5432"`

On a public server this exposes the database to the internet with the password
`postgres`.

**Fix:** remove the `ports` block, or change it to `"127.0.0.1:5432:5432"`. Take the
password from `.env` (`${POSTGRES_PASSWORD:?…}`) and use it in `DATABASE_URL` (line 31).

### 3. Compose can't turn the Discord bot on

- `docker-compose.yml`, the `app.environment` block

`DISCORD_BOT_TOKEN`, `DISCORD_GUILD_ID` and `BANCHO_IRC_ADDR` aren't passed through
(they're in `.env.example:52,60,61`). `.env` isn't in the image either (see
`.dockerignore`), so the bot never starts under compose.

**Fix:** add `DISCORD_BOT_TOKEN: ${DISCORD_BOT_TOKEN:-}`, the same for
`DISCORD_GUILD_ID`, and `BANCHO_IRC_ADDR: ${BANCHO_IRC_ADDR:-irc.ppy.sh:6667}`. Check
that an empty `DISCORD_GUILD_ID` doesn't fail to parse as `Option<u64>`. If it does,
filter empty strings the way `config.rs` `set()` does for the ARG variables.

### 4. `DEV_LOGIN` has no guard, though the docs say it does

- `README.md:81-83` says the backend refuses to start with `DEV_LOGIN` on when
  `FRONTEND_URL` is https.
- `backend/src/config.rs:55` says the same.
- `backend/src/main.rs:159` only logs a warning.

CLAUDE.md says the opposite: it's meant to work over https while testers use the live
server.

**Fix:** pick one. Either correct the README and the `config.rs` comment, or add a
second flag such as `DEV_LOGIN_ALLOW_HTTPS=true` that has to be set too. Without that
flag, `main` exits when `dev_login && config.secure_cookies()`.

### 5. Linking a Discord account that's already linked gives a 500

- `backend/src/db/user.rs:82`: `link_discord`, `ON CONFLICT (osu_user_id) DO UPDATE`
- `backend/migrations/0001_init.sql:30`: `discord_id TEXT UNIQUE NOT NULL`

The upsert only handles a conflict on `osu_user_id`. If a second osu! account links a
Discord account that's already linked, the insert hits the `discord_id` unique
constraint and the request returns a 500.

**Fix:** in `discord_callback` (`handlers/auth.rs:118`), match
`sqlx::Error::Database(e) if e.constraint() == Some("discord_accounts_discord_id_key")`
and return an `AppError::Conflict`, or a redirect with an error code (next point).
Check the real constraint name in the database first.

A cancelled consent screen (`?error=access_denied`, no `code`) and a bad state also end
on a raw 400 or 502 page. Have both callbacks redirect to
`FRONTEND_URL/discord?error=<code>` (or `/?login_error=<code>`) and show a Turkish
message there.

---

## Resilience

### 6. No timeouts on osu! requests

- `backend/src/main.rs:101`: `http_client: reqwest::Client::new()`
- OAuth exchanges use `oauth2::reqwest::async_http_client`, which has no timeout either
- `backend/src/rank_refresh.rs:145` holds `rank_refresh_lock` while it calls osu!

A hung osu! request never returns. During a rank refresh it keeps the lock, so every
later refresh waits forever.

**Fix:** build the client with `.connect_timeout(10s).timeout(30s)`, as
`discord/mod.rs` `http_client` already does. For oauth2, pass a custom async HTTP
function that uses the shared client, or wrap the `request_async` calls in
`tokio::time::timeout`.

### 7. Background tasks aren't supervised

- `backend/src/bancho/mod.rs:326`: `tokio::spawn(manager::run(...))`
- `backend/src/bancho/manager.rs`: `expect("checked above")` at 858, 902, 925, 1396;
  `expect("online")` 932, `expect("checked")` 1591, `expect("listed")` 1651,
  `expect("joined")` 1653, `unreachable!` 2047
- Also unsupervised: the chat writer and score saver (`manager.rs:228`, `250`), the
  Discord worker and scheduler (`discord/mod.rs:248-249`), `room_scores.rs:32`,
  `rank_refresh.rs:161` and `stats_cache.rs:88`

If one of those `expect`s ever fails, the manager task ends. All room management stops
until a restart, and requests to `state.bancho` hang or fail.

**Fix:** add a small helper such as `spawn_supervised(name, || future)`. It loops,
awaits the `JoinHandle`, logs the panic, waits about 2 s and starts the task again. The
manager rebuilds its rooms from saved chat on start, so restarting it is safe. Its
inbox receiver has to survive the restart: keep it in an `Arc<Mutex<..>>`, or have
`run` take `&mut Receiver` inside the loop. Turning the `expect`s into
`let Some(..) else { return }` also helps.

### 8. Discord jobs queued outside the change's transaction

CLAUDE.md says jobs are queued in the same transaction as the change. These write
through `&state.db`, then call `enqueue(&state.db, …)` as a separate step:

| Where | Function | Write that comes first |
|---|---|---|
| `handlers/matches.rs:317` | `reschedule_lobby` | `db::matches::reschedule_lobby` (queues `TimeSet`) |
| `handlers/matches.rs:337` | `delete_lobby` | `db::matches::delete_lobby` (queues `ArchiveChannel`) |
| `handlers/matches.rs:411` | `set_referee` | `set_referee` / `claim_match` / `release_match` and `follow_referee` (queues `StaffChanged`) |
| `handlers/matches.rs:529` | `remove_room` | `db::matches::remove_room` |
| `services/matches.rs:27` | `link_room` | `db::bancho::add_room` (queues `RoomAdded`) |
| `handlers/bracket.rs:845` / `925` | `set_streamer` / `streamer_changed` | `set_streamer` / `claim_stream` / `release_stream` and `follow_streamer` |
| `handlers/team.rs:500` | `sync_team_matches` (banner upload/removal) | `db::team::set_banner` / `delete_banner` |
| `handlers/stages.rs:141` | `set_stage_best_of` | `db::stages::set_stage_best_of` |

If the process dies or the enqueue fails between the two steps, the change stands but
its Discord post (`TimeSet`, `RoomAdded`, `StaffChanged`, the archive) is lost. The
`SyncAll` on the next Ready fixes channels and cards, but not posts.

**Fix:** open `let mut tx = state.db.begin().await?;`, make the db call take
`&mut *tx`, then `enqueue(&mut *tx, …)` and `tx.commit()`. Most of the db functions
already take `impl PgExecutor`. The calls in `discord/*.rs` and `handlers/discord.rs`
that enqueue on the pool without a write just before them are fine as they are.

### 9. The ARG guess limit can be raced

- `backend/src/handlers/arg.rs:85`: `recent_attempts` is read before the transaction
  that starts at line 92

Parallel requests all read the same count and all get through, which brute-forcing
can use.

**Fix:** inside the transaction, run
`SELECT pg_advisory_xact_lock(hashtext('arg:' || $1::text))` with the user id, then
count, then insert. Or lock the user's `osu_users` row with `FOR UPDATE`.

### 10. WebSockets check roles only when they connect

- `backend/src/handlers/live.rs:69` `connect`: `staff` (76) and `referee` (80) are set
  once
- `backend/src/handlers/irc.rs:56` `connect`: `require_role(Referee)` is checked once
- `backend/src/handlers/live.rs:170`: every `Watch` runs `watched_rooms` (102), a
  database query, even for a guest

A demoted referee keeps staff events, presence and the IRC relay until the socket
closes. Anyone can make the server run a query per message by sending `watch` over and
over.

**Fix:**

- Re-read the user's roles from `sessions` every few minutes in each tab loop, or close
  a user's sockets when `PUT /api/users/:id/roles` changes their roles (keep a
  `user_id -> close` map in `Live`).
- Set `ws.max_message_size(4096)` in both `connect`s.
- Rate-limit `Watch` per tab, for example ignore one that comes within 500 ms of the
  last.

### 11. Operations basics

- `backend/src/main.rs`: `axum::serve(...)` has no `.with_graceful_shutdown(...)`, and
  there's no request log (`tower_http::trace::TraceLayer`)
- `backend/src/db/mod.rs:35`: `.max_connections(10)` is fixed, and the Discord
  listener keeps one of those connections for good
- No backup of the `postgres_data` volume

**Fix:**

- Add `with_graceful_shutdown` on SIGTERM/ctrl-c, so a deploy doesn't cut off saves
  mid-transaction.
- Add `TraceLayer` with the `trace` feature of `tower-http`.
- Make the pool size a `Config` field (`DATABASE_MAX_CONNECTIONS`, default 20).
- Add a cron job or sidecar that runs `pg_dump` daily to somewhere off the server.

---

## Performance

### 12. One large bundle, served uncompressed

- `frontend/src/App.tsx:7-17`: every admin page is imported statically (only
  `OverlayApp` and `DevSwitcher` are lazy)
- `backend/Cargo.toml:11`: `tower-http` features are `["cors", "fs"]`, without
  compression

At review time the main bundle was about 554 KB of JS and 245 KB of CSS, uncompressed.
Every visitor downloads the referee cockpit and the map pool editor.

**Fix:** wrap the admin pages (and `MatchPage`, `StatsPage`, `PickemPage`) in `lazy()`
with a `<Suspense>` around `<Routes>`. Add the `compression-gzip` and `compression-br`
features and `.layer(CompressionLayer::new())` in `main.rs`. If a reverse proxy sits in
front, compress there instead.

### 13. `/api/public/bracket` isn't cached

- `backend/src/handlers/bracket.rs:205` `load_bracket`, `235` `list_public_bracket`

Every `bracket` event makes every open tab refetch it. Each fetch runs four queries,
including every team's members.

**Fix:** reuse the single-flight cache in `backend/src/stats_cache.rs`
(`get_or_count`, line 48) under a fixed key, invalidated by `Live::changes` like the
stats. Cache only the matches and teams; work out `my_team_id` per request.

---

## Frontend robustness

### 14. No error boundary

- No `ErrorBoundary` anywhere in `frontend/src`

One exception during render blanks the whole app: the referee cockpit in the middle of
a match, and the OBS overlay on stream.

**Fix:** add a class component with `componentDidCatch` and a "Bir şeyler ters gitti ·
Yeniden yükle" fallback. Wrap `<main>` in `App.tsx`, the referee page, and
`OverlayApp` (whose fallback renders nothing, so the stream stays clean). Key the
boundary by `location.pathname` so navigating away resets it.

### 15. Server errors show in English, and an expired session isn't handled

- `frontend/src/api/client.ts:6-12` `request` throws the server's text, for example
  "that lobby is full"
- Nothing handles 401 globally: when the 7-day session runs out, the UI still looks
  signed in and every request fails

**Fix:**

- Have `AppError::into_response` (`backend/src/error.rs`) return JSON
  `{ "code": "lobby_full", "message": "…" }`. Map the codes to Turkish in one table in
  the frontend, falling back to `message`.
- In `request`, on a 401 from anything but `/api/me`, call a registered handler
  (`onUnauthenticated`) that `AuthContext` sets, to clear the user and show "Oturumun
  sona erdi".

### 16. No linter, little test coverage, hand-written types

- No ESLint config in `frontend/`
- Vitest covers `draftOrder`, `bestOf`, `pickem` and `roles`. `lib/bracket.ts`
  (`winningSlot`, `isLive`, `potentialRows`, `overlapsAny`) and `lib/bbcode.ts` (which
  guards links and images) have no tests
- `frontend/src/api/types.ts` (1,140 lines) mirrors the Rust structs by hand

**Fix:**

- Add ESLint with `eslint-plugin-react-hooks` (`rules-of-hooks`, `exhaustive-deps`) and
  an `npm run lint` script.
- Add `bracket.test.ts` and `bbcode.test.ts`. For bbcode, include `javascript:` links
  and `data:` images.
- Optionally generate `types.ts` with `ts-rs` (`#[derive(TS)]` on the `Serialize`
  structs) and fail CI when it's out of date.

---

## Maintenance

### 17. Outdated dependencies

- `backend/Cargo.toml:17` pins sqlx 0.7, and `Cargo.lock` has 0.7.4. Cargo warns that a
  future Rust version will reject it, and RUSTSEC-2024-0363 (a protocol-level issue
  with very large binds) is fixed in 0.8.1.
- axum 0.7 / axum-extra 0.9 / tower-http 0.5 and oauth2 4 are each a major version
  behind.

**Fix:** upgrade sqlx to 0.8 first; with runtime queries the changes are small (some
`PgExecutor` bounds). axum 0.8 changes path syntax from `/:id` to `/{id}` and drops
`#[async_trait]` on extractors. oauth2 5 changes the client builder and HTTP client
types. Do each one separately and run the tests in between.

### 18. Clippy's only lint

`result_large_err`, 41 times at review time, caused by
`Discord(#[from] serenity::Error)` in `backend/src/discord/mod.rs:195` (`BotError`).

**Fix:** change it to `Discord(#[from] Box<serenity::Error>)` with a manual
`From<serenity::Error>` that boxes it, then run `cargo clippy --all-targets` again.

### 19. Dockerfile

- `Dockerfile:7`: `RUN npm install` copies only `package.json`, so builds don't use
  the lockfile
- `Dockerfile:29`: `cargo build --release` has no layer that caches dependencies, so
  every change rebuilds them all
- The runtime image runs as root
- `Dockerfile:10` refers to `frontend/src/api.js`, which is now `frontend/src/api/client.ts`

**Fix:** copy `package.json package-lock.json` and use `npm ci`. Cache dependencies
with `--mount=type=cache,target=/usr/local/cargo/registry` and
`target=/app/backend/target`, or build a dummy `main.rs` first. Add
`RUN useradd -r app` and `USER app` to the runtime stage. Fix the comment.

### 20. Bot IRC passwords are stored in plaintext

- `backend/migrations/0045_bancho_bots.sql:8`: `irc_password TEXT NOT NULL`

The bots need the plaintext to log in, so the passwords can't be hashed. Anyone who
gets a database dump gets the bot accounts too.

**Fix:** encrypt them with a key from the environment (`BOT_SECRET_KEY`, 32 bytes) and
`chacha20poly1305` or `aes-gcm`. Do it in `db/bancho.rs` `create_bot`, `set_password`
and `list_logins`, store nonce and ciphertext, and add a migration that encrypts the
existing rows (or have hosts enter them again).

### 21. Repo hygiene and stale docs

- A 20 MB Linux binary is tracked at
  `.claude/skills/impeccable/scripts/bin/linux-x64/impeccable`, and a 1.1 MB
  `font-index.json` beside it. Ignore them, or install the skill per machine.
- There's no CI. Add a GitHub Actions workflow that runs `cargo fmt --check`,
  `cargo clippy --all-targets -- -D warnings`, `cargo test`, `npm ci`,
  `npm run typecheck` and `npm test`.
- `CLAUDE.md` is 89 KB and loaded into every session. Keep the rules and invariants
  there and move the long feature descriptions into `docs/`.
- Stale text:
  - `.env.example:7` says `frontend/vite.config.js` (it's `vite.config.ts`).
  - "the navbar's trophy" in `.env.example:63`, `backend/src/config.rs:80` and
    `backend/src/handlers/arg.rs:1`: the link is now the snowflake in the landing
    page's banner.
  - The database is still called `discord_auth_starter` (`docker-compose.yml:8,31`,
    `.env.example:23`).
  - `notes.txt` in the repo root is a leftover to-do note.

---

## Suggested order

1. Before launch: 1, 2, 3, 5, 4
2. Then 6, 7, 8 (they protect live matches)
3. Then 14, 15, 12
4. Then the rest as time allows
