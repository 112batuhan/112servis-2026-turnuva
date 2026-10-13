# osu! + Discord Auth Starter

A minimal starter project: **Rust (axum) backend** + **React (Vite) frontend**,
with osu! as primary login and optional Discord account linking, backed by Postgres.

```
backend/    Rust API — osu! login, optional Discord linking, session auth, Postgres
frontend/   React app — login button, profile view, optional "Link Discord" button
```

## How it works

1. User clicks **Log in with osu!**, which hits `GET /auth/osu` and redirects to osu!'s consent screen.
2. osu! redirects back to `GET /auth/osu/callback` with a code; the backend exchanges it for a token, fetches the user's osu! profile, and **upserts a row in Postgres** keyed by `osu_id`.
3. The backend creates a row in `sessions` and sets a random session token as an httpOnly cookie. Only the token's SHA-256 is stored.
4. `GET /api/me` looks the session up in Postgres and returns the user's stored profile.
5. Once logged in, the user can optionally click **Link Discord**, which hits `GET /auth/discord/link` — this requires an existing osu! session, so it's only reachable after step 1–4.
6. Discord redirects back to `GET /auth/discord/callback`; the backend reads the current user from the session cookie and upserts a row in `discord_accounts` linking it to that osu! user.

Postgres is the source of truth (see `backend/migrations/0001_init.sql`). The cookie is an opaque token, and the session, role and Discord status are looked up on each request. The user tables are:

- `osu_users` — one row per osu! login, including their global and country rank at last login.
- `discord_accounts` — at most one row per `osu_users.id` (`osu_user_id` is `UNIQUE`), populated once Discord is linked. That's the table you'd query from a separate Discord bot later, joining on `discord_id`.
- `sessions` — one row per login, keyed by the SHA-256 of the cookie token, expiring 7 days after login. A background task deletes expired rows every hour (and once at startup).

`GET /api/me` returns the osu! profile with the linked Discord account nested under `discord` (`null` if unlinked).

## 1. Create a Postgres database

```bash
createdb discord_auth_starter
```

Migrations run automatically on backend startup — no separate migration step needed.

## 2. Create an osu! OAuth app

1. Go to your [osu! account settings → OAuth](https://osu.ppy.sh/home/account/edit#oauth) → **New OAuth Application**.
2. Set the callback URL to `http://localhost:8080/auth/osu/callback`.
3. Copy the Client ID and Client Secret.

## 3. Create a Discord OAuth app (for account linking)

1. Go to the [Discord Developer Portal](https://discord.com/developers/applications) → **New Application**.
2. Under **OAuth2 → Redirects**, add `http://localhost:8080/auth/discord/callback`.
3. Copy the Client ID and Client Secret from **OAuth2 → General**.

## 4. Configure

There's a single `.env` at the repo root, shared by the backend, frontend, and
docker-compose. Copy it and fill in your OAuth credentials and `DATABASE_URL`:

```bash
cp .env.example .env
# fill in OSU_CLIENT_ID/SECRET, DISCORD_CLIENT_ID/SECRET, and DATABASE_URL
```

## 5. Run the backend

```bash
cd backend
cargo run
```

## 6. Run the frontend

```bash
cd frontend
npm install
npm run dev
```

Open `http://localhost:5173` and log in with osu!.

## Testing with dummy accounts

Set `DEV_LOGIN=true` in `.env` and restart both the backend and the frontend. A **DEV**
button appears in the bottom-left corner: it switches to any account with one click
and can reseed or clear the dummy players, teams and invites from `backend/seed/`.

Leave `DEV_LOGIN` unset (or `false`) on live servers. The backend then doesn't serve the
dev endpoints, the frontend build doesn't contain the switcher, and the backend refuses
to start with it on when `FRONTEND_URL` is https.

## Running with Docker

`Dockerfile` builds the frontend, builds the Rust backend, and packages a single
image that serves the built frontend as a static SPA while also handling the
API/OAuth routes. `docker-compose.yml` runs that image alongside Postgres.

```bash
cp .env.example .env
# fill in OSU_CLIENT_ID/SECRET, DISCORD_CLIENT_ID/SECRET
docker compose up --build
```

Open `http://localhost:8080` — set your osu!/Discord OAuth app callback URLs to
`http://localhost:8080/auth/osu/callback` and `http://localhost:8080/auth/discord/callback`
respectively (or whatever host you're deploying to). Migrations run automatically
on container startup, same as `cargo run`.

## Roles

Every user has a role stored on `osu_users.role`: **host** (full access), **map_pooler**
(map-pool work, and anything a referee can do), **referee** (runs qualifier lobbies), or
**basic** (the default). Handlers gate on it with
`AuthUser::require_role`.

The `AuthUser` extractor loads the session, the user's role and whether they have a linked
Discord account from Postgres on every request. So:

- a role change takes effect on that user's next request, with no re-login;
- logging out deletes the session row, so the old cookie stops working immediately;
- endpoints can require a linked Discord account with `AuthUser::require_discord_verified()`,
  which fails closed and applies as soon as Discord is linked.

There's no host yet to grant the first one through the API, so bootstrap it by hand:

```sql
UPDATE osu_users SET role = 'host' WHERE osu_id = <your_osu_id>;
```

## Notes for taking this further

- **Discord bot integration**: query `discord_accounts` by `discord_id`, joining to `osu_users` for the linked osu! profile.
- **Log out everywhere**: `DELETE FROM sessions WHERE user_id = $1` ends every session for a user (indexed on `user_id`).
- **HTTPS in production**: set `FRONTEND_URL` to an `https://` origin (the cookie is marked `Secure` automatically from its scheme, see `Config::secure_cookies`) and update the OAuth redirect URIs to `https://`.
- **Unlinking Discord**: not included — add a `DELETE`-style route that removes the user's `discord_accounts` row.
