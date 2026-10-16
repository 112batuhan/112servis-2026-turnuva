-- The public announcement of a bracket match, posted in the announcement channel 15
-- minutes before it, and kept up to date like the match card: its message, and what it
-- last showed so an unchanged match isn't edited again. A match moved after it went
-- out gets a new one for its new time, and the old message is deleted then.
ALTER TABLE match_discord
    ADD COLUMN announce_channel_id BIGINT,
    ADD COLUMN announce_message_id BIGINT,
    ADD COLUMN announce_synced TEXT;
