-- ============================================================================
-- Per-user accounts + data ownership.
-- Idempotent: safe to run on a fresh or existing database.
-- Run this, then seed a user (cargo run --bin seed_user -- <email> <pw> <name>),
-- then run backfill-existing-data.sql to assign current data to that user.
-- ============================================================================

CREATE TABLE IF NOT EXISTS users (
    id            VARCHAR(64)  PRIMARY KEY,            -- format: u_{unix_micros}
    email         VARCHAR(320) UNIQUE NOT NULL,
    password_hash TEXT         NOT NULL,               -- argon2 PHC string
    name          VARCHAR(255),
    created_at    TIMESTAMPTZ  DEFAULT NOW()
);

-- Owner columns. Nullable on purpose: existing rows have no owner until the
-- backfill runs, and a NULL owner matches no user, so legacy rows stay hidden
-- rather than leaking.
ALTER TABLE datasets        ADD COLUMN IF NOT EXISTS user_id VARCHAR(64) REFERENCES users(id);
ALTER TABLE prompts         ADD COLUMN IF NOT EXISTS user_id VARCHAR(64) REFERENCES users(id);
ALTER TABLE evaluation_runs ADD COLUMN IF NOT EXISTS user_id VARCHAR(64) REFERENCES users(id);

CREATE INDEX IF NOT EXISTS idx_datasets_user  ON datasets(user_id);
CREATE INDEX IF NOT EXISTS idx_prompts_user   ON prompts(user_id);
CREATE INDEX IF NOT EXISTS idx_eval_runs_user ON evaluation_runs(user_id);
