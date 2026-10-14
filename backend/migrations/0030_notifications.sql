-- What a notification is about:
-- * match_rescheduled: a match's teams moved it (to the hosts).
-- * confirm_time: the same, to the match's referee or streamer, who confirms they can
--   make the new time or declines, which takes them off the match.
-- * staff_declined: a referee or streamer declined a new time, to everyone who could
--   take their place (hosts included).
CREATE TYPE notification_kind AS ENUM ('match_rescheduled', 'confirm_time', 'staff_declined');
CREATE TYPE match_staff_role AS ENUM ('referee', 'streamer');
CREATE TYPE notification_response AS ENUM ('confirmed', 'declined');

-- One row per recipient. Everything shown is either a snapshot of the event (the times)
-- or read from the match when listed (its code and teams).
CREATE TABLE notifications (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES osu_users(id) ON DELETE CASCADE,
    kind notification_kind NOT NULL,
    match_id UUID NOT NULL REFERENCES matches(id) ON DELETE CASCADE,
    -- The player who accepted the new time, or the staff member who declined it.
    actor_id UUID REFERENCES osu_users(id) ON DELETE SET NULL,
    -- Which of the match's staff slots it's about; null for match_rescheduled.
    staff_role match_staff_role,
    old_starts_at TIMESTAMPTZ,
    new_starts_at TIMESTAMPTZ NOT NULL,
    -- The answer to a confirm_time.
    response notification_response,
    responded_at TIMESTAMPTZ,
    read_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK ((kind = 'match_rescheduled') = (staff_role IS NULL)),
    CHECK (kind = 'confirm_time' OR response IS NULL)
);

CREATE INDEX notifications_user_id_idx ON notifications (user_id, created_at DESC);
CREATE INDEX notifications_match_id_idx ON notifications (match_id);
