-- Stages are drafts until published; only published stages appear on the public
-- (unauthenticated) map pool page.
ALTER TABLE stages ADD COLUMN published BOOLEAN NOT NULL DEFAULT false;
