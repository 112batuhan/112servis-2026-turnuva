-- Referees run matches. They sit between basic and map_pooler on the role ladder
-- (role.rs); the enum's own order doesn't matter.
ALTER TYPE user_role ADD VALUE 'referee' BEFORE 'basic';
