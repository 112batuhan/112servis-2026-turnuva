-- The ARG's answers are no longer stages solved in order: any of the three messages can be
-- entered at any time, and the first to enter the third wins. `arg_solves` keeps one row per
-- player per message they entered (`id` still orders each message's finders), and an attempt
-- records the message it matched, null for a wrong one.
ALTER TABLE arg_solves RENAME COLUMN stage TO message;
ALTER TABLE arg_solves RENAME CONSTRAINT arg_solves_stage_check TO arg_solves_message_check;
ALTER TABLE arg_solves RENAME CONSTRAINT arg_solves_user_id_stage_key TO arg_solves_user_id_message_key;

ALTER TABLE arg_attempts RENAME COLUMN stage TO message;
ALTER TABLE arg_attempts ALTER COLUMN message DROP NOT NULL;
UPDATE arg_attempts SET message = NULL WHERE NOT correct;
