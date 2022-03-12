-- Add up migration script here
BEGIN;
ALTER TABLE subscriptions ADD COLUMN status TEXT NULL;

-- Backfill `status` for historical entries
UPDATE subscriptions
SET status = 'confirmed'
WHERE status IS NULL;
ALTER TABLE subscriptions ALTER COLUMN status SET NOT NULL;
COMMIT;
