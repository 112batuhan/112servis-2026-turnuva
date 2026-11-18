-- The hosts' switch for the auto referee opening rooms by itself (10 minutes before each
-- match in auto mode) and the 20-minute reminder that promises it. Off until the hosts
-- turn it on; the auto referee still runs in rooms a referee opens.
ALTER TABLE app_settings ADD COLUMN auto_rooms BOOLEAN NOT NULL DEFAULT false;
