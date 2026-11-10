# 112servis 2026 Tournament

The website for an osu! 2v2 tournament. It covers the whole tournament: sign-ups, map pools,
qualifier lobbies and a double-elimination bracket. The site's own Bancho bots run the osu!
multiplayer rooms, and a Discord bot mirrors everything into the tournament server.

It's a **Rust** backend (axum 0.8, sqlx 0.8, Postgres) and a **React + TypeScript** frontend
(Vite). Users log in with osu! and link their Discord account. The UI is in Turkish; code, API
error codes and docs are in English.

```
backend/    Rust API, Bancho bots, Discord bot, migrations, dev seed
frontend/   React SPA: public site, staff tools, OBS overlay
docs/       Architecture notes, one file per feature (docs/architecture/)
design/     Reference art for the overlay and icons
```

## Features

**Players and teams**
- osu! OAuth login and Discord account linking. While the Discord bot runs, linking also adds
  the player to the tournament server.
- 2v2 teams formed through invites. The pair's combined rank must stay under a limit, and ranks
  refresh from osu! daily. Teams can upload banners, and hosts can freeze all team changes.
- Players propose and accept new times for their own bracket matches. In-site notifications go
  to players and staff.
- A pick'em: predict each bracket match's winner and score. Each stage opens and locks on its
  own, nobody sees other picks until it locks, and there are standings.

**Map pool and stages**
- Stages with categories and slots. Each map's metadata and mod-adjusted stats (SR, CS, AR, OD,
  BPM, length) are copied in when it's added. There is a shared generic pool, plus editor notes
  that never reach the public site.
- Each stage sets its best of, its ban/pick order (e.g. `b1-b2-p2-*`) and a beatmap pack link.
- A public `/mappool` page with an app-wide song preview player.

**Qualifiers and bracket**
- Qualifier lobbies that teams sign up for (up to 7 per lobby).
- A double-elimination bracket editor on a pan/zoom board, plus list and calendar views. It
  handles winner/loser links, forfeits, and *potential matches*: when a team's next opponent
  isn't decided yet, the possible pairings can still agree on a time.
- Referee and streamer slots that staff claim, with confirm/decline requests when a match
  moves.
- Live scores and bracket updates over one WebSocket per tab.

**Running matches**
- **Bancho bots**: the site's own osu! accounts create, set up and run every multiplayer room.
  They invite players, move them to their slots, kick outsiders, read the scores as maps finish
  and save every chat line. Rooms are spread across the bots within Bancho's per-account room
  limit, and the bots pace their messages.
- **Referee page** (`/admin/referee/:id`): a one-screen cockpit with the pool and the draft,
  the room's slots, the score sheet and the Bancho chat. It sends room commands through the bot
  and has a WebSocket IRC relay so referees can use their own account for private messages.
- **Public match page** (`/match/:id`): the same cockpit, read-only and live, for spectators.
- **Statistics** (`/stats`): per-map and per-stage leaderboards and standings, based on
  Z-scores against the field. Accuracy is filled in afterwards from osu!'s match records.

**Discord bot**
- Hands out the Verified and Player roles and announces teams. It opens a private channel per
  match (archived after the match), sends reminders and public match announcements that keep
  their live score up to date, and posts results.
- DMs referees and streamers with confirm/decline buttons, and has slash commands (`/rank`,
  `/maclar`, `/takim`, `/gecmis`, `/mac`).

**Broadcast**
- An OBS overlay (`/overlay/stream`, 1920×1080) with gameplay, mappool, teams, schedule, bracket
  and winner pages, driven from a control dock (`/overlay/control`) over obs-websocket. Live
  client scores come from [tosu](https://github.com/tosuapp/tosu). The overlay holds back
  results until the delayed spectator clients reach them.

**Everything else**
- Staff roles: host, map pooler, referee, streamer, graphics designer, pool tester, custom
  mapper and commentator. They're a set, not a ladder, and are assigned on `/admin/users`.
- An editable rules page (osu! BBCode) and a public staff page.
- A dev account switcher and dummy seed data for testing (`DEV_LOGIN`).

How each feature works is in [`docs/architecture/`](docs/architecture/). Read the relevant file
before changing a feature.

## Requirements

| What | Used for | Required? |
| --- | --- | --- |
| **Rust** (stable, edition 2021) | Backend | Yes (or Docker) |
| **Node.js 20** + npm | Frontend | Yes (or Docker) |
| **PostgreSQL** (16 in Docker) | All data, including the Discord job queue and team banners | Yes |
| **osu! OAuth app** | Login, plus the osu! API (ranks, beatmaps, match records) | Yes |
| **Discord OAuth app** | Linking Discord accounts | Yes |
| **Discord bot** | Tournament server integration | Optional |
| **osu! accounts for the Bancho bots** | Running multiplayer rooms | Needed for matches |
| **Docker** + Compose | Single-container deploy with Postgres | Optional |

### Postgres

Create an empty database. Migrations (`backend/migrations/`) run automatically on startup.

```bash
createdb tournament
# DATABASE_URL=postgres://postgres:postgres@localhost:5432/tournament
```

docker-compose runs its own Postgres, so you can skip this step when using Docker.

### osu! OAuth app

1. Open [osu! account settings → OAuth](https://osu.ppy.sh/home/account/edit#oauth) →
   **New OAuth Application**.
2. Set the callback URL to `<backend origin>/auth/osu/callback`. Locally that's
   `http://localhost:8080/auth/osu/callback`.
3. Put the client ID and secret in `OSU_CLIENT_ID` / `OSU_CLIENT_SECRET`, and the callback URL
   in `OSU_REDIRECT_URI`.

The same app also gets a client-credentials token (`public` scope) for the backend's osu! API
calls.

### Discord OAuth app (and the optional bot)

1. In the [Discord Developer Portal](https://discord.com/developers/applications), create an
   application.
2. Under **OAuth2 → Redirects**, add `<backend origin>/auth/discord/callback`. Locally that's
   `http://localhost:8080/auth/discord/callback`.
3. Put the client ID and secret in `DISCORD_CLIENT_ID` / `DISCORD_CLIENT_SECRET`, and the
   redirect URL in `DISCORD_REDIRECT_URI`.

To also run the **Discord bot** (optional):

1. Use the **same application**. Linking asks for `guilds.join` to add players to the server,
   and Discord only allows that with the bot of the app that holds the OAuth grant.
2. On the **Bot** page, create a token and put it in `DISCORD_BOT_TOKEN`. Enable the
   **Server Members** privileged intent.
3. Invite the bot to the tournament server with Administrator, or at least Manage Roles, Manage
   Channels, Manage Messages, Pin Messages, Create Invite, Send Messages and Embed Links. Place
   its role above the roles it hands out.
4. Put the server's ID in `DISCORD_GUILD_ID`.
5. After starting the site, a host picks the roles, channels and categories on `/admin/discord`.

The bot runs only when both `DISCORD_BOT_TOKEN` and `DISCORD_GUILD_ID` are set.

### Bancho bots

Matches are run by osu! accounts that the site logs into over IRC. Bancho allows each account
4 rooms at a time, so add more bots for busy days.

1. Generate a key for encrypting the bots' passwords and put it in `BOT_SECRET_KEY`:
   `openssl rand -hex 32`. If the key changes, the saved passwords can't be read, and hosts have
   to enter them again.
2. Get each account's IRC password from
   [osu! account settings → Legacy API](https://osu.ppy.sh/home/account/edit#legacy-api).
3. A host adds each account on `/admin/bancho`.

`BANCHO_IRC_ADDR` (default `irc.ppy.sh:6667`) is used by the bots and by the referee page's IRC
relay. To test locally without Bancho, point it at a stand-in IRC server.

## Running locally

```bash
cp .env.example .env      # fill in at least the osu!/Discord OAuth values and DATABASE_URL

# Backend: http://127.0.0.1:8080, runs migrations on startup
cd backend
cargo run

# Frontend: http://localhost:5173, calls the API at http://localhost:8080
cd frontend
npm ci
npm run dev
```

Open `http://localhost:5173` and log in with osu!. Then make yourself the first host by hand,
since nobody can grant roles yet:

```sql
UPDATE osu_users SET roles = '{host}' WHERE osu_id = <your_osu_id>;
```

Roles are read on every request, so the change applies without logging in again. Hosts grant
all further roles on `/admin/users`.

### Testing with dummy accounts

Set `DEV_LOGIN=true` and restart both sides. A **DEV** button appears in the bottom-left
corner. It signs you in as any account in one click and can reseed or clear the dummy players,
teams, matches and scores from `backend/seed/`. See
[`docs/architecture/dev-seed.md`](docs/architecture/dev-seed.md).

It works over https too, so testers can use it on the live server. But anyone who can reach the
site can then sign in as anyone, hosts included. **Turn it off before launch.**

### Checks

CI (`.github/workflows/ci.yml`) runs all of these, so keep them passing:

```bash
# backend/
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test

# frontend/
npm run lint
npm run typecheck
npm test
```

## Running with Docker

The `Dockerfile` builds the frontend and the backend into one image. The backend serves the
built frontend as a SPA (from `STATIC_DIR`) next to the API and OAuth routes, all from one
origin. `docker-compose.yml` runs that image alongside Postgres 16.

```bash
cp .env.example .env      # fill in the OAuth values and POSTGRES_PASSWORD
docker compose up --build
```

Open `http://localhost:8080`. Point both OAuth callbacks at port 8080, or at your deployed
origin. Compose sets `DATABASE_URL` and `SERVER_ADDR` itself. `VITE_API_URL` and `DEV_LOGIN`
are build arguments, so rebuild the image after changing them.

For production, set `FRONTEND_URL` to your `https://` origin. The session cookie is marked
`Secure` based on its scheme. Update the OAuth redirect URIs to match.

## Environment variables

There's a single `.env` at the repo root, and [`.env.example`](.env.example) documents every
variable. It's read by:

- the backend, through dotenvy, which searches parent directories;
- the frontend, which only sees `VITE_*` variables (`envDir: ".."` in
  `frontend/vite.config.ts`);
- docker-compose.

Each backend variable can also be passed as a CLI flag (`--server-addr`, …). An empty
variable counts as unset.

**Required**

| Variable | Description |
| --- | --- |
| `DATABASE_URL` | Postgres connection string. Local runs only; compose builds its own. |
| `OSU_CLIENT_ID`, `OSU_CLIENT_SECRET` | osu! OAuth app credentials. |
| `OSU_REDIRECT_URI` | `<backend origin>/auth/osu/callback`; must match the osu! app. |
| `DISCORD_CLIENT_ID`, `DISCORD_CLIENT_SECRET` | Discord OAuth app credentials. |
| `DISCORD_REDIRECT_URI` | `<backend origin>/auth/discord/callback`; must match the Discord app. |
| `POSTGRES_PASSWORD` | docker-compose only. Letters and digits only, since it goes into a URL (`openssl rand -hex 24`). Only takes effect when the volume is first created. |
| `BOT_SECRET_KEY` | 64 hex digits (`openssl rand -hex 32`). Encrypts the Bancho bots' IRC passwords; needed before any bot can be saved. |

**Optional**

| Variable | Default | Description |
| --- | --- | --- |
| `SERVER_ADDR` | `127.0.0.1:8080` (Docker: `0.0.0.0:8080`) | Backend listen address. |
| `FRONTEND_URL` | `http://localhost:5173` (Docker: `http://localhost:8080`) | Frontend origin, used for CORS, Origin checks, post-login redirects and whether cookies are `Secure`. |
| `VITE_API_URL` | `http://localhost:8080` (Docker: empty, i.e. same origin) | Where the frontend calls the API. Build-time. |
| `STATIC_DIR` | `./static` (Docker: `/app/static`) | Built frontend to serve as a SPA, if `index.html` exists there. |
| `DATABASE_MAX_CONNECTIONS` | `20` | Postgres pool size. The Discord job worker's listener holds one connection. |
| `BANCHO_IRC_ADDR` | `irc.ppy.sh:6667` | Bancho IRC server for the bots and the referee relay (`cho.ppy.sh:6667` also works). |
| `DISCORD_BOT_TOKEN`, `DISCORD_GUILD_ID` | unset | Turn on the Discord bot when both are set. |
| `DEV_LOGIN` | `false` | Dev account switcher and seed endpoints. Must be off in production. |
| `ARG_ANSWER_1..3`, `ARG_REPLY_1..3` | unset | The `/kupa` puzzle's answers and replies. Off unless all six are set. `{place}` in a reply becomes the finder's place. |
| `ARG_RESULTS_TOKEN` | unset | Turns on the puzzle's results page at `/api/kupa/results/<token>`. |

## Further reading

- [`CLAUDE.md`](CLAUDE.md): the codebase layout and the rules every change follows (SQL,
  migrations, errors, privacy splits, Discord jobs, live updates).
- [`docs/architecture/`](docs/architecture/): how each feature works.
- [`frontend/README.md`](frontend/README.md): frontend conventions.
