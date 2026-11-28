# Fields spike (issue #14) — throwaway, never merged

| path | what |
|---|---|
| `spike-log.md` | time log, per kind of work |
| `reference/` | headless Reference build of BN e7b4976: exact commands |
| `recorder/` | C++ recorder: recording `rng.cpp` copy, headless Fields driver, Observable dump; case and dump formats |
| `soak/` | Fields-only soak runner (C++ side), fixed seed set, layouts |
| `samples/` | smoke case + dump (seed 42, 20 turns), call sites of its draws, soak summary (10 seeds × 100 turns) |

End to end, from nothing:

```bash
# see reference/README.md for $BN (Baseline clone) and $BUILD
spike/recorder/build.sh "$BN" "$BUILD" "$OUT"
"$OUT/fields_recorder" run --bn "$BN" --user-dir "$SCRATCH/userdir" \
    --layout spike/soak/layouts/smoke.layout --seeds 42 --turns 20 --out-dir "$SCRATCH/smoke" --record
spike/soak/run_soak.sh "$OUT/fields_recorder" "$BN" "$SCRATCH/soak"   # 10 seeds × 100 turns
```
