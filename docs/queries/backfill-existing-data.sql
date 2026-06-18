-- ============================================================================
-- Assign all pre-existing data to one account.
-- Run AFTER add-user-auth.sql and AFTER seeding the target user, e.g.:
--   cargo run --bin seed_user -- murtaza.hassani@onepointltd.com '<password>' Murtaza
--
-- Safe to re-run. If the user doesn't exist yet the subquery is NULL and the
-- UPDATEs are no-ops (they only touch rows that are still unowned).
-- ============================================================================

UPDATE datasets
   SET user_id = (SELECT id FROM users WHERE email = 'murtaza.hassani@onepointltd.com')
 WHERE user_id IS NULL;

UPDATE prompts
   SET user_id = (SELECT id FROM users WHERE email = 'murtaza.hassani@onepointltd.com')
 WHERE user_id IS NULL;

UPDATE evaluation_runs
   SET user_id = (SELECT id FROM users WHERE email = 'murtaza.hassani@onepointltd.com')
 WHERE user_id IS NULL;
