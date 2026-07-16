-- DEV ONLY — DESTROYS ALL DATA.
--
-- Drops the entire `public` schema (all tables, types, and the _sqlx_migrations
-- history) and recreates it empty, so the backend re-applies every migration from
-- scratch on next start. Use when the migration history and the actual tables have
-- drifted out of sync — e.g. after editing an already-applied migration, which
-- leaves sqlx trying to re-run it against tables that already exist.
DROP SCHEMA public CASCADE;
CREATE SCHEMA public;
GRANT ALL ON SCHEMA public TO postgres;
GRANT ALL ON SCHEMA public TO public;
