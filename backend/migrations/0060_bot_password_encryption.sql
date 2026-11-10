-- The bots' IRC passwords are kept encrypted with BOT_SECRET_KEY (backend/src/secret.rs):
-- ChaCha20-Poly1305, a random nonce per password. The plaintext column stays only for the
-- rows saved before this: the backend encrypts them at startup once the key is set, and
-- empties the column.
ALTER TABLE bancho_bots
    ALTER COLUMN irc_password DROP NOT NULL,
    ADD COLUMN password_nonce BYTEA,
    ADD COLUMN password_ciphertext BYTEA,
    ADD CONSTRAINT bancho_bots_password_check CHECK (
        irc_password IS NOT NULL
        OR (password_nonce IS NOT NULL AND password_ciphertext IS NOT NULL)
    );
