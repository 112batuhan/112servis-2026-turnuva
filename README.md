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

## Notes for taking this further

- **Discord bot integration**: query the `users` table by `discord_id` to look up a Discord member's linked osu! profile.
- **Revocation**: JWTs can't be invalidated before they expire (7 days by default); add a `jti` denylist table if you need instant revocation.
- **HTTPS in production**: set `cookie.set_secure(true)` in `auth/osu.rs` and update redirect URIs to `https://`.
- **Unlinking Discord**: not included — add a `DELETE`-style route that nulls out the `discord_*` columns for the current user.
