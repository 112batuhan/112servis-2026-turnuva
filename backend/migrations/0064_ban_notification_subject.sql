-- A banned notification is about its recipient, and a player_banned one about a player
-- who may have no team, so they need neither a match nor a team.
-- Separate from 0063, which adds the kind: a new enum value can't be named in the
-- transaction that adds it.
ALTER TABLE notifications
    DROP CONSTRAINT notifications_subject_check,
    ADD CONSTRAINT notifications_subject_check
        CHECK (match_id IS NOT NULL OR team_id IS NOT NULL OR kind IN ('banned', 'player_banned'));
