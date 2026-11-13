-- reschedule_accepted: a player accepted a proposed time and the match moved, to every
-- player of both teams but the one who accepted.
ALTER TYPE notification_kind ADD VALUE 'reschedule_accepted';
