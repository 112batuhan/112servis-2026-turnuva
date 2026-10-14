-- Notifications for players too:
-- * reschedule_proposed: the opponent proposed times for a match, to both players of
--   the other team. Further proposals don't add another while one is still unread.
-- * rank_over_limit: a rank update took a team over the limit, to its players and the
--   hosts.
ALTER TYPE notification_kind ADD VALUE 'reschedule_proposed';
ALTER TYPE notification_kind ADD VALUE 'rank_over_limit';

-- The new kinds can't be named until this migration commits, so the checks below only
-- name the old ones.
ALTER TABLE notifications
    ALTER COLUMN match_id DROP NOT NULL,
    ALTER COLUMN new_starts_at DROP NOT NULL,
    -- The recipient's team for reschedule_proposed, the team over the limit for
    -- rank_over_limit.
    ADD COLUMN team_id UUID REFERENCES teams(id) ON DELETE CASCADE,
    -- rank_over_limit: the actor's rank change that did it and the team value it led to.
    ADD COLUMN old_rank BIGINT,
    ADD COLUMN new_rank BIGINT,
    ADD COLUMN team_value DOUBLE PRECISION,
    DROP CONSTRAINT notifications_check,
    ADD CONSTRAINT notifications_staff_role_check
        CHECK ((kind IN ('confirm_time', 'staff_declined')) = (staff_role IS NOT NULL)),
    ADD CONSTRAINT notifications_match_check
        CHECK (kind NOT IN ('match_rescheduled', 'confirm_time', 'staff_declined')
               OR (match_id IS NOT NULL AND new_starts_at IS NOT NULL)),
    ADD CONSTRAINT notifications_subject_check CHECK (match_id IS NOT NULL OR team_id IS NOT NULL);

CREATE INDEX notifications_team_id_idx ON notifications (team_id);
