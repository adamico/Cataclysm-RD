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

## Session 2: mechanical translation + untangling/design

Rust workspace `spike/rust/` (`bn_dice`, `fields`, `bubble_stub`, `combat_stub`);
coverage and stubs in `spike/rust/STUBS.md`. Lines are `wc -l` of the Rust
files as written (not run through `rustfmt`; ~110-column lines). Two windows had
no timestamp between files; there the wall-clock is **apportioned by lines
written** and marked `~`, and both rows show the shared window.

| session | kind of work | who | start | end | wall-clock | lines produced | notes |
|---|---|---|---|---|---|---|---|
| 2 | reading (shared by both kinds) | agent | 2026-11-28T09:28:37+01:00 | 2026-11-28T09:32:52+01:00 | 4m15s | 0 | issue #14 + session 1 comment, this log, CASE/DUMP formats; BN `map_field.cpp` (all 1,692 lines), `field.*`, `field_type.*`, `submap_fields.h`, `batch_turns.cpp`, plus what they call: `rng.cpp`/`rng.h`, `action_time_scale.h`, `calendar.cpp`, `map.cpp`/`mapbuffer.cpp` field ops, `fire.h`, `emit.h`, `field_type.json` |
| 2 | untangling/design | agent | 2026-11-28T09:32:52+01:00 | 2026-11-28T09:36:30+01:00 | ~1m09s (of 3m38s) | 430 | workspace + crate split; `storage.rs` (field storage pulled out of `submap`: grid, `field_count`, `field_cache`), `ports.rs` (7 ports: SubmapStore, BubbleCaches, BubbleMap, MapData, Items, Combat, Clock) |
| 2 | mechanical translation | agent | 2026-11-28T09:32:52+01:00 | 2026-11-28T09:36:30+01:00 | ~2m29s (of 3m38s) | 925 | `bn_dice` (rng.cpp helpers as a `Dice` trait, 205), `content.rs` (field_type JSON load in int-id order, 538), `field.rs` (field.cpp, 182) |
| 2 | mechanical translation | agent | 2026-11-28T09:36:30+01:00 | 2026-11-28T09:40:30+01:00 | 4m00s | 898 | `process.rs`: `process_fields_in_submap` and every per-type rule, helpers, cache compaction |
| 2 | mechanical translation | agent | 2026-11-28T09:40:30+01:00 | 2026-11-28T09:43:25+01:00 | 2m55s | 1,180 | `creatures.rs` (player/monster/creature_in_field, 703), `map_rules.rs` (hot air, emit, propagate, wind blockers, map-level add/mod field, 252), `heap.rs` (libstdc++ heap, 94), `batch.rs` (Approximation, opt-in, 101), `lib.rs` (30) |
| 2 | mechanical translation | agent | 2026-11-28T09:43:25+01:00 | 2026-11-28T09:43:46+01:00 | 0m21s | 0 | **wait** first `cargo build` (3 s, serde_json from the offline cache); compiled first time with 1 warning, fixed |
| 2 | untangling/design | agent | 2026-11-28T09:43:46+01:00 | 2026-11-28T09:44:35+01:00 | 0m49s | 388 | `bubble_stub`: `Region` (per-submap storage + neighbour lookup), terrain catalogue, items, caches, bubble-map geometry |
| 2 | untangling/design | agent | 2026-11-28T09:44:35+01:00 | 2026-11-28T09:46:09+01:00 | ~0m40s (of 1m34s) | 238 | `combat_stub`: `RecordingCombat` records every sink, does nothing else |
| 2 | tests (smoke) | agent | 2026-11-28T09:44:35+01:00 | 2026-11-28T09:46:24+01:00 | ~1m09s | 289 | 11 smoke tests; libstdc++ heap tie order captured from a clang++ program; 1 wrong test expectation (bees) fixed, the code was right |
| 2 | untangling/design (docs) | agent | 2026-11-28T09:46:24+01:00 | 2026-11-28T09:48:01+01:00 | 1m37s | ~100 docs | `STUBS.md`: rule coverage table, stub per port, divergences and quirks |
| 2 | log, commit, push, comment | agent | 2026-11-28T09:48:01+01:00 | 2026-11-28T09:48:45+01:00 | 0m44s | ~70 docs | this section, commit, push, issue comment |

## Session 2 totals

- Wall-clock: 09:28:37 → 2026-11-28T09:48:45+01:00 (20m08s). Waits: ~5 s of cargo builds and test
  runs in all; nothing else waited.
- By kind (agent wall-clock, Rust lines):

  | kind | time | Rust lines | lines/hour |
  |---|---|---|---|
  | reading (shared) | 4m15s | 0 | — |
  | mechanical translation | ~9m45s | 3,003 | ~18,500 |
  | untangling/design | ~2m38s code + 1m37s docs | 1,056 + ~100 docs | ~24,000 (code time only) |
  | tests (smoke) | ~1m09s | 289 | — |

  Charging the reading to translation: ~3,000 Rust lines in ~14 min, from about
  3,000 BN lines (the Fields files incl. `batch_turns.*`) plus ~250 BN lines of
  `map.cpp`/`mapbuffer.cpp` field ops ≈ **14,000 BN lines/hour**. This is an upper bound: no
  replay check yet (session 3) and no review yet (session 4).
- The Tangle ("field storage lives in `submap`"): ~2 min of explicit work
  (`storage.rs`, the `SubmapStore` port, `Region`) plus a per-access cost spread
  through `process.rs` (every `cur.` becomes a lookup by (submap, tile, type)).
  Awkward parts:
  1. BN holds `field_entry&` (`cur`, `dstfld`, `elec`) across calls that may touch
     the same tile, including `cur` itself (electricity at `dx = dy = 0`). Rust
     can't hold two `&mut` into the store, so every access goes back through it.
  2. BN iterates a tile's `std::map` while rules insert into it (wandering fields
     on their own tile) and erase from it. Rust walks it by "next key after the
     current one", which matches `std::map` iterator semantics.
  3. Items live on `submap` too; they became a separate port addressed by
     (submap, tile, index) with opaque detached-item handles.
  4. The bubble-map rules need `&mut` field storage **and** terrain reads for
     `impassable`/`permeable`; with both on one `Region` the borrows clash, so the
     stub `BubbleMap` uses hand-set point sets. A real port should put those
     queries on `SubmapStore`, or split terrain from fields.
  5. `field_count` is kept by hand by every rule (and not kept by
     `fd_gas_remover`); it stays a public counter on `SubmapFields` so the
     same bugs reproduce.
  6. `floor_cache` and `is_uniform` are bubble-map caches stored on `submap`
     that Fields reads or clears; they became port methods.

## Session 3: oracle and tests, Fields part

The Reference build from session 1 was still on disk (another session's
scratchpad), so nothing was rebuilt. The recorder was rebuilt six times into
this session's scratchpad (18–68 s each). Kinds:
- **fixed**: oracle machinery that any context would reuse (recorder
  directives, link-time wraps, the case reader, `ReplayDice`, dump writer,
  world builder, soak runner and comparison, coverage);
- **Fields**: layouts, per-rule tests, the Fields-specific stub answers and
  chasing failures;
- **fix**: changing the translation.

Rows that ran during a wait are marked *overlaps*.

| session | kind of work | who | start | end | wall-clock | lines produced | notes |
|---|---|---|---|---|---|---|---|
| 3 | reading (shared) | agent | 2026-11-28T09:50:12+01:00 | 2026-11-28T09:53:20+01:00 | 3m08s | 0 | issue #14 plan and both session comments, this log, `STUBS.md`, CASE/DUMP formats, recorder sources, Rust ports and `process.rs` structure |
| 3 | oracle and tests: fixed | agent | 2026-11-28T09:53:20+01:00 | 2026-11-28T09:57:39+01:00 | ~2m40s (of 4m19s) | ~330 C++ | layout directives (z-levels, furn, item, rad, clock), `ter_def`/`item_def`/`tile` marks, `--dump-every`, `item::burn` wrapped with `-Wl,--wrap`; 2 recorder builds (18 s, 36 s; one link fix: the flag landed after `&& :`) |
| 3 | oracle and tests: Fields | agent | 2026-11-28T09:53:20+01:00 | 2026-11-28T09:57:39+01:00 | ~1m40s (of 4m19s) | 230 layout | 26 per-rule layouts |
| 3 | oracle and tests: Fields | agent | 2026-11-28T09:57:45+01:00 | 2026-11-28T10:05:08+01:00 | 7m23s | 0 | **wait**: recording 26 layouts × 8 seeds × 5 turns (8 in parallel; slow because `annotate_case.py` ran one addr2line per pc on a 737 MB binary) |
| 3 | oracle and tests: fixed | agent | 2026-11-28T09:57:45+01:00 | 2026-11-28T10:02:42+01:00 | ~5m | ~620 Rust | *overlaps the wait*: crate `fields_harness` (layout, case, `ReplayDice`, world, dump), replay test driver, `Dice::foreign` hook, Items stub replay |
| 3 | oracle and tests: Fields | agent | 2026-11-28T10:02:42+01:00 | 2026-11-28T10:05:54+01:00 | 3m12s | ~40 | first replay runs: smoke passes 20 turns byte-exact; 2 harness bugs (f32 compare, serde float round trip); `smoke_dump_is_byte_exact`; 3-mutant check (2 caught) |
| 3 | oracle and tests: Fields | agent | 2026-11-28T10:05:54+01:00 | 2026-11-28T10:07:34+01:00 | 1m40s | ~25 layout | BN segfaults in the gas remover (gdb: `_Rb_tree_increment` at `map_field.cpp:1474`), two layouts with invalid intensities fixed; re-record (**wait** 39 s) |
| 3 | oracle and tests: fixed | agent | 2026-11-28T10:07:34+01:00 | 2026-11-28T10:12:31+01:00 | 4m57s | ~110 | `coverage.py`; `annotate_case.py`: one addr2line for all pcs, inline frames (`-i`), innermost game frame; **waits** 1m28s + 1m42s re-annotating 209 cases; 2 more layouts (record 39 s) |
| 3 | oracle and tests: fixed | agent | 2026-11-28T10:12:31+01:00 | 2026-11-28T10:17:30+01:00 | ~5m | ~260 | soak: Rust `soak` bin, `compare_soak.py` (exact permutation + Holm), BN soak runs (19 s, 23 s), busy layout, whole-run replay of a 251,875-draw case (0.5 s) and a corrupted-case check; 64-seed wide recording started in the background |
| 3 | oracle and tests: fixed | agent | 2026-11-28T10:17:30+01:00 | 2026-11-28T10:20:46+01:00 | 3m16s | ~260 C++ | recorder bubble mode (creature cases in `g->m`, `creature_def`/`creature` marks) and 6 Combat-side wraps; build 39 s, compiled first time |
| 3 | oracle and tests: Fields | agent | 2026-11-28T10:20:46+01:00 | 2026-11-28T10:22:39+01:00 | 1m53s | 70 layout | 10 creature layouts; first creature record (**wait** 1m23s) |
| 3 | oracle and tests: Fields | agent | 2026-11-28T10:22:39+01:00 | 2026-11-28T10:28:06+01:00 | 5m27s | ~300 Rust, ~50 C++ | `ReplayCombat`; wide set shows missing burn products (stub gap): computed in the `item::burn` wrap, pushed by the stub; build 52 s; item re-record (2m33s, overlapped) |
| 3 | mechanical translation, fix | agent | 2026-11-28T10:28:06+01:00 | 2026-11-28T10:30:27+01:00 | 2m21s | 3 | **translation bug 1**: `field_effect.is_environmental` default (see `rust/REPLAY_FINDINGS.md`); same bug class checked across the loader, no more |
| 3 | oracle and tests: Fields | agent | 2026-11-28T10:30:27+01:00 | 2026-11-28T10:32:48+01:00 | 2m21s | ~60 | onfire hp and `zapped` inside virtual `deal_damage` (stub side); call marks carry pcs (fixed part); build 1m08s |
| 3 | oracle and tests: Fields | agent | 2026-11-28T10:32:48+01:00 | 2026-11-28T10:38:07+01:00 | 5m19s | 0 | **wait**: creature re-record (CPU shared with the wide run) |
| 3 | oracle and tests: Fields | agent | 2026-11-28T10:38:07+01:00 | 2026-11-28T10:39:50+01:00 | 1m43s | ~30 | bleeding (fd_blood/gibs from monster code), teleport position, monster placed before dangerous fields; build 33 s |
| 3 | oracle and tests: fixed (docs) | agent | 2026-11-28T10:39:50+01:00 | 2026-11-28T10:43:16+01:00 | 3m26s | ~80 docs | *overlaps the* **wait** (creature re-record 3m26s): CASE/DUMP/recorder README additions, `cases/README.md` |
| 3 | oracle and tests: Fields | agent | 2026-11-28T10:43:16+01:00 | 2026-11-28T10:46:28+01:00 | 3m12s | 50 layout | full `cargo test`, coverage; 7 more creature layouts (record **wait** 2m23s) |
| 3 | oracle and tests: Fields | agent | 2026-11-28T10:46:28+01:00 | 2026-11-28T10:55:55+01:00 | 9m27s | ~5 | **wait** for the wide set (to 10:50:43); wide replay: product weight by charges (oracle side); build 18 s; re-record items 8 + 64 seeds (**wait** 4m27s) |
| 3 | oracle and tests: Fields | agent | 2026-11-28T10:55:55+01:00 | 2026-11-28T10:58:12+01:00 | 2m17s | 0 | session 1 lead: not an edge bug; BN's engine is `minstd_rand0`, and the first draw after a small seed is near its minimum |
| 3 | oracle and tests: Fields (docs) | agent | 2026-11-28T10:58:12+01:00 | 2026-11-28T11:00:14+01:00 | 2m02s | ~260 docs | `SOAK_RESULT.md`, `REPLAY_FINDINGS.md`, `STUBS.md`, this log |

## Session 3 totals

- Wall-clock: 09:50:12 → 11:00:14 (70m02s) for the work rows, the longest session. Commit,
  push and comment come after the last row.
- **Waits not overlapped by work: ≈ 27 min.**
  - recordings and re-annotation: ≈ 22m40s;
  - recorder builds: 4m37s over 8 builds;
  - most of it is BN start-up (~13 s per process), CPU shared between
    parallel recordings, and one addr2line process per case.
- Agent time by kind (approximate where a window held both kinds):

  | kind | agent time | lines |
  |---|---|---|
  | reading (shared) | 3m08s | 0 |
  | oracle and tests: fixed (machinery) | ≈ 23 min, 8 of them during waits | ≈ 600 C++ (recorder), ≈ 1,330 Rust (`fields_harness` without the test file), ≈ 280 Python/shell, ≈ 80 docs |
  | oracle and tests: Fields (layouts, tests, stub answers, failures) | ≈ 27 min | 389 layout lines (44 layouts), ≈ 250 Rust (tests), ≈ 50 C++, ≈ 260 docs |
  | mechanical translation, fix | **2m21s** | 3 |

- **Result.**
  - 48 replay tests, all passing; the 64-seed wide set passes too.
  - 72 of 114 draw sites in `map_field.cpp` hit, and 41 of the 47 in
    `process_fields_in_submap`.
  - **One translation bug** in session 2's ~4,300 Rust lines, fixed in 2m21s.
  - 10 harness, stub or layout problems fixed on the oracle side.
  - Soak: 0 of 168 comparisons differ (target: 1.25 SD per test, ≈ 1.9 SD
    after Holm).
  - Gaps: `player_in_field`, emit / propagate / hot air / wind blockers, the
    gas remover's removal path (BN crashes).
- For the estimate: the Fields part of the oracle took about as long as the
  translation itself (≈ 27 min against session 2's ≈ 14). Waits added as much
  again. Replay found one bug, so session 2's speed holds up *where the cases
  reach*. Unchecked: `player_in_field`, about 330 BN lines and the largest
  single function.

## Session 4: review

| session | kind of work | who | start | end | wall-clock | lines produced | notes |
|---|---|---|---|---|---|---|---|
| 4 | review | user | 2026-11-28T11:08:07+01:00 | 2026-11-28T11:31:28+01:00 | 23m21s | 0 | read `STUBS.md`, `REPLAY_FINDINGS.md` and the `fields` crate next to BN |

Review findings (code quality, not behaviour; none are equivalence bugs):

1. **Comments stand in for structure.** The port needs a code convention: a passage that needs a comment gets refactored instead, and decisions go in ADRs, not in code comments.
2. **`creatures.rs` has a lot of duplication.** It needs a proper code review and a refactor pass.
3. **`map_rules.rs` duplicates functions**, for example `mb_add_field` and `add_field`.

For the estimate: review ran at about 4,300 Rust lines in 23m21s (≈ 11,000 lines/hour) for a read-through. Its findings mean the translated code needs a **refactor pass** that the spike didn't measure. Mechanical translation that copies BN's structure comes out duplicated and over-commented, and the replay tests are what make that refactor safe.

## Spike totals

| session | kind | who | wall-clock |
|---|---|---|---|
| 1 | oracle and tests: fixed | agent | 19m53s (≈ 13 min of build waits) |
| 2 | mechanical translation + untangling/design | agent | 20m08s |
| 3 | oracle and tests: fixed + Fields, translation fix | agent | ≈ 70 min (≈ 27 min of waits) |
| 4 | review | user | 23m21s |
