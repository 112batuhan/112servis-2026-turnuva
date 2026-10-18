-- The hidden puzzle behind the navbar's trophy (handlers/arg.rs). One row per player per
-- stage they solved; `id` orders the solvers of a stage, so the first to solve stage 3
-- wins. A stage only counts once the one before it is solved.
CREATE TABLE arg_solves (
    id BIGSERIAL PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES osu_users(id) ON DELETE CASCADE,
    stage SMALLINT NOT NULL CHECK (stage BETWEEN 1 AND 3),
    solved_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (user_id, stage)
);

-- Every guess, right or wrong: the results page lists them, and the guess route counts a
-- player's last minute of them to rate-limit brute forcing.
CREATE TABLE arg_attempts (
    id BIGSERIAL PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES osu_users(id) ON DELETE CASCADE,
    stage SMALLINT NOT NULL,
    guess TEXT NOT NULL,
    correct BOOLEAN NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX arg_attempts_user_idx ON arg_attempts (user_id, created_at);
