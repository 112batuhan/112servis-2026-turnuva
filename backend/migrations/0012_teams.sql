-- 2v2 teams. A team is created with both members when an invite is accepted;
-- user_id as the primary key keeps every player in at most one team.
CREATE TABLE teams (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE team_members (
    user_id UUID PRIMARY KEY REFERENCES osu_users(id) ON DELETE CASCADE,
    team_id UUID NOT NULL REFERENCES teams(id) ON DELETE CASCADE,
    joined_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX team_members_team_id_idx ON team_members (team_id);

CREATE TABLE team_invites (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    inviter_id UUID NOT NULL REFERENCES osu_users(id) ON DELETE CASCADE,
    invitee_id UUID NOT NULL REFERENCES osu_users(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (inviter_id, invitee_id),
    CHECK (inviter_id <> invitee_id)
);

CREATE INDEX team_invites_invitee_id_idx ON team_invites (invitee_id);
