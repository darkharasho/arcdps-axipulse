# Test fixtures

- `wvw.zevtc` — an anonymized mid-size WvW fight, shared byte-for-byte
  with `axipulse/tests/fixtures/wvw.zevtc` so both products are validated
  against the same fight.
- `wvw.ei.json` — Elite Insights output for that log, frozen before the
  migration. It is the equality oracle: each `FightData` field family is
  migrated by asserting the native computation matches this. **Delete it,
  and `ei_model.rs`, in the final migration commit.**
