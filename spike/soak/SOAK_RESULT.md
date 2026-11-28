# Fields-only soak run: result (issue #14, session 3)

**Setup.** Both sides start from the same layout, run 100 turns for each of the
fixed seeds in `seeds.txt` (1, 2, 3, 5, 8, 13, 21, 34, 55, 89), and write
`bn-fields-dump/1` snapshots every turn.

- C++: `run_soak.sh <recorder> <bn> <out> <layout> 100 seeds.txt --dump-every`,
  BN's own `process_fields_in_submap` with BN's engine.
- Rust: `cargo run --release -p fields_harness --bin soak -- <layout> <defs-case> seeds.txt 100 <out> --dump-every`.
  The dice are **live**: `BnDice` over SplitMix64, not a replay and not BN's
  stream. Terrain flags come from a recorded case of the same layout.
- Comparison: `compare_soak.py` (method and detection target in its docstring).
  For each checked turn (25, 50, 100), field type and metric (tiles, sum of
  intensity, mean intensity, mean age), each seed is one value per side. A
  two-sided exact permutation test on the difference of means (all C(20,10)
  splits) runs per comparison, with Holm-Bonferroni over the family at α = 0.05.

**Detection target.** With 10 seeds per side the run is meant to detect, with
about 80 % power, a shift of **1.25 pooled between-seed SDs** in any one metric
at α = 0.05. After Holm's correction the first comparison needs **≈ 1.9 SD**.
The per-row "detectable" column gives that in the metric's own units. Smaller
differences pass unseen.

## Results

| layout | comparisons | differ (Holm) | smallest p | metrics with seed-to-seed spread |
|---|---|---|---|---|
| `layouts/smoke.layout` (session 1's) | 60 | **0** | 0.064 | 19 of 60 |
| `layouts/busy.layout` (fire on flammable floor, 5 gases, bees, electricity, shock vent, gas vent; ~250k BN draws per seed) | 108 | **0** | 0.020 (uncorrected) | 77 of 108 |

Tables: `results/smoke-100turns.compare.md`, `results/busy-100turns.compare.md`.
Per-type summaries at turn 100: `results/busy-100turns.{bn,rust}.summary.tsv`.

**Verdict: pass, with stated power.** Nothing differs at the stated target. What
that rules out on the busy layout at turn 100, for example: a change of ±16
bee tiles (out of 1,208), ±4 fire tiles (of 23), ±85 turns of mean fire age, or
±0.07 in mean fire intensity.

**What has no power.** The smoke layout is nearly deterministic:
- 41 of its 60 metrics have zero spread across seeds. Its fire sits on dirt
  and never burns.
- Blood, the fire and toxic gas barely move.

So the smoke soak checks little beyond "the same thing happens". The busy
layout was added for that reason. Its smoke and electricity rows are thin too:
the type is absent in some runs (n 5/3, 7/8), and the tests on those rows
can't detect much.

**Caveats.**
- **BN's seeds are not independent streams.** BN's engine is `std::minstd_rand0`
  (`rng.h:26`), an LCG with x₁ = 16807·s mod (2³¹−1). With the small soak seeds:
  - the first draw is near the bottom of its range in every run: `rng(1,100)`
    returned 1 in all 105 recorded runs whose first draw it was;
  - later outputs are `c_k·s mod m` for a fixed `c_k`, so runs with different
    seeds are lattice-correlated.

  The test treats seeds as independent, so its p-values are approximate on the
  BN side. The Rust side mixes its seeds.
- **What the soak can't model.** Items and creatures are not in the soak
  layouts: in live mode their stubs do nothing.
- **What a pass doesn't prove.** A pass here says the distributions agree at
  the stated power. The exact check is the replay. The busy layout's seed 1 was
  also recorded (251,875 draws) and replays exactly over all 100 turns, with
  every dump byte-identical (`soak_case_replays_whole_run`).
