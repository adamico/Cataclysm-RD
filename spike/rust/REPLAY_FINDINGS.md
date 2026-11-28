# Replay findings (issue #14, session 3)

What the replay oracle found when it checked session 2's translation against
recorded BN behaviour. Replay unit (ADR 0004): one field entry on one tile in
one turn, with its neighbours. Each layout in `spike/cases/layouts` puts one
entry (or a few far apart) with the neighbours its rule reads. Turn 1 is that
unit; turns 2–5 replay the state that grew from it.

## Translation bugs

| # | rule | what was wrong | how the replay caught it | time to fix |
|---|---|---|---|---|
| 1 | `creature_in_field` field effects (`field_type::load`, `content.rs`) | `field_effect.is_environmental` defaulted to **true** when the JSON omits it, copied from the struct initializer in `field_type.h`. BN loads it with `optional(joe, was_loaded, "is_environmental", fe.is_environmental)`, which has no default, so `generic_factory.h:792` assigns `bool() = false`. Every BN field effect without the key (smoke, poison, relax gas, …) is therefore non-environmental. Rust took the `add_env_effect` branch and drew the duration a second time. | `creature_gases_veggy` turn 1, all 8 seeds: "Rust called `rng(7,7)`, BN called out of Fields to `Creature::add_effect` first". The first `rng(7,7)` (from `get_effect`) matched, then BN went straight to the plain `add_effect`. | **2m21s** (10:28:06 → 10:30:27), from the first failing run to green, including reading `generic_factory.h`. One line changed. |

**Fix rate: 1 translation bug in about 4,300 lines of session 2 Rust**, roughly
3,000 BN lines translated. It sits in content loading, not in a rule. Every
rule's branch order and draw order matched BN wherever a case reached it.

It was found because the replay compares draw *sequences*. Comparing outcomes
would not have caught it. The Combat stub records `add_env_effect` and returns
"applied", so the effect is applied either way; only the extra duration roll
shows. A statistical soak would have needed creatures to see it.

After the bug I checked the rest of that bug class: every `optional(...)`
call without a default in `field_type.cpp`, against its struct initializer.
`is_environmental` is the only one whose initializer isn't zero. No further bug.

## Not translation bugs (fixed on the oracle side, for the record)

These failed first and are harness or stub problems. They cost time, but they
say nothing about how far session 2 can be trusted.

| what failed | cause | fix |
|---|---|---|
| `fire_items`: `item::burn` inputs differ | harness: float sums recorded with `%.9g` were compared as `f64` | compare as `f32` |
| `fire_items`: `roll_remainder(0.40799999237060547)` vs `0.4079999923706054` | harness: `serde_json`'s default float parser is off by one ulp on some 17-digit inputs | `features = ["float_roundtrip"]` |
| `fire_container` (wide set, 3/64 seeds): item count 1 vs 2 | stub: BN's `create_burnproducts` pushes ash when a stick burns away. The Items stub made no products, and the function is file-local in `map_field.cpp`, so it can't be wrapped. | The `item::burn` wrapper computes the same products from the same materials (no draws) and records them. The stub pushes them. `Items::burnproducts` now takes `&mut self`. |
| `fire_container` (wide, 2/64): product weight 1 g vs 100 g | oracle: ash counts by charges; the recorder weighed one charge | weigh `charges_default()` charges |
| `creature_fire`, `creature_misc`: hp lost, BN 18 vs Rust 9 | stub: `onfire` burns the monster once inside `add_effect` (Combat side) | hp is checked only on turns with no `add_effect`/`deal_damage` |
| `creature_elec`: unexpected `add_effect(zapped)` | stub: BN's `deal_damage` is virtual (can't be wrapped at link time) and adds `zapped` itself | call marks now carry the caller's pc. A replayed `deal_damage` consumes the draws and calls whose call site isn't Fields code. |
| `creature_misc`: an extra Rust draw on turn 2 | stub: `fd_fatigue` teleported the monster in BN; the stub's `teleport` didn't move it | the `teleport` wrapper records where the monster landed; the stub moves it there |
| `creature_*_veggy`: BN has `fd_blood_veggy` / `fd_gibs_veggy` | stub: a damaged monster bleeds through monster code (Combat side) | creature dumps leave out blood and gibs that weren't in the layout |
| `creature_fire3`: recorder could not place `mon_bee` | oracle: `place_critter_at` refuses a dangerous tile | place the monster before the fields |
| `gas_spread`, `vents`: BN `debugmsg` | my layouts: invalid intensities (smoke 5, flame burst 3) | fixed the layouts |

## Coverage

- **48 replay tests**, one per layout:
  - 30 layouts for `process_fields_in_submap`, with 8 seeds × 5 turns each;
  - the smoke case (seed 42, 20 turns);
  - 17 creature layouts;
  - `smoke_dump_is_byte_exact` (deliverable 3);
  - `soak_case_replays_whole_run` (opt-in, 100 turns, 251,875 draws).

  All pass. The process-path layouts also pass the **64-seed × 8-turn wide set**
  (`cases/recorded-wide`, not committed).
- **Draw sites.** 72 of the 114 lines in `map_field.cpp` that call a dice helper
  have recorded draws (`python3 -I spike/cases/coverage.py "$BN"`).
  - `process_fields_in_submap`: 41 of its 47 sites have draws attributed to
    them. The other six are below.
  - `creature_in_field` / `monster_in_field`: 31 of 43 sites.
  - The rest are `player_in_field` and emit / hot air (gaps, below).

  The six process-path sites without attributed draws:
  - 1152 / 1158 / 1164 (fuel turning terrain to dirt or ash): their draws
    *are* recorded (90 draws), but they share one call instruction after the
    compiler merged them, so addr2line reports line 0 (`map_field.cpp:?`).
  - 1416 (`random_entry_removed(grounded)`) and 1560 (`random_entry(valid_dirs)`):
    their draws are recorded too (`rng.h:185`: 17, `rng.h:137`: 68). They go
    through out-of-line templates, whose caller isn't an inline frame.
  - 1530 (acid vent refill): dead code in BN, as `STUBS.md` already says.
- **Calls out of Fields answered from the case:**
  - `item::burn`, plus its burn products;
  - `Creature::add_effect`, both overloads;
  - `check_dead_state`, `add_env_effect`, `teleport::teleport`,
    `monster::make_fungus`.

  Their own draws (149 from `item.cpp:9438`, 80 from `creature.cpp:1392`,
  `teleport.cpp`, `mondeath.cpp`, …) are consumed, not replayed.

| rule (STUBS.md table) | replay test(s) | status |
|---|---|---|
| field types load, ids in load order | every test (Rust loads BN's JSON; turn-0 dumps match) | covered |
| `field_entry` / `field` accessors, add/remove/displayed | every test | covered |
| dead-entry cleanup, newborn suppression, has_fire | `fire_web` (web set to 0, then cleaned up), `decay` (age 0) | covered |
| intensity upgrade | `upgrade` (clock on the 6-hour boundary) | covered |
| underwater age speed-up | `fire_water` | covered |
| fire: items | `fire_items`, `fire_container`, `fire_items_spread`, `incendiary` | covered (item internals answered from the case) |
| fire: terrain/furniture fuel, ash/dirt | `fire_wall`, `fire_ash_ter`, `fire_furn` | covered |
| fire: z-rise / z-fall | `fire_zrise`, `fire_zfall` | covered |
| fire: flashpoint, growth, spread, pits, webs | `fire_grass`, `fire_growth`, `fire_pit`, `fire_web`, `fire_edge` | covered |
| gas spread: fall, 8 dirs, permeable, outdoor | `gas_spread`, `gas_fall`, `gas_edge`, `smoke` | covered |
| cross-submap spread, submap edges | `fire_edge` (corner of 4 submaps), `gas_edge`, `bees`, `wandering`, `vents` (shock bolts) | covered |
| fungicidal gas | `fungicidal` | covered |
| electricity (arc, ground, free) | `electricity` | covered |
| fire vent, flame burst, incendiary, shock vent, acid vent | `vents`, `incendiary` | covered (acid refill dead in BN) |
| gas remover | `gas_remover` (harmless fields only) | **partly**: the removal path crashes BN (see below) |
| push items, bees, radiation, wandering | `push_items`, `bees`, `radiation`, `wandering` | covered |
| half-life decay | every test, `decay` | covered |
| field cache compaction | every test (cache order decides draw order) | covered indirectly |
| `creature_in_field`, `monster_in_field` | 17 `creature_*` tests | covered for monsters; branches still without draws: vehicle chances (562/563), the smoke-3 moves roll (660), fungal haze / insecticidal paths (774–789) |
| `player_in_field`, `burn_body_part` | none | **gap** |
| `create_hot_air`, `emit_field`, `propagate_field`, `get_wind_blockers` | none | **gap** |
| `batch_turns_field` | none | Approximation, unchecked by agreement |

## Gaps, with reasons

- **`player_in_field` (≈ 330 BN lines).** Its queries touch a whole Character:
  traits, bionics, worn items and their coverage, armour per body part, env
  resist, arms, mutation category. The recorder's avatar is generated by BN
  (`character_type::NOW`) with random gear. A replay needs a `character_def`
  mark that answers about 15 query kinds per body part, plus wraps for
  `Character::hurtall`, `irradiate`, `get_random_body_part` (const, virtual?)
  and the 7-argument `add_env_effect`. Estimate: ~30 min on the pattern the
  monster cases set. Not done this session.
- **`create_hot_air`, `emit_field`, `propagate_field`.** Neither recorded entry
  point reaches them: BN skips hot air outside the bubble, and emits come from
  monsters, items and furniture. They need a third recorder mode that calls
  them directly on `g->m`. The rules draw little (lines 167, 835, 836, 870)
  and `propagate_field` has a smoke test against the libstdc++ heap order.
  ~20 min.
- **`get_wind_blockers`.** Draws nothing; it needs a no-draw equality test
  against BN's output.
- **`fd_gas_remover` removing gases.** BN's range-for erases and then
  increments the erased iterator. In the Reference build this **segfaults**
  (`std::_Rb_tree_increment` from `map_field.cpp:1474`, gdb backtrace;
  `cases/layouts-bn-crash/gas_remover_crash.layout`). There is no BN behaviour
  to compare against. Rust keeps the documented divergence (remove every match).
- **What replay can't see.** A wrong constant on the *result* side of a draw
  (`rng(1,100) < 25*(i-1)` turned into `24*(i-1)`) only fails if some
  recorded draw lands in the gap. Mutation test, `process.rs`:

  | mutation | caught? |
  |---|---|
  | ash chance `one_in(200 - 50i)` → `200 - 40i` | caught (`fire_furn`) |
  | gas spread start direction `start + c` → `start + c + 1` | caught (11 tests) |
  | spread chance 25 → 24 | **not caught** in 8 seeds |

  Wider seed sets shrink the gap but never close it. Flipping one recorded
  `one_in(3)` result in the busy soak case desyncs the replay at turn 34.

## Lead from session 1: smoke on a submap edge

"Smoke at (23,18) did not spread in 20 turns" is **not an edge bug**:

- Smoke spreads with `rng(1,100) <= 10`.
- BN's engine is `std::minstd_rand0`. Seeded with a small number it makes a
  tiny first output, so the first draw after seeding is near its minimum:
  `rng(1,100) = 1` in all 105 recorded runs that start with one.
- In the smoke layout, (17,17) is first in the field cache and gets that draw,
  so it spreads at turn 1. (23,18) gets ordinary draws and, with seed 42,
  missed a 10 % chance 20 times running (p ≈ 0.12).
- In the `gas_edge` layout the edge smoke comes first, spreads in 64 of 64
  seeds, and crosses into the east submap in 33 of 64.
- Rust replays all of it exactly.

The engine itself is the real surprise. Session 2's notes assumed mt19937; it
is a 31-bit LCG, and small seeds give correlated streams (`SOAK_RESULT.md`).
