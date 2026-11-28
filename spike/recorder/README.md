# Recorder (C++ oracle, fixed part)

`fields_recorder` is a separate program built from the **untouched** BN sources
of the Reference build. It links every BN object except `rng.cpp.o` and
`main.cpp.o`; in their place it links:

- `rng_recording.cpp`: a recording copy of BN's `src/rng.cpp`. Same draw logic,
  plus hooks that write every helper-level draw (function, arguments, result,
  call site) to a case file. Format: [CASE_FORMAT.md](CASE_FORMAT.md).
- `fields_recorder.cpp`: a `main` that loads the core Content headlessly (the
  same steps as `init_global_game_state()` in BN `tests/test_main.cpp`), builds
  a region of submaps from a layout file, runs BN's own
  `process_fields_in_submap()` for N turns per seed, and writes the
  Observable dump ([DUMP_FORMAT.md](DUMP_FORMAT.md)) and, with `--record`, the
  case file.

It does not play a game: no UI, no player turns, no mapgen.

## Build

Needs a Reference build first (`spike/reference/README.md`).

```bash
spike/recorder/build.sh "$BN" "$BUILD" "$OUT"
```

The script takes BN's compile commands for `src/rng.cpp` and `src/main.cpp`
from `compile_commands.json` and BN's link command for `cataclysm-bn` from
`ninja -t commands`, and swaps only the two objects, so every flag matches the
Reference build. It fails if BN's `rng.cpp.o` would still be linked.

## Run

```bash
"$OUT/fields_recorder" run --bn "$BN" --user-dir "$SCRATCH/userdir" \
    --layout spike/soak/layouts/smoke.layout --seeds 42 --turns 20 \
    --out-dir "$SCRATCH/out" --record
```

`--user-dir` is **wiped** at start (like BN's test runner). `--bn` is the BN
source tree: Content is read from `$BN/data`.

Per seed: `seed-<s>.dump.jsonl` (snapshots at turn 0 and after the last turn)
and, with `--record`, `seed-<s>.case.jsonl`. Exit code 2 means BN logged an
error (`debug_has_error_been_observed()`).

Resolve call sites of the draws:

```bash
python3 -I spike/recorder/annotate_case.py "$OUT/fields_recorder" seed-42.case.jsonl --summary
```

## Layout file

```
region <w> <h>                               # submaps, default 1 1
ter all <ter_id>                             # default t_dirt
ter <x> <y> <ter_id>                         # region tile coords
field <x> <y> <fd_id> <intensity> <age_turns>
```

Give layout fields `age_turns >= 1`: BN skips effects on newborn (age 0) fields
for one tick.

Session 3 adds (each takes an optional trailing `z`, default 0):

```
zlevels <lo> <hi>                            # z-levels of the region
clock <turns>                                # start time = turn_zero + turns (default 3600)
furn <x> <y> <furn_id> [z]
item <x> <y> <itype_id> [z]                  # one item per line, spawned at the start time
rad <x> <y> <radiation> [z]
monster <x> <y> <mon_id>                     # creature case: runs map::creature_in_field in g->m
```

and the run options `--dump-every` (a snapshot after every turn) and
`--case <name>` (the case name in the header). Per-rule cases are recorded with
`spike/cases/record_cases.sh`; see `spike/cases/README.md`.

## Checking the copy against BN's rng.cpp

```bash
diff "$BN/src/rng.cpp" spike/recorder/rng_recording.cpp
```

The diff is the hooks only: each helper keeps its body, with the result held
in a local so it can be logged before returning.
