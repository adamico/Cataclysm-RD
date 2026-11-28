# Engine calibration: BN v0.13.0 and CDDA 0.H

This is the calibration pass from the [Engine scoring rubric](engine-scoring-rubric.md#calibration), the first step of [Which Baseline: Bright Nights v0.13.0 or CDDA 0.H-RELEASE?](https://github.com/adamico/Cataclysm-RD/issues/9). It reads the two analyses side by side, the [BN report](https://github.com/adamico/Cataclysm-RD/blob/research/bn-engine-analysis/docs/research/bn-engine-analysis.md) and the [CDDA report](https://github.com/adamico/Cataclysm-RD/blob/research/cdda-engine-analysis/docs/research/cdda-engine-analysis.md). It aligns the scores where the anchors were applied unevenly, and it recounts both Engines with one shared split rule. Date: 2026-11-27.

## 1. One split rule, recounted

The rubric's first Frontend rule ("includes a curses header") misfired on both Baselines. About 80 core files include `cursesdef.h` only for the `catacurses::window` handle type, among them `map.cpp`, `game.cpp`, `monster.cpp` and `character.cpp`. Each analysis fixed this its own way, so their Engine totals did not compare: BN gave 860 files and 455k lines, CDDA 636 files and 422k lines. The rubric now has one rule (see its _Engine, Frontend and Content split_), and it was applied mechanically to both clones.

| Split (files / `wc -l` lines / comment-stripped code) | BN v0.13.0              | CDDA 0.H                |
| ----------------------------------------------------- | ----------------------- | ----------------------- |
| Engine                                                | 774 / 422,072 / 333,829 | 639 / 426,867 / 339,618 |
| Frontend                                              | 167 / 95,261 / 76,199   | 159 / 99,356 / 78,861   |
| Content                                               | 3,165 / 1,064,216       | 6,561 / 1,515,178       |
| Vendored (excluded)                                   | 73 / 66,559             | 26 / 21,577             |

| Engine context (files / lines) | BN            | CDDA          |
| ------------------------------ | ------------- | ------------- |
| Creatures                      | 193 / 136,704 | 198 / 162,348 |
| Terrain & Map                  | 138 / 93,559  | 81 / 67,722   |
| Fields                         | 17 / 4,325    | 15 / 4,474    |
| Items                          | 97 / 66,719   | 73 / 71,151   |
| Vehicles                       | 36 / 20,992   | 22 / 23,219   |
| Time & Weather                 | 10 / 4,106    | 10 / 4,134    |
| Unclaimed                      | 283 / 95,667  | 240 / 93,819  |

- **Context assignment.** 21 file stems exist in both Engines but had been assigned to different contexts. All 21 now follow CDDA's assignment, so every move is on the BN side:
  - combat and digestion (`ballistics`, `projectile`, `dispersion`, `consumption`, `stomach`) go from Items to Creatures;
  - `mission*` and `sounds` go from Unclaimed to Creatures;
  - `condition`, `diary`, `kill_tracker` and `memorial_logger` go to Unclaimed;
  - `fungal_effects` goes to Fields, `tileray` to Vehicles and `mapsharing` to Unclaimed.
- **BN-only Engine code**, with no CDDA counterpart:
  - GPU compute (`compute/`, `shaders/`): 25 files, 8,600 lines, in Terrain & Map;
  - Lua bindings (`catalua*`): 63 files, 15,894 lines, in Unclaimed;
  - the thread pool: 2 files, 352 lines;
  - the distribution and fluid grids: 4 files, 3,138 lines.

  Together these are 27,984 lines. Without them, BN's Engine is about 394k lines against CDDA's 427k.

- **Line counts include comments.** `cloc` was not available, so lines are `wc -l` counts. The code column strips `//` and `/* */` comments.

## 2. Score changes

Each change aligns one application of an anchor with the other report's application, given the same evidence. A few contested facts were checked again in the clones: `processors_for_type` (BN 0, CDDA 3), `fd_*` equality tests in `map_field.cpp` (BN 45, CDDA 47), `temperature_cache` (present in both), and `friend` counts (`game.h` BN 15 / CDDA 24, `map.h` 6 / 14, `vehicle.h` 1 / 8).

| Baseline | Context        | Criterion | From → to | Reason                                                                                                                                                                                                                                     |
| -------- | -------------- | --------- | --------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| BN       | Creatures      | M1        | 2 → 3     | Same evidence as CDDA (home classes exist, rules leak into `game.cpp` and `map_field.cpp`). That is the level-3 anchor almost word for word.                                                                                               |
| BN       | Terrain & Map  | Q4        | 2 → 1     | Same blocking leaks as CDDA: `submap` owns the vehicles, the items and the fields, and the level cache holds raw `vehicle*`. Having fewer `friend`s (6 against 14) doesn't lift the score out of "large refactor first".                   |
| BN       | Fields         | M3        | 3 → 2     | Fields' own globals are low in both Baselines only because processing is written as `map::` members. Other contexts call `add_field`/`get_field*` freely in both.                                                                          |
| BN       | Items          | Q2        | 2 → 1     | About 3.4k public declarations, plus the ownership protocol. CDDA scored 1 on about 3.2k with hidden rules, and both sit at the anchor's "hundreds or more, with hidden rules".                                                            |
| BN       | Vehicles       | M2        | 3 → 2     | The same naming problem as CDDA: JSON `vehicle_part` is C++ `vpart_info`, while C++ `vehicle_part` is the installed part. The main concept is misnamed, not a secondary one.                                                               |
| BN       | Vehicles       | Q4        | 2 → 1     | Same facts as CDDA: `submap` owns the vehicles, the map moves them and caches raw pointers, and Items' base classes are inherited here.                                                                                                    |
| BN       | Time & Weather | M1        | 3 → 4     | The leaks found (`item::add_rain_to_container` and others defined in `weather.cpp`) are other contexts' rules living here. They hurt those contexts' M1 and this context's Q3, not this context's ownership. CDDA scored the same facts 4. |
| BN       | Time & Weather | Q2        | 4 → 3     | BN has the same per-turn `temperature_cache` clearing rule that CDDA's 3 rests on.                                                                                                                                                         |
| BN       | Unclaimed      | M1        | 1 → 2     | Both have a god-object `game` beside infrastructure and scripting that do have homes (Lua in BN, EOC in CDDA).                                                                                                                             |
| BN       | Unclaimed      | M2        | 2 → 3     | The little Content that maps here matches in both (missions, achievements, EOC, widgets).                                                                                                                                                  |
| BN       | Unclaimed      | Q1        | 2 → 3     | The same pattern as CDDA: deep `json`/`generic_factory` beside a shallow `game.h` hub. This is the level-3 anchor.                                                                                                                         |
| BN       | Unclaimed      | Q4        | 1 → 2     | In both, the utilities are leaves that could sit behind a seam today, and only `game`/`do_turn`/`savegame` are wired to everything.                                                                                                        |
| BN       | Unclaimed      | Q5        | 2 → 3     | Both have fixture-free utility tests and no game-loop tests.                                                                                                                                                                               |
| CDDA     | Fields         | Q3        | 3 → 2     | `fd_*` ids are used in at least 28 files outside Fields (19 in Creatures, 9 in Terrain & Map), as in BN. That is "many outside files", not "a known set".                                                                                  |
| CDDA     | Items          | Q1        | 2 → 3     | The same pattern as BN: deep `item_contents`/`item_pocket`/`item_location`/`visitable` beside a flat `item.h`.                                                                                                                             |
| CDDA     | Unclaimed      | Q2        | 2 → 1     | About 3.8k public declarations plus the hand-ordered load sequence in `init.cpp`, against BN's 1 on about 4.2k.                                                                                                                            |

The differences left are real ones:

- **Fields:** CDDA has the data-driven processor pipeline, so its M1 and Q1 are higher. BN has none.
- **Items isolation:** BN uses `g->` 500 times against CDDA's 73, because CDDA passes the map and carrier as parameters.
- **Items tests:** BN's item tests barely use the map fixture.

## 3. Calibrated summary matrix

| Context        | BN M1 M2 M3 → DM | BN Q1–Q5 → Dm     | CDDA M1 M2 M3 → DM | CDDA Q1–Q5 → Dm   |
| -------------- | ---------------- | ----------------- | ------------------ | ----------------- |
| Creatures      | 3 3 1 → **3**    | 2 1 2 1 3 → **2** | 3 3 1 → **3**      | 2 1 2 1 3 → **2** |
| Terrain & Map  | 3 3 2 → **3**    | 3 1 2 1 3 → **2** | 3 3 2 → **3**      | 3 1 2 1 3 → **2** |
| Fields         | 2 4 2 → **2**    | 3 3 2 2 2 → **2** | 3 4 2 → **3**      | 4 3 2 2 2 → **2** |
| Items          | 3 3 2 → **3**    | 3 1 2 2 4 → **2** | 3 3 3 → **3**      | 3 1 2 2 3 → **2** |
| Vehicles       | 2 2 2 → **2**    | 2 2 2 1 3 → **2** | 2 2 2 → **2**      | 2 2 2 1 3 → **2** |
| Time & Weather | 4 4 3 → **4**    | 4 3 3 3 4 → **3** | 4 4 3 → **4**      | 4 3 3 3 4 → **3** |
| Unclaimed      | 2 3 1 → **2**    | 3 1 2 2 3 → **2** | 2 3 1 → **2**      | 3 1 2 2 3 → **2** |

DM is the Deep Model median and Dm the Deep module median. There is no grand total. After calibration, the two Engines differ only on the Fields Deep Model median, so modularity does not decide the Baseline.
