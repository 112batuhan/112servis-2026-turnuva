-- Ban notifications:
-- * banned: the hosts banned the recipient, with the reason they gave copied in. It's
--   about the recipient, not a match or a team, so it has neither.
-- * teammate_banned: the hosts banned the recipient's teammate (the actor), to the
--   teammate, about their team.
-- * player_banned: a host banned a player (the actor), to the other hosts, with the
--   reason, about their team if they're in one (otherwise about neither).
ALTER TYPE notification_kind ADD VALUE 'banned';
ALTER TYPE notification_kind ADD VALUE 'teammate_banned';
ALTER TYPE notification_kind ADD VALUE 'player_banned';

-- banned, player_banned: the reason the host gave; empty when none.
ALTER TABLE notifications ADD COLUMN ban_reason TEXT;
