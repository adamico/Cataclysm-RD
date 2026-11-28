# Recorded Fields cases (issue #14, session 3)

- `layouts/*.layout`: one layout per Fields rule (or rule group). Each puts one
  field entry, or a few far enough apart, with the neighbours the rule reads
  (terrain, furniture, items, fields next door, z-levels, submap edges).
  Layouts with a `monster` line are **creature cases**: they run in the bubble
  map and replay `map::creature_in_field` → `monster_in_field` instead of
  `process_fields_in_submap`.
- `recorded/<layout>/seed-<s>.{case,dump}.jsonl`: seeds 1–8, 5 turns, `--record
  --dump-every`, call sites resolved (`src`). The smoke layout is recorded at
  seed 42 for 20 turns. Replayed by `spike/rust/fields_harness/tests/replay.rs`
  (one test per layout).
- `recorded-wide/` (not committed, ~150 MB): seeds 101–164, 8 turns, for the
  process-path layouts. Replay with
  `REPLAY_CASES=$PWD/spike/cases/recorded-wide cargo test --release -p fields_harness --test replay`.
- `layouts-bn-crash/gas_remover_crash.layout`: BN segfaults on it (see
  `spike/rust/REPLAY_FINDINGS.md`). Not replayable.
- `coverage.py`: draw-site coverage of the recorded cases against
  `src/map/map_field.cpp`.

Record (needs the recorder from `spike/recorder/build.sh`):

```bash
spike/cases/record_cases.sh "$OUT/fields_recorder" "$BN"            # all layouts
SEEDS=42 TURNS=20 spike/cases/record_cases.sh "$OUT/fields_recorder" "$BN" smoke
```

Coverage:

```bash
python3 -I spike/cases/coverage.py "$BN"
```
