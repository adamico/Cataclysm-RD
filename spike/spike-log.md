# Fields spike — time log

Issue #14. Throwaway branch `prototype/fields-spike`, never merged. Times from
`date -Iseconds` (local, +01:00). Wall-clock includes build waits; waits are
called out in notes. Where agent work ran **while** a build compiled, both rows
are logged and overlap: the session total is the first start to the last end,
not the sum of the rows.

Lines produced count non-generated files written in that row (code + docs);
`rng_recording.cpp` is ~180 lines of BN's `rng.cpp` copied plus ~260 lines of hooks.

| session | kind of work | who | start | end | wall-clock | lines produced | notes |
|---|---|---|---|---|---|---|---|
| 1 | oracle and tests: fixed | agent | 2026-11-28T09:07:23+01:00 | 2026-11-28T09:08:25+01:00 | 1m02s | 0 | read issue #14, BN CMake options; configure Reference build (21 s of it a configure wait) |
| 1 | oracle and tests: fixed | agent | 2026-11-28T09:08:29+01:00 | 2026-11-28T09:13:57+01:00 | 5m28s | 0 | **wait**: first full Reference build (`ninja -j22`, 536 steps, 5m17s; 11 s lost to a missing `/usr/bin/time`) |
| 1 | oracle and tests: fixed | agent | 2026-11-28T09:08:40+01:00 | 2026-11-28T09:14:00+01:00 | 5m20s | ~600 | *overlaps the wait*: read `rng.*`, `process_fields_in_submap`, BN test harness; wrote `rng_recording.cpp`, `draw_recorder.h`, `fields_recorder.cpp`, `build.sh`, smoke layout, CASE/DUMP formats |
| 1 | oracle and tests: fixed | agent | 2026-11-28T09:14:07+01:00 | 2026-11-28T09:15:10+01:00 | 1m03s | ~5 | recorder link: fixed link-target name and a `pipefail`/`grep -q` false failure; first run: BN can't find `data/` (`USE_PREFIX_DATA_DIR` defaults ON) |
| 1 | oracle and tests: fixed | agent | 2026-11-28T09:15:25+01:00 | 2026-11-28T09:22:07+01:00 | 6m42s | 0 | **wait**: reconfigure `-DUSE_PREFIX_DATA_DIR=OFF` → global `-DPREFIX` changed → full rebuild |
| 1 | oracle and tests: fixed | agent | 2026-11-28T09:15:30+01:00 | 2026-11-28T09:21:30+01:00 | ~6m | ~330 | *overlaps the wait*: soak runner + summariser + seed set, `annotate_case.py`, Reference and recorder READMEs |
| 1 | oracle and tests: fixed | agent | 2026-11-28T09:22:16+01:00 | 2026-11-28T09:24:19+01:00 | 2m03s | ~10 | recorder rebuild (22 s), smoke run works but aborts at exit in BN's static teardown (Lua hook cache); gdb backtrace; `_Exit` after flushing |
| 1 | oracle and tests: fixed | agent | 2026-11-28T09:24:19+01:00 | 2026-11-28T09:26:39+01:00 | 2m20s | 0 | checks: case and dump byte-identical across runs; recording does not change the dump; soak 10 seeds × 100 turns (16 s); call-site annotation |
| 1 | oracle and tests: fixed | agent | 2026-11-28T09:26:39+01:00 | 2026-11-28T09:27:16+01:00 | 0m37s | ~40 | this log, samples, commit, push, issue comment |

## Session 1 totals

- Wall-clock: 09:07:23 → 2026-11-28T09:27:16+01:00 (19m53s).
- Of which build waits: 21 s configure + 5m17s first build + 6m42s rebuild +
  ~45 s of recorder builds ≈ 13 min. The rebuild (6m42s) was avoidable: it came
  from one wrong default (`USE_PREFIX_DATA_DIR`).
- Agent work not overlapped by a wait: ≈ 8 min.
- Lines produced: ~1,050 code (C++ 864, shell 96, Python 98, layout 16) + ~260 docs.

## Per-run costs (for the estimate)

| step | cost on this machine (24 threads) |
|---|---|
| configure | 21 s |
| full Reference build | 5m17s – 6m42s |
| recorder build (2 TUs + link of 737 MB binary) | ~22 s |
| recorder start-up (load `bn` Content) | ~13 s |
| Fields processing, 3×3 submaps, 100 turns | well under 1 s per seed |
