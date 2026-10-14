-- The pinned embed in each match channel that shows the match as it is now (time,
-- teams, staff, rooms). card_synced is what it last showed, so an unchanged match
-- isn't edited again.
ALTER TABLE match_discord
    ADD COLUMN card_message_id BIGINT,
    ADD COLUMN card_synced TEXT;
