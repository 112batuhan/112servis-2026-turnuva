-- Every chat line said in an osu! multiplayer room's channel (#mp_<mp_id>), kept for
-- moderation. Keyed by the room rather than a match, so the lines sent while a room
-- is being set up, before its link is saved to the match, are kept too.
CREATE TABLE room_chat_messages (
    id BIGSERIAL PRIMARY KEY,
    mp_id BIGINT NOT NULL,
    sent_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    sender TEXT NOT NULL,
    text TEXT NOT NULL,
    -- A /me line.
    action BOOLEAN NOT NULL DEFAULT false,
    -- The referee whose IRC connection saw the line.
    logged_by UUID REFERENCES osu_users(id) ON DELETE SET NULL
);

CREATE INDEX room_chat_messages_mp_id_idx ON room_chat_messages (mp_id, sent_at);
