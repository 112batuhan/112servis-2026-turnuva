# osu! + Discord Auth Starter

A minimal starter project: **Rust (axum) backend** + **React (Vite) frontend**,
with osu! as primary login and optional Discord account linking, backed by Postgres.

```
backend/    Rust API — osu! login, optional Discord linking, JWT auth, Postgres
frontend/   React app — login button, profile view, optional "Link Discord" button
```

## How it works

1. User clicks **Log in with osu!**, which hits `GET /auth/osu` and redirects to osu!'s consent screen.
2. osu! redirects back to `GET /auth/osu/callback` with a code; the backend exchanges it for a token, fetches the user's osu! profile, and **upserts a row in Postgres** keyed by `osu_id`.
3. The backend signs a JWT containing just the user's internal `id` and sets it as an httpOnly cookie.
4. `GET /api/me` verifies that JWT and looks the user up in Postgres, returning their stored profile.
5. Once logged in, the user can optionally click **Link Discord**, which hits `GET /auth/discord/link` — this requires an existing osu! session, so it's only reachable after step 1–4.
6. Discord redirects back to `GET /auth/discord/callback`; the backend reads the current user from the JWT cookie and upserts a row in `discord_accounts` linking it to that osu! user.

Postgres is the source of truth (see `backend/migrations/0001_init.sql`) — the JWT only carries a user id, everything else is looked up on each request. Two tables:

- `osu_users` — one row per osu! login, including their global and country rank at last login.
- `discord_accounts` — at most one row per `osu_users.id` (`osu_user_id` is `UNIQUE`), populated once Discord is linked. That's the table you'd query from a separate Discord bot later, joining on `discord_id`.

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

## 4. Backend setup

```bash
cd backend
cp .env.example .env
# fill in OSU_CLIENT_ID/SECRET, DISCORD_CLIENT_ID/SECRET, DATABASE_URL, and JWT_SECRET
cargo run
```

## 5. Frontend setup

```bash
cd frontend
cp .env.example .env
npm install
npm run dev
```

Open `http://localhost:5173` and log in with osu!.

## Running with Docker

`Dockerfile` builds the frontend, builds the Rust backend, and packages a single
image that serves the built frontend as a static SPA while also handling the
API/OAuth routes. `docker-compose.yml` runs that image alongside Postgres.

```bash
cp .env.example .env
# fill in JWT_SECRET, OSU_CLIENT_ID/SECRET, DISCORD_CLIENT_ID/SECRET
docker compose up --build
```

Open `http://localhost:8080` — set your osu!/Discord OAuth app callback URLs to
`http://localhost:8080/auth/osu/callback` and `http://localhost:8080/auth/discord/callback`
respectively (or whatever host you're deploying to). Migrations run automatically
on container startup, same as `cargo run`.

## Roles

Every user has a role stored on `osu_users.role`: **host** (full access), **map_pooler**
(map-pool work), or **basic** (the default). The role is embedded in the login JWT so
most endpoints can authorize without a database hit.

The JWT also carries `discord_verified` (true once the user has a linked Discord account),
so endpoints can be gated on Discord linkage without a lookup — call
`AuthUser::require_discord_verified()`. It fails closed (an unverified token never grants
access) and is refreshed the moment Discord is linked, as well as on every `/api/me`.

Because the role rides in a long-lived JWT, changing it has to invalidate the old token.
Each user row carries a `token_version` that's also in the JWT; `POST /api/users/{id}/role`
(host-only) changes the role and bumps `token_version`, so:

- privileged handlers call `auth::require_role`, which does one indexed lookup to confirm
  the token's `token_version` still matches the DB — a superseded token is rejected immediately;
- `GET /api/me` re-issues the cookie from the DB row, so an active client's fast-path token
  refreshes to the new role within one poll.

There's no host yet to grant the first one through the API, so bootstrap it by hand:

```sql
UPDATE osu_users SET role = 'host', token_version = token_version + 1 WHERE osu_id = <your_osu_id>;
```

## Notes for taking this further

- **Discord bot integration**: query `discord_accounts` by `discord_id`, joining to `osu_users` for the linked osu! profile.
- **Log out everywhere**: `token_version` already supports it — bump it for a user to invalidate all their existing tokens, not just on role change.
- **HTTPS in production**: set `set_secure(true)` in `auth::auth_cookie` and update redirect URIs to `https://`.
- **Unlinking Discord**: not included — add a `DELETE`-style route that removes the user's `discord_accounts` row.
