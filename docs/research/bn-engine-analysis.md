# Bright Nights v0.13.0 Engine analysis

Answers [How modular is the Bright Nights v0.13.0 Engine?](https://github.com/adamico/Cataclysm-RD/issues/6), scored with the [Engine scoring rubric](engine-scoring-rubric.md). Terms: [GLOSSARY.md](../../GLOSSARY.md) and `/codebase-design` (module, interface, depth, seam, adapter, leverage, locality).

## 1. Header

- **Baseline:** Cataclysm: Bright Nights `v0.13.0`, repo `cataclysmbn/Cataclysm-BN`.
- **Commit:** `e7b497637d452928ef1a9a77f175c5fd9c3fa75e` (tag `v0.13.0` resolves to the expected `e7b4976`; committed 2026-09-24).
- **Date of analysis:** 2026-11-27.
- **Method:** static reading of a shallow, read-only clone. No build, no `compile_commands.json`, no git history. `cloc` is not installed, so **lines are physical lines (`wc -l` semantics), comments included**; non-blank lines are given too. All scripts are in the appendix and were run with `python3 -I` from a directory outside the clone.

```sh
git clone --depth 1 --branch v0.13.0 https://github.com/cataclysmbn/Cataclysm-BN.git bn
git -C bn rev-parse HEAD   # e7b497637d452928ef1a9a77f175c5fd9c3fa75e
python3 -I split.py      bn out   # Engine / Frontend / Content / Vendored split, lines per file
python3 -I json_types.py bn > out/types.tsv     # JSON `type` values, core vs mods
python3 -I loaders.py    bn > out/loaders.tsv   # JSON type -> loader function -> defining .cpp
python3 -I contexts.py   bn out   # Engine-B file -> context (needs manual.tsv next to it)
python3 -I evidence.py   bn out   # include edges, fan-in/out, globals, public surface, inheritance, friend, foreign ids
python3 -I depmap.py     bn out   # include edges between Engine-B, Frontend-B and Vendored
```

Supporting greps (run in `bn/src` unless noted): thread-pool users `grep -rlE 'parallel_for|get_thread_pool'`; GPU call sites `grep -rnE 'cata_gpu::|gpu_lm::|uses_sdl_gpu_compute'`; Lua binding size `grep -ohE '\b(luna::set_fx|SET_FX_T|SET_FX|SET_MEMB|SET_MEMB_RO|SET_MEMB_N_RO|luna::new_usertype)\b' catalua_bindings*.cpp | sort | uniq -c`; tests by name in `bn/tests` with the regexes given under Q5 below; `calendar::turn` writes `grep -rnE 'calendar::turn\s*(=[^=]|\+=|-=|\+\+)'`; cache-dirty calls `grep -cE '\bset_[a-z_]+_cache_dirty\s*\(|invalidate_map_cache\s*\('` per Engine-B file.

## 2. Code statistics

| Split | Files | Lines | Non-blank |
|---|---|---|---|
| Engine (rubric rule, literal) | 786 | 347,566 | 306,687 |
| Frontend (rubric rule, literal) | 155 | 169,767 | 151,353 |
| **Engine-B** (used for all context work, see below) | **860** | **455,113** | **402,737** |
| **Frontend-B** | **81** | **62,220** | **55,303** |
| Content (`data/`) | 3,165 | 1,064,216 | 1,057,049 |
| Vendored (excluded) | 73 | 66,559 | 57,245 |

Source files counted: `.cpp .h .hpp .c .hlsl .inl` under `src/`. Content counts every file under `data/` (binary files contribute 0 lines). Content breaks down as JSON 2,941 files / 1,009,055 lines (of which `data/json` 1,930 / 700,053 and `data/mods` 994 / 206,562; `data/names` 97,413), Lua 71 / 10,568 (`data/json` 17 / 2,884, `data/mods` 48 / 6,083, `data/raw` 4 / 1,568 dev tools), one `.po` 35,679, and small text files. Vendored = `src/lua` (Lua 5.3 interpreter), `src/sol` (sol2), `src/third-party` (63 files / 53,890, the same regex `src/CMakeLists.txt` uses to exclude them), plus 10 third-party files kept at the top of `src/` (12,669 lines: `fmtlib_*`, `wdirent.h`, `mingw.thread.h`, `simplexnoise.*`, identified by their licence headers).

**Where the classification differs from the rubric.** Applied literally, the "includes a curses header" rule puts 63 core files in Frontend only because they include `cursesdef.h`, which is BN's `catacurses::window` handle type and is included by 87 files. Among them are `map/map.cpp`, `monster.cpp`, `monster.h`, `mtype.h`, `creature.cpp`, `character.h`, `npc.h`, `game.h`, `scent_map.cpp`, `construction.cpp`, `bionics.cpp`, `ranged.cpp` and `magic/magic.cpp`. Scoring Creatures without `monster.cpp`, or Terrain & Map without `map.cpp`, would make no sense. So contexts are scored over **Engine-B**:

- rubric Engine,
- plus files that are Frontend *only* because they include `cursesdef.h` (63 files, 78,995 lines),
- plus 11 files that include SDL or tiles headers for an incidental reason: `game.cpp` (`cursesport.h`, `sdltiles.h`), `ballistics.cpp` (a `tilecontext` lookup), `player_hardcoded_effects.cpp`, `init.cpp`, `path_info.cpp` (Android JNI), `debug.cpp` (clipboard), and the GPU lighting backend `compute/gpu_lm.{h,cpp}`, `compute/gpu_platform.{h,cpp}` and `compute/gpu_transparency.cpp` (they use `SDL3/SDL_gpu.h` as a compute API, not for rendering).

Files that are Frontend by the *name* rule never enter Engine-B. The calibration pass should check that the CDDA analysis made the same choice. CDDA has the same `cursesdef.h` pattern, so the literal rule would distort it the same way.

**Ambiguous files** (in Engine-B but clearly mixing in UI code): all 63 `cursesdef.h`-only files listed above, the most UI-heavy being `action.cpp`, `advanced_inv.h`, `advanced_inv_pane.h`, `auto_pickup.cpp`, `avatar_action.cpp`, `character_display.cpp`, `clzones.cpp`, `crafting_gui.cpp`, `dialogue_win.h`, `editmap.h`, `game_inventory.cpp`, `handle_action.cpp`, `help.cpp`, `iexamine.cpp`, `input.cpp`, `iuse_software_*.cpp` (in-game minigames), `pickup.cpp`, `popup.h`, `string_input_popup.h`, `trade_win.h`, `ui.h`, `wish.cpp`, `worldfactory.cpp`, `vehicle/veh_interact.h`, `vehicle/vehicle_display.cpp`. Also `game.cpp`, which is the game loop *and* the UI dispatcher. The other direction leaks too: 133 Engine-B files include a Frontend-B header, mostly `output.h` (105) and `ui_manager.h` (49).

**Dependency map (include edges between splits).** Engine-B → Engine-B 8,722; Engine-B → Frontend-B 221; Frontend-B → Engine-B 975; Frontend-B → Frontend-B 188; Engine-B → Vendored 18. Content depends on the Engine through 139 `add("type", loader)` registrations in `src/init.cpp` (`DynamicDataLoader::initialize`) and through the Lua bindings (section 6).

**Engine-B lines per context**

| Context | Files | Lines | Non-blank |
|---|---|---|---|
| Creatures | 192 | 132,217 | 117,483 |
| Terrain & Map | 149 | 94,733 | 83,776 |
| Fields | 15 | 4,015 | 3,600 |
| Items | 133 | 77,912 | 68,644 |
| Vehicles | 37 | 24,709 | 21,718 |
| Time & Weather | 10 | 4,106 | 3,640 |
| Unclaimed | 324 | 117,421 | 103,876 |

How files were assigned: step 1, content anchor, 159 files (each JSON type mapped to a context; the `.cpp` defining its loader, plus the same-stem `.h`). Step 2a, `src/` sub-directories as declared clusters, 123 files. Step 2b, include clustering, 60 files (plurality of include edges in both directions to step-1/2a files, at least 2 edges and 1.5× the runner-up; headers with in-degree over 30 are skipped as shared vocabulary; a vote that contradicts a name rule counts as not settled). Step 3, name regexes, 440 files. Step 4, leftovers to Unclaimed, 70 files. Manual fixes, 8 files (`manual.tsv` in the appendix). Step 2a and the hub/name-conflict guards are additions to the rubric's procedure. Without them, raw include voting pulled `json.h`, `rng.h`, `calendar.h`, `item.h` and `vehicle/vehicle.cpp` into Creatures.

**Include edges between contexts** (row includes column; Engine-B only; diagonal = internal):

| from \ to | Cre | Map | Fld | Itm | Veh | T&W | Unc |
|---|---|---|---|---|---|---|---|
| Creatures | 800 | 202 | 29 | 322 | 94 | 79 | 1,285 |
| Terrain & Map | 139 | 413 | 18 | 110 | 86 | 44 | 771 |
| Fields | 15 | 13 | 17 | 6 | 4 | 7 | 66 |
| Items | 195 | 89 | 10 | 349 | 53 | 31 | 641 |
| Vehicles | 61 | 46 | 2 | 61 | 100 | 10 | 227 |
| Time & Weather | 10 | 11 | 0 | 2 | 1 | 14 | 73 |
| Unclaimed | 341 | 205 | 27 | 187 | 43 | 44 | 1,360 |

**Per-context evidence counts** (from `evidence.py`; "pub" = public declarations in the context's headers, "free" = namespace-scope function declarations; globals are occurrences in the context's own files):

| Context | fan-in files | fan-out files | `g->` | `get_map()` | `get_avatar()`+`get_player_character()` | `calendar::turn` | other globals¹ | static mutable² | pub + free (headers) | cross-ctx inherit / friend |
|---|---|---|---|---|---|---|---|---|---|---|
| Creatures | 149 | 197 | 1,468 | 324 | 159 | 176 | 162 | 39 | 4,862 + 818 (100 h) | 8 / 17 |
| Terrain & Map | 154 | 181 | 505 | 220 | 49 | 127 | 228 | 54 | 3,220 + 584 (71 h) | 3 / 4 |
| Fields | 58 | 61 | 0 | 2 | 0 | 3 | 8 | 1 | 227 + 9 (8 h) | 0 / 0 |
| Items | 163 | 161 | 500 | 140 | 70 | 78 | 148 | 21 | 3,000 + 382 (70 h) | 1 / 10 |
| Vehicles | 78 | 116 | 192 | 58 | 34 | 24 | 37 | 4 | 932 + 34 (17 h) | 3 / 4 |
| Time & Weather | 172 | 61 | 116 | 5 | 7 | 21 | 32 | 5 | 182 + 100 (5 h) | 0 / 2 |
| Unclaimed | 445 | 199 | 549 | 129 | 79 | 103 | 374 | 90 | 2,804 + 1,442 (184 h) | 3 / 13 |

¹ `get_weather()`, `get_overmapbuffer`/`ACTIVE_OVERMAP_BUFFER`, `MAPBUFFER(_REGISTRY)`, `item_controller`, `get_option<`, `get_event_bus`/grid trackers/`get_stats`. ² regex for non-const `static` variables; a heuristic. A write heuristic (`g->x = …`, `calendar::turn +=`) found almost nothing (61 in total), because BN mutates globals through member calls. So reads and writes were split by sampling only, and every M3 score is marked accordingly.

**Hard-coded foreign ids** (uses, inside a context's files, of another context's id constants: `fd_*` fields, `t_*`/`f_*` terrain and furniture, `mon_*` monsters, `itype_*` items, `vpart_*` parts):

| In \ of | fd_ | t_/f_ | mon_ | itype_ | vpart_ |
|---|---|---|---|---|---|
| Creatures | 84 | 145 | – | 732 | 31 |
| Terrain & Map | 117 | – | 107 | 689 | 66 |
| Fields | – | 9 | 0 | 7 | 0 |
| Items | 25 | 30 | 70 | – | 27 |
| Vehicles | 5 | 35 | 6 | 213 | – |
| Time & Weather | 0 | 18 | 19 | 15 | 0 |
| Unclaimed | 21 | 253 | 162 | 234 | 44 |

## 3. Per-context scorecards

Tests per context come from file names in `tests/` (184 `.cpp`): Creatures `monster|mon_|creature|npc|char_|melee|mutation|bionic|effect|magic|martial|behavior|avatar|player` → 32; Terrain & Map `map|mapgen|overmap|submap|terrain|furniture|shadowcast|vision|visib|pathfind|trap|construct|grid` → 18; Fields `field|fire|scent|emit|gas|smoke` → 3; Items `item|itype|craft|recipe|reload|comestible|ammo|gun|iuse|visitable|inventory|pocket|rot|salvage|requirement|clothing|armor|wield|projectile|ranged|throw` → 34; Vehicles `vehicle|veh_|wheel` → 16; Time & Weather `weather|calendar|sun|time|season|temperature` → 4. The setup of a sample was read for each context.

### Creatures

192 files, 132,217 lines: `creature.*`, `character*.{h,cpp}`, `avatar*`, `player*`, `npc*`, `monster*`, `mtype.*`, `monmove.cpp`, `monattack.cpp`, `mattack_*`, `mongroup.*`, `monstergenerator.*`, `mutation*`, `bionics.*`, `effect.*`, `bodypart.*`, `martialarts.*`, `magic/`, `enchantments/`, `activity_*`, `melee.cpp`, `ranged.cpp`, `suffer.cpp`, `npctalk*`, `dialogue*`, `condition.*`, `morale*`, `skill*`, `profession.*`, `scenario.*` (full list: `contexts.tsv`). **Doubtful:** `activity_*` (character activities, heavy on items and map), `ranged.cpp` and `melee.cpp` (combat, not creatures as such), `magic/` and `enchantments/` (could be their own context), `condition.cpp` and `npctalk*` (dialogue). Rough split inside it: monsters 28 files / 23,347 lines; NPC and dialogue 16 / 18,604; magic and enchantments 23 / 7,556; characters, the player and everything else 125 / 64,106.

| Criterion | Score (1–5) | Confidence | Evidence |
|---|---|---|---|
| M1 Concept ownership | 2 | med | Home classes exist (`Creature` in `creature.h`, `Character` in `character.h`, `monster`/`mtype`), but key rules live elsewhere. The monster turn loop, the LOD tiers and the plan/apply scheduling are in `game.cpp` (`game::monmove`, ~22 `lod_tier`/`monster_lod_enabled` refs). Field effects on creatures are in `map/map_field.cpp` (`map::player_in_field` ~345 lines, `monster_in_field`, `creature_in_field`). Boarding is in `map.cpp` (`map::board_vehicle`). `Character` is a god class: `character.cpp` 13,016 lines, 905 declarations in `character.h`. |
| M2 Language alignment | 3 | med | The main nouns match: `MONSTER` → `mtype`, `effect_type`, `bionic` → `bionic_data`, `SPELL` → `spell_type`, `npc` → `npc_template`. Secondary names drift: `mutation` → `mutation_branch` (and `trait_id`), `SPECIES` → `species_type`, `martial_art` → `martialart`. Three names cover the player: `Character`, `player`, `avatar` (`class avatar : public player : public Character`). |
| M3 Context isolation | 1 | high (writes: low) | Highest global use in the Engine: `g->` 1,468, `get_map()` 324, `get_avatar()`/`get_player_character()` 159, `calendar::turn` 176, static mutable 39. Other contexts reach in through the same globals (`get_avatar()` is a `friend` of `game`, `game.h:182`). Read/write split only sampled: `monmove.cpp` mutates the map through `g->m`. |
| Q1 Leverage | 2 | med | A few deep entry points exist (`monster::compute_plan`/`apply_plan` in `monmove.cpp`, `Character::process_turn`), but they sit on a very wide, mostly shallow surface: 4,862 public declarations and 818 free functions over 100 headers (`character.h` 905, `npc.h` 436, `creature.h` 310, `monster.h` 301). |
| Q2 Interface size | 1 | high | Thousands of public members, plus ordering rules a caller can break: `compute_plan()` thread-safety preconditions (`monster.h:256–300`), `reset_stats`/recalc rules for encumbrance and effects, and `lod_tier` set by `game.cpp`. |
| Q3 Locality | 2 | med | A change to monster behaviour typically touches `game.cpp` (scheduling, LOD) and `map_field.cpp` (field reactions). Creatures hard-codes 732 `itype_*`, 145 terrain/furniture and 84 `fd_*` ids from other contexts. 149 outside files include its headers. |
| Q4 Seam cleanliness | 1 | med | `Character` and `monster` inherit Items' `location_visitable<>` template (`character.h:269`, `monster.h:105`). `game` owns the avatar and exposes it as a global (`friend avatar &get_avatar()`). Map and vehicles hold `Character*`/`monster*` (passengers, `creature_tracker`). The Lua bindings expose `Character`, `monster`, `npc` and `avatar` directly (section 6). Globals run in both directions. |
| Q5 Testability | 3 | med | 32 test files go through public calls, but need the full shared fixture: the include-voted Creatures tests call `clear_avatar()`/`clear_character()` 132 times and `clear_map()` 55 times; `monster_test.cpp` calls `clear_map` 23 times and `g->` 7 times. |

### Terrain & Map

149 files, 94,733 lines: `map/` (except field files), `mapgen/`, `overmap*`, `omdata.h`, `regional_settings.*`, `trap.*`, `trapfunc.cpp`, `construction*`, `iexamine*`, `clzones.*`, `shadowcasting.*`, `pathfinding.*`, `explosion.*`, `distribution_grid.*`, `fluid_grid.*`, `active_tile_data*`, `start_location.*`, `travel/`, `compute/`, `shaders/`. **Doubtful:** `iexamine.cpp` (8,647 lines of examine actions over items, creatures and vehicles), `explosion.cpp`, `construction.cpp`, `distribution_grid`/`fluid_grid` (BN's power and liquid networks), `compute/` and `shaders/` (GPU lighting), `data_vars.h`, `descriptions.cpp`, `auto_note.h`. Rough split: world generation (mapgen, overmap, region) 45 files / 31,196 lines; light and vision (`lightmap`, `shadowcasting`, `compute`, `shaders`, `map_memory`) 31 / 13,333; local map 73 / 50,204.

| Criterion | Score (1–5) | Confidence | Evidence |
|---|---|---|---|
| M1 Concept ownership | 3 | med | `map`/`submap` (`map/map.h`, `map/submap.h`), `ter_t`/`furn_t` (`map/mapdata.h`), `overmap` and `mapgen` each have a home. But `map.cpp` also holds vehicle physics (38 `map::*veh*` functions: `vehmove`, `move_vehicle`, `vehicle_vehicle_collision`, `displace_vehicle`) and the reality-bubble caches for every context. |
| M2 Language alignment | 3 | med | `mapgen`, `palette`, `overmap_special`, `region_settings`, `trap` and `construction` match one to one. Drift: `terrain` → `ter_t`/`ter_id`/`ter_str_id`, `furniture` → `furn_t`, `overmap_terrain` → `oter_t`/`oter_type_t` (with an `oter_id_migration` type), legacy `t_`/`f_` id prefixes. |
| M3 Context isolation | 2 | high (writes: low) | `g->` 505, `get_map()` 220, `calendar::turn` 127, `MAPBUFFER(_REGISTRY)` 64, overmapbuffer globals 49, static mutable 54. The map is itself a global (`get_map()` is a `friend` of `game`, `game.h:180`), and outside contexts poke its caches: 68 `set_*_cache_dirty`/`invalidate_map_cache` calls in 12 files outside Map (Vehicles 34). |
| Q1 Leverage | 3 | med | Some deep entry points (`map::build_map_cache`, `submap_load_manager`, `mapgen` from JSON, `overmapbuffer` generation), mixed with a wide surface: `map/map.h` 618 declarations, `map/mapbuffer.h` 349, `map/mapdata.h` 243. |
| Q2 Interface size | 1 | high | 3,220 public + 584 free declarations over 71 headers, and hand-invalidated caches exposed as public setters: 16+ `set_*_cache_dirty` functions and 56 "dirty" lines in `map/map.h` (`map.h:806–853`). This is the rubric's level-1 example. |
| Q3 Locality | 2 | med | Map code hard-codes 689 `itype_*`, 117 `fd_*` and 107 `mon_*` ids. Terrain flags are tested from every context (for example `monmove.cpp:202–214`, `g->m.has_flag(...)`). Vehicle rules live in `map.cpp`. |
| Q4 Seam cleanliness | 2 | med | `map` derives from `submap_load_listener` (fine), but has `friend class editmap`, `mapbuffer` and `location_visitable<map_cursor>` (`map.h:734–737`). Its level cache holds raw vehicle pointers (`veh_cached_parts`, `vehicle_list` in `map.h:444–446`), and submaps own vehicles (`submap.h:341`). The GPU backend is reached from only `map.cpp` and `lightmap.cpp`, a clean internal seam. |
| Q5 Testability | 3 | med | 18 test files. Pure modules test small (`shadowcasting_test`, `simple_pathfinding_test`, `overmap_noise_test`). The map itself needs the full game: `map_test.cpp` uses `g->` 144 times and `clear_map` 17 times. |

### Fields

15 files, 4,015 lines: `map/field.*`, `map/field_type.*`, `map/emit.*`, `map/map_field.cpp`, `map/submap_fields.h`, `scent_map.*`, `scent_block.*`, `fire.h`, `fire_spread_loader.*`. **Doubtful:** scent (`scent_type` is a JSON type and scent diffuses like a field, but it is a separate grid), and `map_field.cpp`, whose functions are `map::` members.

| Criterion | Score (1–5) | Confidence | Evidence |
|---|---|---|---|
| M1 Concept ownership | 2 | med | `field_type`, `field` and `field_entry` have a home, but behaviour is keyed on hard-coded ids: 45 `fd_*` comparisons in `map_field.cpp`, and 252 `fd_*` uses outside Fields (Map 117, Creatures 84, Items 25, Unclaimed 21, Vehicles 5). Field effects on creatures (`map::player_in_field`, ~345 lines) are Creatures rules kept here. |
| M2 Language alignment | 4 | high | The `field_type` JSON keys and the loader keys in `map/field_type.cpp` line up one to one (28 keys, plus `immune_mtypes`). `legacy_enum_id` is the one legacy name. `emit` and `scent_type` match too. |
| M3 Context isolation | 3 | med | Its own files use almost no globals (`g->` 0, `get_map()` 2, `calendar::turn` 3, `MAPBUFFER` 8), but only because processing is written as `map::` member functions using `this`. Field storage sits inside submaps, and every context touches it through `map::add_field`/`get_field*`. |
| Q1 Leverage | 3 | med | One deep entry point (`process_fields_in_submap`, `map_field.cpp:1020`), plus a flat accessor surface on `map` (~70 field-related lines in `map/map.h`). |
| Q2 Interface size | 3 | med | 227 public + 9 free declarations over 8 headers. Ordering rules: field caches must be marked dirty (`mark_field_cache_dirty`, `map_field.cpp:926`), and a JSON flag `dirty_transparency_cache` tells the map to invalidate light. |
| Q3 Locality | 2 | med | Adding a field behaviour means editing `map_field.cpp` plus the `fd_*` checks in other contexts (252 uses). |
| Q4 Seam cleanliness | 2 | med | No hierarchy and no `friend`, but the storage (`submap_fields.h`) and the processing (`map::` members) belong to `map`. A seam needs Fields cut out of the map class first. |
| Q5 Testability | 2 | med | 3 name-matched tests (`creature_in_field_test.cpp` 36 lines and needs `clear_map`; `scent_test.cpp` checks against an old scent update; `start_location_fire_test.cpp`). There is no `field_type` unit test. |

### Items

133 files, 77,912 lines: `item.*`, `itype.*`, `item_factory.*`, `item_group.*`, `item_contents.*`, `iuse*.{h,cpp}`, `inventory.*`, `visitable.*`, `locations.*`, `location_ptr.*`, `location_vector.*`, `detached_ptr.*`, `requirements.*`, `recipe*`, `crafting*`, `material.*`, `ammo*`, `flag.*`, `fault.*`, `relic.*`, `artifact.*`, `pickup.*`, `consumption.*`, `ballistics.*`, `projectile.*`, `reload/`, `units/`, `savegame_json.cpp`. **Doubtful:** `savegame_json.cpp` (it serialises every context but was content-anchored by `ITEM_BLACKLIST`/migration loaders; it would sit better in Unclaimed), `crafting*`/`recipe*`/`requirements*` (a crafting context?), `consumption.cpp`/`stomach.*` (character digestion), `iuse_software_*` (minigames, UI), `advanced_inv*` (UI). Rough split: crafting 18 files / 10,363 lines; `iuse` 16 / 22,157; item core 99 / 45,392.

| Criterion | Score (1–5) | Confidence | Evidence |
|---|---|---|---|
| M1 Concept ownership | 3 | med | `item`/`itype`/`Item_factory` each have one home, and BN has an explicit ownership model (`location_ptr`, `detached_ptr`, `location_vector`). But item rules also live in `character.cpp` (403 item mentions) and `iuse.cpp` (9,537 lines, keyed on ids). Rain filling is `item::add_rain_to_container`, defined in `weather/weather.cpp:219`. |
| M2 Language alignment | 3 | med | `AMMO`/`GUN`/`TOOL`/`COMESTIBLE`/… map to `islot_*` slots on `itype`. `use_action` names match `iuse_actor` classes. One word means two things: the JSON "item" definitions are `itype`, while C++ `item` is an instance. `GENERIC` has no matching class. |
| M3 Context isolation | 2 | high (writes: low) | `g->` 500, `get_map()` 140, `get_avatar()` 54, `calendar::turn` 78. A singleton factory, `item_controller` (`item_factory.h:43`), is used 36 times here and 41 times outside. |
| Q1 Leverage | 3 | med | `visitable` and `location_visitable` give real leverage (`visit_items`, `remove_items_with` across characters, the map, vehicles and containers), and `iuse_actor::use` is a deep polymorphic entry point. But `item.h` has 632 declarations, mostly getters. |
| Q2 Interface size | 2 | med | 3,000 public + 382 free declarations over 70 headers (`iuse_actor.h` 670, `item.h` 632, `itype.h` 341). The ownership protocol must be followed (detach to a `detached_ptr` before moving), along with charges and count invariants. |
| Q3 Locality | 2 | high | Item ids are hard-coded everywhere: `itype_*` is used 732 times in Creatures, 689 in Map, 234 in Unclaimed and 213 in Vehicles. 163 outside files include Items headers. |
| Q4 Seam cleanliness | 2 | med | Items' `location_visitable<>` is a base class of `Character`, `monster`, `map_cursor` and `vehicle_cursor`. `vehicle_part` owns `location_ptr<item, true> base` and a `location_vector<item>` (`vehicle_part.h:302–303`). `item` also derives from Unclaimed's `game_object<item>` (`item.h:233`). There are 10 cross-context friends. |
| Q5 Testability | 4 | med | 34 name-matched tests. `item_test.cpp` (415 lines, 16 `TEST_CASE`) uses no `clear_map`, `g->` or `get_map()`, so items test standalone. The include-voted Items tests call `clear_map` only 6 times. |

### Vehicles

37 files, 24,709 lines: all of `src/vehicle/` plus `grab.cpp`. **Doubtful:** `grab.cpp` (it also grabs furniture), `vehicle/veh_interact.*` and `vehicle/vehicle_display.cpp` (UI).

| Criterion | Score (1–5) | Confidence | Evidence |
|---|---|---|---|
| M1 Concept ownership | 2 | high | `vehicle`, `vehicle_part` and `vpart_info` have a home in `src/vehicle/`, but movement and collision are `map` members (38 `map::*veh*` functions in `map/map.cpp`: `vehmove` :1077, `move_vehicle` :1270, `vehicle_vehicle_collision` :1529, `displace_vehicle` :1884). `game.cpp` has 466 vehicle mentions. |
| M2 Language alignment | 3 | high | `vehicle_group`, `vehicle_placement` and `vehicle_spawn` match. But the JSON `vehicle_part` → `vpart_info` while C++ `vehicle_part` is the installed instance, so one word means two things. JSON `vehicle` → `vehicle_prototype`. |
| M3 Context isolation | 2 | med (writes: low) | `g->` 192, `get_map()` 58, `get_avatar()`/`get_player_character()` 34 in 24.7k lines, the densest global use after Creatures. The map keeps vehicle caches (`vehicle_list`, `veh_cached_parts`) and Vehicles calls `set_*_cache_dirty` 34 times. |
| Q1 Leverage | 2 | med | One class carries the context: `vehicle/vehicle.h` has 550 declarations, with many shallow per-part accessors. |
| Q2 Interface size | 2 | med | 932 public + 34 free declarations over 17 headers. `vehicle.h` has 23 `mutable` cache members, and there are ordering rules (`refresh()` after part changes, map vehicle caches dirtied by hand). |
| Q3 Locality | 2 | med | Changes to vehicle motion touch `map.cpp` and `game.cpp`. Vehicles hard-codes 213 `itype_*` ids. |
| Q4 Seam cleanliness | 2 | med | No deep hierarchy, but submaps own vehicles (`std::vector<std::unique_ptr<vehicle>>`, `submap.h:341`), the map caches raw `vehicle*`, and `vehicle_stack`, `vehicle_cursor` and `vehicle_selector` inherit Items' `item_stack`/`location_visitable` (`vehicle.h:140`, `vehicle_selector.h:11, 22`). |
| Q5 Testability | 3 | high | 16 name-matched tests, which go through public calls but need a map: `vehicle_test.cpp` calls `clear_map` 18 times and `get_map()` 22 times. |

### Time & Weather

10 files, 4,106 lines: `calendar.*`, `timed_event.*`, `weather/` (`weather.*`, `weather_gen.*`, `weather_type.*`). **Doubtful:** `timed_event.*` (scheduled world events, not time itself).

| Criterion | Score (1–5) | Confidence | Evidence |
|---|---|---|---|
| M1 Concept ownership | 3 | med | `time_point`/`time_duration`/`calendar` (`calendar.h`) and `weather_manager`/`weather_type` have clear homes, but `weather.cpp` defines other contexts' members (`item::add_rain_to_container` :219, `trap::funnel_turns_per_charge` :293) and character effects (`weather_effect::wet_player` :362). |
| M2 Language alignment | 4 | med | `weather_type` JSON ↔ `weather_type` C++ (JSON `weather_type_id` literals: 0 in C++). Time units read naturally (`time_duration::from_turns`, `12_hours`). |
| M3 Context isolation | 3 | high | `calendar::turn` is a static mutable global read 636 times in 118 files but written in only ~6 places (`game.cpp:2006`, `savegame.cpp:224`, the game modes, the debug menu), so access is funnelled. `get_weather()` is used 118 times in 30 files. |
| Q1 Leverage | 4 | med | Small value types with large reach: 164 Engine-B files include `calendar.h`. `weather_manager::update_weather` is one entry point. |
| Q2 Interface size | 4 | med | 182 public + 100 free declarations over 5 headers. No hand-ordering rules were found. |
| Q3 Locality | 3 | med | Weather effects on items, traps and characters live here, so a known set of outside classes changes with weather rules. |
| Q4 Seam cleanliness | 3 | med | The global clock (`calendar::turn`) and `game`-owned `weather_manager` (`friend weather_manager &get_weather()`, `game.h:183`) must be cut. There is no hierarchy. |
| Q5 Testability | 4 | high | `calendar_test.cpp` (34 lines), `weather_test.cpp` (345 lines, 0 `clear_map`/`g->`/`get_map()`) and `sun_test.cpp` test through public calls with a small setup. |

### Unclaimed

324 files, 117,421 lines: `game.*`, `init.*`, `savegame*.cpp` (except `savegame_json.cpp`), `json.*`, `generic_factory.*`, `catalua*` (63 files, 15,894 lines), `thread_pool.*`, `options.h`, `debug.*`, `coordinates.h`, `point.*`, `units*.h`, `string_*`, `mission*`, `sounds.*`, `event*`, `achievement.*`, `world*`, `mod_manager.*`, `handle_action.cpp`. **Doubtful:** `sounds.cpp` (noise propagation that monsters react to is Simulation), `game.cpp` (it holds the monster turn loop), `mission*`, `computer*`, `coordinates.h`/`point.h` (Map geometry or shared vocabulary?), `stats_tracker.*`.

| Criterion | Score (1–5) | Confidence | Evidence |
|---|---|---|---|
| M1 Concept ownership | 1 | high | Expected. `game.cpp` (17,028 lines, 334 `game::` functions) holds the loop, monster scheduling, UI dispatch and world setup. |
| M2 Language alignment | 2 | low | Expected. Little Content maps here (options, snippets, events, missions, achievements). Mission and achievement names match their JSON; the rest has no Content vocabulary. |
| M3 Context isolation | 1 | high | `extern std::unique_ptr<game> g` (`game.h:63`) is the hub of every global: `g->` 3,330 times across Engine-B, `get_option<` 748. Unclaimed alone has static mutable 90 and the most write-heuristic hits (44). |
| Q1 Leverage | 2 | med | `json.h` (305 declarations) and `generic_factory.h` give real leverage to every loader, but `game.h` (306) is a shallow hub of pass-throughs. |
| Q2 Interface size | 1 | high | 2,804 public + 1,442 free declarations over 184 headers. |
| Q3 Locality | 2 | med | 445 files outside include Unclaimed headers. Changes to `game` ripple across contexts. |
| Q4 Seam cleanliness | 1 | high | `game` hands out every context through `friend` globals (`get_map`, `get_player_character`, `get_avatar`, `get_weather`, `get_distribution_grid_tracker`; 15 friend lines in `game.h`) and inherits Map's `submap_load_listener` (`game.h:170`). |
| Q5 Testability | 2 | low | Utility tests exist (`json_test`, `units_test`, `cata_variant_test`, `generic_factory_test`, `catalua_test`), but none of the game loop runs without the full game. |

## 4. Summary matrix

| Context | M1 | M2 | M3 | Deep Model median | Q1 | Q2 | Q3 | Q4 | Q5 | Deep module median |
|---|---|---|---|---|---|---|---|---|---|---|
| Creatures | 2 | 3 | 1 | 2 | 2 | 1 | 2 | 1 | 3 | 2 |
| Terrain & Map | 3 | 3 | 2 | 3 | 3 | 1 | 2 | 2 | 3 | 2 |
| Fields | 2 | 4 | 3 | 3 | 3 | 3 | 2 | 2 | 2 | 2 |
| Items | 3 | 3 | 2 | 3 | 3 | 2 | 2 | 2 | 4 | 2 |
| Vehicles | 2 | 3 | 2 | 2 | 2 | 2 | 2 | 2 | 3 | 2 |
| Time & Weather | 3 | 4 | 3 | 3 | 4 | 4 | 3 | 3 | 4 | 4 |
| Unclaimed | 1 | 2 | 1 | 1 | 2 | 1 | 2 | 1 | 2 | 2 |

No grand total.

## 5. FFI cut-cost notes

Header counts per context (`std::map`/`multimap`, `std::unordered_*`, `std::list`, `std::set`, `virtual`, raw-pointer data members, by regex):

| Context | map | unordered | list | set | virtual | raw-ptr members |
|---|---|---|---|---|---|---|
| Creatures | 127 | 33 | 10 | 128 | 278 | 29 |
| Terrain & Map | 65 | 52 | 8 | 40 | 46 | 51 |
| Fields | 9 | 1 | 0 | 3 | 0 | 0 |
| Items | 95 | 19 | 13 | 88 | 43 | 49 |
| Vehicles | 17 | 2 | 0 | 18 | 3 | 17 |
| Time & Weather | 0 | 1 | 1 | 0 | 0 | 0 |
| Unclaimed | 60 | 41 | 9 | 34 | 46 | 134 |

- **Creatures:** the `Creature → Character → player → avatar/npc` and `Creature → monster` hierarchy, with 278 `virtual`s, would cross any seam, as would the `location_visitable<>` CRTP base from Items. Shared state: `g->` everywhere, `creature_tracker` holding `shared_ptr_fast<monster>`, `Character*` passengers in vehicles. Hot loop: `game::monmove`, then `compute_plan` and `apply_plan` per monster per turn, calling `map` queries (`has_flag`, `sees`, LOS caches) and Lua hooks. It is the highest-volume crossing in the Simulation.
- **Terrain & Map:** STL-heavy caches (`std::map<tripoint_bub_ms, std::pair<vehicle*, int>> veh_cached_parts`, `std::set<vehicle*>`), and submaps own items, fields, vehicles and creatures' spawn points by value. Hot loops: per-tile cache builds (transparency, floor, outside, seen), lightmap and shadowcasting, terrain-flag queries from every creature. A cut at the map would see the densest traffic in the Engine.
- **Fields:** small and container-light (9 `std::map`), but its storage sits inside `submap` and its processing in `map::` members. Hot loop: `process_fields_in_submap` per active submap per turn, calling creature effects (`creature_in_field`). It is the cheapest Simulation context to cut *if* the map exposes a per-submap field array.
- **Items:** `std::list`/`std::map`-heavy (`item_contents`, `inventory`), with ownership through `location_ptr`/`detached_ptr` templates, which `cxx` cannot express directly. The `iuse_actor` hierarchy has 43 `virtual`s. Hot loops: active item processing (`active_item_cache`) and the `visitable` walks called from characters, the map and vehicles.
- **Vehicles:** a moderate container use, but `vehicle_part` embeds items (`location_ptr<item>`) and the map owns and caches vehicles by raw pointer. Hot loop: `map::vehmove`/`vehproceed`, which lives on the map side, so a seam would cut through `map.cpp`.
- **Time & Weather:** almost no containers or hierarchies. The only shared state to cut is `calendar::turn` and `get_weather()`. `time_point`/`time_duration` are plain value types, easy to mirror as `#[repr(C)]` structs. It is the cheapest seam in the Engine.
- **Unclaimed:** `game` (raw pointers, 134 raw-pointer members across its headers), `json`/`generic_factory` templates, and the Lua bindings (sol2 usertypes over raw C++ classes). Nothing here can cross `cxx` cheaply. A port would rewrite it rather than wrap it.

## 6. Cross-cutting seams

**Threading.** `thread_pool.h`/`.cpp` (352 lines) provide `cata_thread_pool` with `submit`/`submit_returning`, `parallel_for` and `parallel_for_chunked`. They are used in 12 `.cpp` files (and referenced in 25), across three contexts:
- *Terrain & Map:* the per-z-level map cache build (`map.cpp:6321, 8201, 8852`), lightmap (`lightmap.cpp:703, 1485`), shadowcasting (`shadowcasting.cpp:735`), the mapbuffer (`mapbuffer.cpp:3121`), async submap loading and mapgen (`submap_load_manager.cpp:485, 500, 865`), and overmap generation (`overmapbuffer.cpp:147, 1514`).
- *Creatures:* parallel monster planning, driven from `game.cpp:6453`.
- *Fields:* scent diffusion (`scent_map.cpp:314, 349`).

Each use sits behind an option (`PARALLEL_MONSTER_PLANNING`, `PARALLEL_MAP_CACHE`, `PARALLEL_SCENT_UPDATE`, all default `true`, under `MULTITHREADING_ENABLED`; `cached_options.h`). The monster work is a deliberate plan/apply split: `monster::compute_plan() const` returns a `monster_plan_t` of intended mutations, and `apply_plan()` applies it on the main thread (`monster_plan.h`). That is a good internal seam and a good model for a Rust port. One caveat: `monmove.cpp:391–396` notes that worker-thread RNG breaks save-file determinism, which is fine under **Gameplay equivalence** but rules out bit-exact comparison testing with parallelism on. The thread pool is not its own context: it is an Unclaimed utility whose callers are Map caches, monster planning and scent.

**Monster AI level of detail.** The `MONSTER_LOD_ENABLED` option (default `true`, `options.cpp:2435`) and 8 `lod_*` tuning knobs assign each monster a `lod_tier` (`monster.h:616`). Higher tiers think less often, an action budget caps how many monsters move per turn, and budget-cut monsters lose their moves (`game.cpp:6404–6419`). This is an **Approximation** in the glossary's sense, and it is **on by default**. Benchmark-save numbers for BN must be taken with it off, or flagged, before they count as Gameplay equivalence. The LOD logic lives in `game.cpp` (22 refs) and `monmove.cpp` (5), not in Creatures, which is one more Creatures rule kept outside its home.

**GPU lighting.** `src/compute/` (7 files, 6,409 lines) plus 18 HLSL compute shaders in `src/shaders/` (2,191 lines). They compute the lightmap, transparency, seen and visibility caches and sight pairs. `compute_backend.h` picks `sdl_gpu_hardware`, `sdl_gpu_software` or `cpu_compute` from the `COMPUTE_ACCELERATION` option (`auto` by default, SDL builds only, `options.cpp:2377`). The seam is narrow: outside `compute/`, only `map/map.cpp` (43 refs), `map/lightmap.cpp` (26) and `main.cpp` (2, device init) call it. Its interface still carries ordering rules: the dirty flags in `gpu_lm.h:168–181` (`transparency_dirty`, `floor_dirty`, `vehicle_floor_dirty`, `rebuild_seen_cache`, and `download_lightmap`, commented "transitional CPU readback"). With two adapters (CPU and GPU), this is a **real seam** inside Terrain & Map, and the clearest candidate for a "Light & Vision" context (section 7). It does not cut across other contexts directly, but light and vision feed every creature's `sees()`.

**Lua bindings.** 63 `catalua*` files (15,894 lines; 30 binding translation units, 9,680 lines) bind about 3,900 members with sol2: `luna::set_fx` 1,626, `SET_FX_T` 878, `SET_MEMB_RO` 680, `SET_FX` 480, `SET_MEMB` 176, `SET_MEMB_N_RO` 50. They span about 95 usertypes, and expose raw Engine classes (`Character`, `avatar`, `monster`, `npc`, `item`, `itype`, `map`, `mtype`, `ter_t`, `furn_t`, `overmapbuffer`, `distribution_grid`, `time_point`). Of the 89 hook names (`catalua_hooks.cpp`), several run in hot loops: `on_monster_do_turn`, `on_creature_do_turn`, `on_npc_do_turn`, `on_monster_try_move`, `on_every_x` and `on_tick`. They are guarded by `cata::has_hooks(...)`, as in `game.cpp:6542`. Hook call sites appear in 29 files across all contexts, and 47 non-binding Engine files include a `catalua` header (Creatures 34). So Lua cuts across every context and **widens each one's interface**: whatever a binding exposes becomes something a port must keep.

**Do Lua scripts look like Content?** Yes. The core mod loads Lua through `data/json/main.lua` and `preload.lua` (`catalua.cpp:502, 524`). That is 17 files / 2,884 lines in `data/json/lua`: item uses (`iuse/voltmeter.lua`, `sonar.lua`, `artifact_analyzer.lua`), mapgen (`mapgen/lab.lua`, 534 lines), traits (`traits/lua_traits.lua`) and plumbing (`plumbing.lua`, 662 lines). Another 48 files / 6,083 lines sit in mods (`rpg_system`, `teleportation_mod`, `ebook_lua`, and others). They live under `data/`, sit beside JSON, and define game behaviour per id, so by the rubric's rule and by role they are Content. But they are *executable* Content written against the current C++ class shapes. Migrating them means keeping (or re-implementing) a Lua API of about 3,900 members, or rewriting about 9,000 lines of Lua.

## 7. Context proposals

1. **Split Creatures into Monsters and Characters (plus a thin Creature core).** Evidence: monsters are 28 files / 23,347 lines, while characters, the player, NPCs and dialogue are about 141 files / 82,700 lines. They share only the `Creature` base. Monster AI has its own data family (`MONSTER`, `SPECIES`, `monstergroup`, `monster_attack`, `MONSTER_FACTION`), its own turn loop and the plan/apply seam. The Character side carries `mutation`, `bionic`, `effect_type`, `martial_art`, `profession`, `skill`. For Dense-area Simulation, Monsters is the part that matters.
2. **Split out NPC & Dialogue** (16 files / 18,604 lines: `npc*`, `npctalk*`, `dialogue*`, `condition*`, `talk_topic`, `npc_class`, `faction`) from Characters. They form a JSON family of their own (`talk_topic` 1,004 + 788 objects).
3. **Split Terrain & Map into three:** World generation (mapgen, overmap, region; 45 files / 31,196 lines, with its own JSON families `mapgen`, `palette`, `overmap_*`, `city_building`, `region_settings`), Local map (the reality bubble, `map`/`submap`/`mapbuffer`), and **Light & Vision** (`lightmap`, `shadowcasting`, `compute/`, `shaders/`, `map_memory`; 31 files / 13,333 lines). Light & Vision already has two adapters (CPU and GPU) behind `compute_backend.h`.
4. **Merge Fields into Local map, or give it a per-submap interface.** Fields is 4,015 lines, its storage is in `submap` and its processing is `map::` members. As its own context it scores Deep module 2 for structural reasons. Either treat it as a module inside Local map, or cut it out behind a per-submap field-array interface. Keep scent with it: both are diffusing grids, both run under `parallel_for`.
5. **Move vehicle movement into Vehicles.** The 38 `map::*veh*` functions (`vehmove`, `move_vehicle`, collisions) are Vehicles rules. Vehicles only reaches a Deep Model fit of 3 once they move.
6. **Split Crafting out of Items** (`recipe`, `uncraft`, `requirement`, `tool_quality`, `construction` if it moves out of Map; 18 files / 10,363 lines in Items). Recipes are the second-largest JSON family (3,660 + 799 objects).
7. **Name a Scripting context (Lua)** rather than leave it in Unclaimed: 63 files / 15,894 lines, a stable language of its own (hooks, bindings), and a direct contract with Content.
8. **Move `savegame_json.cpp` (5,094 lines) to Unclaimed / Save-Load.** The content anchor pulled it into Items through migration loaders, but it serialises every context.

## 8. Open doubts

- **No `cloc`:** lines include comments. The CDDA numbers must use the same `wc -l` semantics to compare, or both should be re-run with `cloc`.
- **Engine-B vs literal rule:** the calibration pass must check that CDDA made the same `cursesdef.h` choice; otherwise the Engine totals and Creatures/Map scores do not compare.
- **The public-surface counts come from a brace and access-label tracker, not a compiler.** Templates, macros (`SET_FX`, `LUA_*`), and `#if` branches skew them. Treat them as relative, not absolute.
- **Reads vs writes of globals could not be split mechanically** (the write heuristic found 61). The M3 write judgements rest on samples.
- **Context assignment:** 70 files fell to Unclaimed only because nothing claimed them (step 4), and `iexamine.cpp`, `explosion.cpp`, `sounds.cpp`, `activity_*`, `consumption.cpp` and `savegame_json.cpp` each serve several contexts.
- **Tests per context by file name** say nothing about coverage. Without a build, there was no way to see which code the tests reach.
- **Whether the GPU path is the default in practice:** `COMPUTE_ACCELERATION=auto` uses the GPU when an SDL GPU device exists, so the CPU path's share of real play is unknown statically.
- **Thread safety:** the static view cannot confirm that `compute_plan()` and the parallel cache builds are free of data races beyond what the comments claim.

## Appendix: measurement scripts

Run each as `python3 -I <script> <clone> <out>` from a directory outside the clone; `manual.tsv` sits next to `contexts.py`.

<details><summary><code>split.py</code></summary>

```python
#!/usr/bin/env python3
"""Engine / Frontend / Content / Vendored split per docs/research/engine-scoring-rubric.md.

Usage: python3 -I split.py <clone-root> <out-dir>
Lines = physical lines (wc -l semantics: count of b'\\n'); non-blank lines also reported.
No cloc available, so comments are NOT stripped.
Writes <out>/split.tsv : path<TAB>split<TAB>lines<TAB>nonblank<TAB>reason
"""
import os, re, sys

root, out = sys.argv[1], sys.argv[2]
SRC_EXT = ('.cpp', '.h', '.hpp', '.c', '.hlsl', '.inl')
VENDORED = re.compile(r'^src/(lua|sol|third-party)/')          # as in src/CMakeLists.txt EXCLUDE REGEX
VENDORED_TOP = re.compile(r'^src/(fmtlib_[^/]*|wdirent\.h|mingw\.thread\.h|simplexnoise\.(cpp|h))$')  # third-party files kept at src/ top level (license headers: {fmt}, dirent, mingw-std-threads, Eshelman simplex noise)
FE_INCLUDE = re.compile(r'^\s*#\s*include\s*[<"]([^>"]*)[>"]', re.M)
FE_HEADER = re.compile(r'(curses|sdl|imgui)', re.I)             # cursesdef.h, cursesport.h, ncurses_def.h, sdl_*.h, sdltiles.h, SDL3/*, platform/sdl_video.h
FE_NAME = [re.compile(p) for p in (r'_ui\.', r'^cata_tiles', r'^sdltiles', r'^output', r'menu',
                                    r'^ui_', r'^panels', r'^wincurse', r'^cursesport')]

def count(path):
    with open(path, 'rb') as f:
        b = f.read()
    if b'\0' in b[:8192]:
        return None, None, b
    lines = b.count(b'\n')
    nonblank = sum(1 for l in b.split(b'\n') if l.strip())
    return lines, nonblank, b

rows = []
for top in ('src', 'data'):
    for dp, dn, fn in os.walk(os.path.join(root, top)):
        dn.sort()
        for f in sorted(fn):
            p = os.path.join(dp, f)
            rel = os.path.relpath(p, root)
            if top == 'data':
                lines, nb, _ = count(p)
                rows.append((rel, 'Content', lines, nb, os.path.splitext(f)[1] or '-'))
                continue
            if not f.endswith(SRC_EXT):
                continue
            lines, nb, b = count(p)
            if VENDORED.match(rel):
                rows.append((rel, 'Vendored', lines, nb, 'vendored dir'))
                continue
            if VENDORED_TOP.match(rel):
                rows.append((rel, 'Vendored', lines, nb, 'vendored top-level file'))
                continue
            text = b.decode('utf-8', 'replace')
            hdrs = [h for h in FE_INCLUDE.findall(text) if FE_HEADER.search(os.path.basename(h)) or FE_HEADER.search(h.split('/')[0])]
            name_hit = [r.pattern for r in FE_NAME if r.search(f)]
            if hdrs or name_hit:
                why = []
                if hdrs: why.append('include:' + ','.join(sorted(set(hdrs))))
                if name_hit: why.append('name:' + ','.join(name_hit))
                rows.append((rel, 'Frontend', lines, nb, ';'.join(why)))
            else:
                rows.append((rel, 'Engine', lines, nb, '-'))

with open(os.path.join(out, 'split.tsv'), 'w') as o:
    for r in rows:
        o.write('\t'.join(str(x) for x in r) + '\n')

tot = {}
for rel, s, l, nb, why in rows:
    t = tot.setdefault(s, [0, 0, 0])
    t[0] += 1; t[1] += l or 0; t[2] += nb or 0
print('split\tfiles\tlines\tnonblank')
for s in ('Engine', 'Frontend', 'Content', 'Vendored'):
    print(s, *tot.get(s, [0, 0, 0]), sep='\t')
# Content by extension
ext = {}
for rel, s, l, nb, why in rows:
    if s == 'Content':
        e = ext.setdefault(why, [0, 0]); e[0] += 1; e[1] += l or 0
print('content by ext:', sorted(ext.items(), key=lambda x: -x[1][1])[:6])
```

</details>

<details><summary><code>json_types.py</code></summary>

```python
#!/usr/bin/env python3
"""Count JSON `type` values under data/json (core mod 'bn' has path ../../json) and data/mods.
Usage: python3 -I json_types.py <clone-root> > types.tsv   (type<TAB>count_core<TAB>count_mods)
"""
import json, os, sys
from collections import Counter

root = sys.argv[1]
core, mods = Counter(), Counter()

def walk(base, ctr):
    for dp, dn, fn in os.walk(base):
        for f in fn:
            if not f.endswith('.json'):
                continue
            try:
                with open(os.path.join(dp, f), encoding='utf-8') as fh:
                    d = json.load(fh)
            except Exception:
                continue
            for o in (d if isinstance(d, list) else [d]):
                if isinstance(o, dict) and isinstance(o.get('type'), str):
                    ctr[o['type']] += 1

walk(os.path.join(root, 'data/json'), core)
walk(os.path.join(root, 'data/mods'), mods)
for t in sorted(set(core) | set(mods), key=lambda t: -(core[t] + mods[t])):
    print(t, core[t], mods[t], sep='\t')
```

</details>

<details><summary><code>loaders.py</code></summary>

```python
#!/usr/bin/env python3
"""Map each JSON type registered in src/init.cpp (DynamicDataLoader::initialize, add("type", loader))
to the .cpp file that defines the loader function.
Usage: python3 -I loaders.py <clone-root>   -> type<TAB>loader<TAB>defining-file(s)
"""
import os, re, sys

root = sys.argv[1]
src = os.path.join(root, 'src')
init = open(os.path.join(src, 'init.cpp'), encoding='utf-8').read().split('\n')
adds = []
for i, l in enumerate(init):
    m = re.match(r'\s*add\(\s*"([^"]+)"\s*,\s*(.*)', l)
    if not m:
        continue
    rest = m.group(2) + ' ' + ' '.join(init[i + 1:i + 3])
    rest = re.sub(r'\[\]\s*\([^)]*\)\s*\{', ' ', rest)       # strip lambda header
    sym = None
    for c in re.findall(r'&?\s*([A-Za-z_][\w]*(?:::[A-Za-z_]\w*)*)\s*[\(\);,]', rest):
        if c in ('jo', 'src', 'JsonObject', 'const', 'std', 'string', 'add', 'item_controller', 'get'):
            continue
        sym = c
        break
    if 'item_controller' in rest and sym is None:
        sym = 'Item_factory::load'
    adds.append((m.group(1), sym or '?', rest))

cpp = {}
for dp, dn, fn in os.walk(src):
    if re.search(r'/src/(lua|sol|third-party)', dp + '/'):
        continue
    for f in fn:
        if f.endswith('.cpp'):
            p = os.path.join(dp, f)
            cpp[os.path.relpath(p, src)] = open(p, encoding='utf-8', errors='replace').read()

for t, sym, rest in adds:
    if 'item_controller' in rest:
        mm = re.search(r'item_controller->(\w+)', rest)
        sym = 'Item_factory::' + mm.group(1) if mm else 'Item_factory::?'
    parts = sym.split('::')
    tail = '::'.join(parts[-2:]) if len(parts) > 1 else parts[0]
    pat = re.compile(r'^[\w:<>&*\s,]*\b' + re.escape(tail) + r'\s*\(', re.M)
    hits = [f for f, txt in cpp.items() if pat.search(txt) and f != 'init.cpp']
    if not hits and len(parts) > 1:  # template generic_factory load via static member: search class name file
        cls = parts[-2]
        hits = [f for f, txt in cpp.items() if re.search(r'\b' + re.escape(cls) + r'::', txt)][:3]
    print(t, sym, ','.join(sorted(hits)) or '?', sep='\t')
```

</details>

<details><summary><code>contexts.py</code></summary>

```python
#!/usr/bin/env python3
"""Assign Engine-B files to the seven seed contexts (rubric "Assigning code to contexts").

Usage: python3 -I contexts.py <clone-root> <out-dir>
Needs <out>/split.tsv (split.py) and <out>/loaders.tsv (loaders.py).
Engine-B = rubric Engine + files the rubric puts in Frontend ONLY because they include
cursesdef.h (BN's catacurses::window handle type) + the OVERRIDE_TO_ENGINE list below.
Steps: 1 content anchor (loader-defining .cpp + same-stem .h)
       2a the repo's own sub-directories (declared clusters)
       2b include clustering: edges (both directions) to step-1/2a files; plurality >= 2 edges and
          >= 1.5x runner-up; headers with in-degree > HUB skipped; a vote contradicting a name rule
          counts as "not settled"
       3 name regexes   4 leftovers -> Unclaimed   M manual.tsv (listed in the report)
Writes <out>/contexts.tsv (path, context, step, lines, nonblank, indegree) and <out>/edges.tsv.
"""
import os, re, sys
from collections import Counter, defaultdict

root, out = sys.argv[1], sys.argv[2]
CTX = ['Creatures', 'Terrain & Map', 'Fields', 'Items', 'Vehicles', 'Time & Weather', 'Unclaimed']
HUB = 30

OVERRIDE_TO_ENGINE = {  # Frontend by include, but simulation/loading code; SDL use is incidental
    'src/game.cpp', 'src/ballistics.cpp', 'src/player_hardcoded_effects.cpp', 'src/init.cpp',
    'src/path_info.cpp', 'src/debug.cpp',
    'src/compute/gpu_lm.cpp', 'src/compute/gpu_lm.h', 'src/compute/gpu_platform.cpp',
    'src/compute/gpu_platform.h', 'src/compute/gpu_transparency.cpp',
}

T = {}
def put(ctx, *types):
    for t in types: T[t] = ctx
put('Creatures', 'MONSTER', 'SPECIES', 'monstergroup', 'MONSTER_BLACKLIST', 'MONSTER_WHITELIST',
    'monster_adjustment', 'MONSTER_FACTION', 'monster_attack', 'harvest', 'speech', 'mutation',
    'mutation_category', 'mutation_type', 'mutation_flag', 'trait_group', 'TRAIT_BLACKLIST', 'dream',
    'bionic', 'effect_type', 'body_part', 'anatomy', 'npc', 'npc_class', 'talk_topic', 'behavior',
    'faction', 'martial_art', 'technique', 'weapon_category', 'skill', 'skill_display_type', 'skill_boost',
    'profession', 'profession_item_substitutions', 'morale_type', 'SPELL', 'enchantment',
    'enchantment_value', 'enchantment_flag', 'enchantment_condition', 'enchantment_vision', 'vitamin',
    'disease_type', 'hit_range', 'activity_type', 'scenario', 'SCENARIO_BLACKLIST')
put('Terrain & Map', 'terrain', 'furniture', 'trap', 'mapgen', 'palette', 'mapgen_color_palette',
    'overmap_terrain', 'oter_id_migration', 'overmap_land_use_code', 'overmap_connection',
    'overmap_location', 'overmap_special', 'city_building', 'map_extra', 'map_feature_description',
    'region_settings', 'region_overlay', 'ter_furn_transform', 'gate', 'construction',
    'construction_category', 'construction_group', 'rotatable_symbol', 'start_location', 'LOOT_ZONE')
put('Fields', 'field_type', 'emit', 'scent_type')
put('Items', 'AMMO', 'GUN', 'ARMOR', 'PET_ARMOR', 'TOOL', 'TOOLMOD', 'TOOL_ARMOR', 'BOOK', 'COMESTIBLE',
    'CONTAINER', 'ENGINE', 'WHEEL', 'FUEL', 'GUNMOD', 'MAGAZINE', 'BATTERY', 'GENERIC', 'BIONIC_ITEM',
    'ITEM_CATEGORY', 'MIGRATION', 'item_group', 'item_action', 'ammunition_type', 'ammo_effect',
    'material', 'tool_quality', 'requirement', 'recipe', 'uncraft', 'nested_category', 'recipe_group',
    'recipe_category', 'fault', 'clothing_mod', 'charge_removal_blacklist', 'to_cbc_migration',
    'ITEM_BLACKLIST', 'json_flag')
put('Vehicles', 'vehicle_part', 'vehicle', 'vehicle_group', 'vehicle_placement', 'vehicle_spawn',
    'vehicle_color_palette')
put('Time & Weather', 'weather_type')
put('Unclaimed', 'WORLD_OPTION', 'EXTERNAL_OPTION', 'world_type', 'snippet', 'ascii_art', 'MOD_INFO',
    'colordef', 'named_color', 'sound_effect', 'sound_effect_preload', 'playlist', 'overlay_order',
    'mod_tileset', 'achievement', 'score', 'event_transformation', 'event_statistic', 'mission_definition')

# loader resolutions that hit a generic helper or the wrong file; pinned by hand (None = anchor nothing)
LOADER_FIX = {'requirement': 'requirements.cpp', 'MONSTER': 'monstergenerator.cpp',
              'SPECIES': 'monstergenerator.cpp', 'monster_attack': 'monstergenerator.cpp',
              'region_overlay': 'regional_settings.cpp', 'construction': 'construction.cpp',
              'map_extra': 'mapgen/map_extras.cpp', 'map_feature_description': 'map/map_feature_descriptions.cpp',
              'overmap_connection': 'overmap_connection.cpp', 'rotatable_symbol': 'rotatable_symbols.cpp',
              'charge_removal_blacklist': None, 'to_cbc_migration': None,  # live in savegame_json.cpp (serialises every context)
              'sound_effect': 'sounds.cpp', 'sound_effect_preload': 'sounds.cpp', 'playlist': 'sounds.cpp',
              'trait_group': 'trait_group.cpp', 'recipe_category': 'recipe_dictionary.cpp'}

DIRS = [  # step 2a
    ('Fields', r'^src/map/(field|emit|map_field|submap_fields)'),
    ('Vehicles', r'^src/vehicle/'), ('Time & Weather', r'^src/weather/'),
    ('Terrain & Map', r'^src/(map|mapgen|travel|compute|shaders)/'),
    ('Creatures', r'^src/(magic|enchantments)/'), ('Items', r'^src/(reload|units)/'),
    ('Unclaimed', r'^src/(utils|platform)/'),
]
NAME = [  # step 3, applied to the basename, first match wins
    ('Unclaimed', r'^(game\.|game_|catalua|init\.|savegame|main|debug|options|cached_|json|filesystem|cata_|'
                  r'string_|translations|units|point|coordinates|line\.|cube|cuboid|rect_|enum|type_id|int_id|'
                  r'rng|color|input|ui\.|cursesdef|pimpl|value_ptr|ret_val|flat_set|generic_|assign|make_static|'
                  r'clone_ptr|memory_fast|numeric_|all_enum|catacharset|fstream|messages|message_types|'
                  r'event|mission|achievement|safe_reference|thread_pool|batch_turns|world|mod_|action\.|'
                  r'handle_action|computer|editmap|wish|help|popup|string_input|trade_win|title_screen|dump)'),
    ('Vehicles', r'^(veh_|vehicle|vpart|turret|wheel|grab)'),
    ('Fields', r'^(field|scent|fire|emit)'),
    ('Time & Weather', r'^(weather|calendar|season|timed_event)'),
    ('Terrain & Map', r'^(map|submap|overmap|omdata|om_|trap|construction|lightmap|shadowcasting|tileray|'
                      r'level_cache|fragment_cloud|regional|start_location|zones|clzones|explosion|gates|'
                      r'iexamine|cellular_automata|simple_pathfinding|pathfind|distribution_grid|fluid_grid|'
                      r'active_tile|flood_fill|drawing_primitives|overlay)'),
    ('Items', r'^(item|itype|iuse|ammo|gun|ballistic|projectile|magazine|material|recipe|craft|requirement|'
              r'inventory|pickup|pocket|container|visitable|clothing|armor|flag\.|fault|relic|artifact|'
              r'damage|dispersion|handle_liquid|active_item|consumption|salvage|rot\.|location|locations|'
              r'detached_ptr|drop_token|stomach|game_inventory|advanced_inv|auto_pickup)'),
    ('Creatures', r'^(monster|mon|mtype|mattack|creature|character|avatar|player|npc|bionic|mutation|effect|'
                  r'bodypart|anatomy|skill|profession|martial|magic|spell|morale|activity|addiction|disease|'
                  r'behavior|faction|talker|dialogue|speech|harvest|kill|scenario|melee|ranged|suffer|'
                  r'condition|bonuses|teleport|fungal|vitamin|trait|flag_trait|memorial|diary|distraction|'
                  r'pldata|newcharacter|regen|weighted)'),
]
def name_ctx(p):
    b = os.path.basename(p)
    for c, rx in NAME:
        if re.search(rx, b):
            return c
    return None

rows = [l.rstrip('\n').split('\t') for l in open(os.path.join(out, 'split.tsv'))]
info = {r[0]: r for r in rows}
eng = []
for r in rows:
    p, s, why = r[0], r[1], r[4]
    if not p.startswith('src/'):
        continue
    if s == 'Engine' or p in OVERRIDE_TO_ENGINE or (s == 'Frontend' and why == 'include:cursesdef.h'):
        eng.append(p)
engset = set(eng)
bybase = defaultdict(list)
for p in eng:
    bybase[os.path.basename(p)].append(p)

INC = re.compile(r'^\s*#\s*include\s*"([^"]+)"', re.M)
def resolve(frm, inc):
    cand = os.path.normpath(os.path.join(os.path.dirname(frm), inc))
    if cand in engset: return cand
    cand = os.path.normpath(os.path.join('src', inc))
    if cand in engset: return cand
    b = bybase.get(os.path.basename(inc), [])
    return b[0] if len(b) == 1 else None

edges = defaultdict(set)
for p in eng:
    txt = open(os.path.join(root, p), encoding='utf-8', errors='replace').read()
    for inc in INC.findall(txt):
        q = resolve(p, inc)
        if q and q != p:
            edges[p].add(q)
rev = defaultdict(set)
for a, bs in edges.items():
    for b in bs: rev[b].add(a)

ctx, step = {}, {}
for l in open(os.path.join(out, 'loaders.tsv')):
    t, sym, files = l.rstrip('\n').split('\t')
    if t not in T: continue
    if t in LOADER_FIX:
        files = [LOADER_FIX[t]] if LOADER_FIX[t] else []
    else:
        files = [f for f in files.split(',') if f != '?']
    for f in files:
        for p in ('src/' + f, 'src/' + f[:-4] + '.h'):
            if p in engset and p not in ctx:
                ctx[p], step[p] = T[t], '1'
for p in eng:
    if p in ctx: continue
    for c, rx in DIRS:
        if re.search(rx, p):
            ctx[p], step[p] = c, '2a'
            break
anchored = dict(ctx)
for p in eng:
    if p in ctx or len(rev[p]) > HUB: continue
    c = Counter(anchored[q] for q in (edges[p] | rev[p]) if q in anchored and anchored[q] != 'Unclaimed')
    if not c: continue
    (top, n), *rest = c.most_common()
    second = rest[0][1] if rest else 0
    nc = name_ctx(p)
    if n >= 2 and n >= 1.5 * second and (nc is None or nc == top):
        ctx[p], step[p] = top, '2b'
for p in eng:
    if p in ctx: continue
    nc = name_ctx(p)
    if nc:
        ctx[p], step[p] = nc, '3'
for p in eng:
    if p not in ctx:
        ctx[p], step[p] = 'Unclaimed', '4'

mf = os.path.join(os.path.dirname(os.path.abspath(__file__)), 'manual.tsv')
if os.path.exists(mf):
    for l in open(mf):
        if l.strip() and not l.startswith('#'):
            p, c = l.rstrip('\n').split('\t')[:2]
            if p in ctx:
                ctx[p], step[p] = c, 'M'

with open(os.path.join(out, 'contexts.tsv'), 'w') as o:
    for p in sorted(eng):
        r = info[p]
        o.write('\t'.join([p, ctx[p], step[p], r[2], r[3], str(len(rev[p]))]) + '\n')
ec = Counter()
for a, bs in edges.items():
    for b in bs:
        if ctx[a] != ctx[b]: ec[(ctx[a], ctx[b])] += 1
with open(os.path.join(out, 'edges.tsv'), 'w') as o:
    for (a, b), n in sorted(ec.items()):
        o.write(f'{a}\t{b}\t{n}\n')
tot = defaultdict(lambda: [0, 0, 0])
for p in eng:
    t = tot[ctx[p]]; t[0] += 1; t[1] += int(info[p][2]); t[2] += int(info[p][3])
print('Engine-B files', len(eng), 'lines', sum(int(info[p][2]) for p in eng))
for c in CTX:
    print(c, *tot[c], sep='\t')
print('by step', sorted(Counter(step.values()).items()))
```

</details>

<details><summary><code>manual.tsv</code></summary>

```text
# path	context	reason
src/gamemode_defense.cpp	Unclaimed	game mode, include vote pulled it to Map
src/gamemode_tutorial.cpp	Unclaimed	game mode, include vote pulled it to Map
src/math_defines.h	Unclaimed	utility header
src/path_info.h	Unclaimed	file-system paths, not map pathing
src/poly_serialized.h	Unclaimed	serialisation utility
src/preload_config.h	Unclaimed	startup config
src/consistency_report.h	Unclaimed	data-loading utility
src/weighted_list.h	Unclaimed	generic container
```

</details>

<details><summary><code>evidence.py</code></summary>

```python
#!/usr/bin/env python3
"""Per-context evidence counts (rubric "Evidence recipes"). Static, regex-based, no compiler.

Usage: python3 -I evidence.py <clone-root> <out-dir>     (needs <out>/contexts.tsv)
Prints, per context:
  include edges   internal / out (to other non-Unclaimed ctx) / in (from other non-Unclaimed ctx) / to-Unclaimed
  fan-in          distinct Engine-B files outside the context that #include one of its headers
  fan-out         distinct Engine-B files outside the context it #includes
  globals         occurrences of the global accessors in GLOBALS, and a write heuristic (g->x = ..., calendar::turn +=)
  static_mut      non-const `static` variables (file or function scope), regex heuristic
  pub             public member declarations in the context's headers (brace/access-label tracker, see pub_members())
  free_fn         free-function declarations at namespace scope in its headers
  inherit_x       `class X : public Y` where Y is defined in another context's header
  friend_x        friend declarations in its headers naming a class defined in another context
  foreign_ids     uses of other contexts' hard-coded id constants (fd_*, t_*/f_* terrain/furniture, mon_*, itype_*, vpart_*)
  tests           tests/*.cpp whose Engine-B includes vote for this context (plurality, Unclaimed ignored)
"""
import os, re, sys
from collections import Counter, defaultdict

root, out = sys.argv[1], sys.argv[2]
CTX = ['Creatures', 'Terrain & Map', 'Fields', 'Items', 'Vehicles', 'Time & Weather', 'Unclaimed']
rows = [l.rstrip('\n').split('\t') for l in open(os.path.join(out, 'contexts.tsv'))]
ctx = {r[0]: r[1] for r in rows}
files = defaultdict(list)
for p, c in ctx.items(): files[c].append(p)
bybase = defaultdict(list)
for p in ctx: bybase[os.path.basename(p)].append(p)
text = {p: open(os.path.join(root, p), encoding='utf-8', errors='replace').read() for p in ctx}

def strip(t):  # drop comments and string literals
    t = re.sub(r'/\*.*?\*/', ' ', t, flags=re.S)
    t = re.sub(r'//[^\n]*', '', t)
    t = re.sub(r'"(?:\\.|[^"\\\n])*"', '""', t)
    return t
code = {p: strip(t) for p, t in text.items()}

INC = re.compile(r'^\s*#\s*include\s*"([^"]+)"', re.M)
def resolve(frm, inc):
    for cand in (os.path.normpath(os.path.join(os.path.dirname(frm), inc)), os.path.normpath(os.path.join('src', inc))):
        if cand in ctx: return cand
    b = bybase.get(os.path.basename(inc), [])
    return b[0] if len(b) == 1 else None
inc = defaultdict(set)
for p, t in text.items():
    for i in INC.findall(t):
        q = resolve(p, i)
        if q and q != p: inc[p].add(q)

GLOBALS = {'g->': r'\bg->', 'get_map()': r'\bget_map\(\)', 'get_player_character()': r'\bget_player_character\(\)',
           'get_avatar()': r'\bget_avatar\(\)', 'get_weather()': r'\bget_weather\(\)', 'calendar::turn': r'\bcalendar::turn\b',
           'overmapbuffer': r'\b(get_overmapbuffer|ACTIVE_OVERMAP_BUFFER)\b', 'mapbuffer': r'\bMAPBUFFER(_REGISTRY)?\b',
           'item_controller': r'\bitem_controller\b', 'get_option<': r'\bget_option\s*<',
           'event_bus/grids/stats': r'\b(get_event_bus|get_distribution_grid_tracker|get_fluid_grid\w*|get_stats)\s*\('}
WRITE = re.compile(r'(\bg->[\w.\->\[\]]+\s*(=[^=]|\+=|-=|\+\+|--))|(\bcalendar::turn\s*(=[^=]|\+=|-=|\+\+))')
STATIC_MUT = re.compile(r'^\s*static\s+(?!const\b|constexpr\b|inline\b|void\b|bool\s+\w+\s*\(|cata::|thread_local\s+const)'
                        r'(?:[\w:]+(?:<[^;()]*?>)?[\s*&]+)+\w+\s*(?:=[^=;]*|\{[^;]*\}|\[[^\]]*\])?\s*;', re.M)
FOREIGN = {'Fields': r'\bfd_[a-z0-9_]+\b', 'Terrain & Map': r'\b[tf]_[a-z0-9_]{3,}\b',
           'Creatures': r'\bmon_[a-z0-9_]+\b', 'Items': r'\bitype_[a-z0-9_]+\b', 'Vehicles': r'\bvpart_[a-z0-9_]+\b'}

CLASS = re.compile(r'\b(class|struct)\s+(?:\[\[[^\]]*\]\]\s*)?(\w+)\s*(?:final\s*)?(:[^{;]*)?\{')
def pub_members(t):
    """Count public declarations inside class/struct bodies and free fn decls at namespace scope.
    A declaration = a statement at the class's own brace depth that ends in ';' or opens a body '{'
    and contains '(' (methods) or is a data member (ends with ';' and no '(')."""
    pub = free = 0
    stack = []  # (kind, access) kind in {'class','ns','other'}
    i, n = 0, len(t)
    stmt = ''
    while i < n:
        ch = t[i]
        if ch == '{':
            m = CLASS.search(stmt + '{')
            s = stmt.strip()
            if m and m.end() >= len(stmt + '{') - 1 and not re.search(r'\benum\b', s):
                stack.append(['class', 'public' if m.group(1) == 'struct' else 'private'])
            elif re.match(r'^(namespace|extern\s+"")', s) or s.startswith('namespace'):
                stack.append(['ns', None])
            else:
                if stack and stack[-1][0] == 'class' and stack[-1][1] == 'public' and '(' in s: pub += 1
                elif (not stack or stack[-1][0] == 'ns') and '(' in s and not s.startswith('#'): free += 1
                stack.append(['other', None])
            stmt = ''
        elif ch == '}':
            if stack: stack.pop()
            stmt = ''
        elif ch == ';':
            s = stmt.strip()
            if stack and stack[-1][0] == 'class':
                if stack[-1][1] == 'public' and s and not s.startswith(('friend', 'using', 'typedef', 'static_assert')):
                    pub += 1
            elif (not stack or stack[-1][0] == 'ns') and '(' in s and not s.startswith(('#', 'using', 'typedef', 'static_assert', 'extern template')):
                free += 1
            stmt = ''
        elif ch == ':' and stack and stack[-1][0] == 'class':
            m = re.search(r'\b(public|private|protected)\s*$', stmt)
            if m and (i + 1 >= n or t[i + 1] != ':'):
                stack[-1][1] = m.group(1); stmt = ''
            else:
                stmt += ch
        elif ch == '\n' and stmt.lstrip().startswith('#'):
            stmt = ''
        else:
            stmt += ch
        i += 1
    return pub, free

# class -> defining context
cls_ctx = {}
for p in ctx:
    if p.endswith(('.h', '.hpp')):
        for m in CLASS.finditer(code[p]):
            cls_ctx.setdefault(m.group(2), ctx[p])

res = {}
for c in CTX:
    fs = files[c]; S = set(fs)
    r = res[c] = Counter()
    r['files'] = len(fs); r['lines'] = sum(int(x[3]) for x in rows if x[1] == c)
    fanin, fanout = set(), set()
    for p in ctx:
        for q in inc[p]:
            if p in S and q in S: r['inc_internal'] += 1
            elif p in S:
                fanout.add(q)
                if ctx[q] == 'Unclaimed': r['inc_to_unclaimed'] += 1
                else: r['inc_out'] += 1
            elif q in S:
                fanin.add(p)
                if ctx[p] != 'Unclaimed': r['inc_in'] += 1
    r['fan_in'], r['fan_out'] = len(fanin), len(fanout)
    for p in fs:
        k = code[p]
        for g, rx in GLOBALS.items():
            r['G ' + g] += len(re.findall(rx, k))
        r['G writes(heur)'] += len(WRITE.findall(k))
        r['static_mut'] += len(STATIC_MUT.findall(k))
        if p.endswith(('.h', '.hpp')):
            a, b = pub_members(k); r['pub'] += a; r['free_fn'] += b
            r['hdr:' + p[4:]] = a + b
            for m in CLASS.finditer(k):
                if m.group(3):
                    for base in re.findall(r'(?:public|protected|private)?\s*([\w:]+)(?:<[^>]*>)?', m.group(3)[1:]):
                        b0 = base.split('::')[-1]
                        if b0 in cls_ctx and cls_ctx[b0] != c and b0 not in ('public', 'private', 'protected'):
                            r['inherit_x'] += 1; r['inherit_x:' + b0 + '(' + cls_ctx[b0] + ')'] += 1
            for m in re.finditer(r'\bfriend\s+(?:class\s+|struct\s+)?([\w:]+)', k):
                b0 = m.group(1).split('::')[-1]
                if b0 in cls_ctx and cls_ctx[b0] != c:
                    r['friend_x'] += 1; r['friend_x:' + b0 + '(' + cls_ctx[b0] + ')'] += 1
        for oc, rx in FOREIGN.items():
            if oc != c:
                r['foreign_ids ' + oc] += len(re.findall(rx, k))

# tests
tdir = os.path.join(root, 'tests')
tcount = Counter(); tfiles = defaultdict(list)
for f in sorted(os.listdir(tdir)):
    if not f.endswith('.cpp'): continue
    t = open(os.path.join(tdir, f), encoding='utf-8', errors='replace').read()
    votes = Counter()
    for i in INC.findall(t):
        q = resolve('src/x', i)
        if q and ctx[q] != 'Unclaimed': votes[ctx[q]] += 1
    if votes:
        top = votes.most_common(1)[0][0]
        tcount[top] += 1; tfiles[top].append(f)
SETUP = {'clear_map': r'\bclear_map\s*\(', 'clear_avatar/character': r'\bclear_(avatar|character)\s*\(',
         'get_map()': r'\bget_map\(\)', 'g->': r'\bg->', 'TEST_CASE': r'\bTEST_CASE\s*\('}
for c in CTX:
    s = Counter()
    for f in tfiles[c]:
        t = open(os.path.join(tdir, f), encoding='utf-8', errors='replace').read()
        for k, rx in SETUP.items(): s[k] += len(re.findall(rx, t))
        if re.search(r'\bclear_map\s*\(', t): s['files_with_clear_map'] += 1
    res[c]['tests'] = tcount[c]
    for k, v in s.items(): res[c]['T ' + k] = v

with open(os.path.join(out, 'tests_by_ctx.tsv'), 'w') as o:
    for c in CTX:
        o.write(c + '\t' + ' '.join(tfiles[c]) + '\n')
for c in CTX:
    print('==', c)
    r = res[c]
    print('  ', ', '.join(f'{k}={v}' for k, v in r.items() if not k.startswith(('inherit_x:', 'friend_x:', 'hdr:'))))
    print('   widest headers (pub+free):', ', '.join(f'{k[4:]}={v}' for k, v in r.most_common() if k.startswith('hdr:'))[:400])
    print('   headers:', sum(1 for k in r if k.startswith('hdr:')))
    print('   inherit:', ', '.join(f'{k[10:]}x{v}' for k, v in r.most_common() if k.startswith('inherit_x:')))
    print('   friend:', ', '.join(f'{k[9:]}x{v}' for k, v in r.most_common() if k.startswith('friend_x:')))
```

</details>

<details><summary><code>depmap.py</code></summary>

```python
#!/usr/bin/env python3
"""Include edges between splits (Engine-B, Frontend-B, Vendored) and per-split totals.
Usage: python3 -I depmap.py <clone-root> <out-dir>   (needs split.tsv, contexts.tsv)
Frontend-B = rubric Frontend minus the files contexts.py moved into Engine-B.
"""
import os, re, sys
from collections import Counter, defaultdict
root, out = sys.argv[1], sys.argv[2]
split = {l.split('\t')[0]: l.rstrip('\n').split('\t') for l in open(os.path.join(out, 'split.tsv'))}
engb = {l.split('\t')[0] for l in open(os.path.join(out, 'contexts.tsv'))}
side = {}
for p, r in split.items():
    if not p.startswith('src/'): continue
    side[p] = 'Engine-B' if p in engb else ('Vendored' if r[1] == 'Vendored' else 'Frontend-B')
tot = defaultdict(lambda: [0, 0, 0])
for p, s in side.items():
    t = tot[s]; t[0] += 1; t[1] += int(split[p][2]); t[2] += int(split[p][3])
for s, t in sorted(tot.items()): print(s, *t, sep='\t')
bybase = defaultdict(list)
for p in side: bybase[os.path.basename(p)].append(p)
INC = re.compile(r'^\s*#\s*include\s*[<"]([^>"]+)[>"]', re.M)
def resolve(frm, inc):
    for c in (os.path.normpath(os.path.join(os.path.dirname(frm), inc)), os.path.normpath(os.path.join('src', inc))):
        if c in side: return c
    b = bybase.get(os.path.basename(inc), [])
    return b[0] if len(b) == 1 else None
e = Counter(); files_e2f = Counter(); hdr = Counter()
for p, s in side.items():
    t = open(os.path.join(root, p), encoding='utf-8', errors='replace').read()
    hit = False
    for i in INC.findall(t):
        q = resolve(p, i)
        if q and q != p:
            e[(s, side[q])] += 1
            if s == 'Engine-B' and side[q] == 'Frontend-B':
                hit = True; hdr[q[4:]] += 1
    if hit: files_e2f['Engine-B files including a Frontend-B header'] += 1
for (a, b), n in sorted(e.items()): print(f'{a} -> {b}\t{n}')
print(dict(files_e2f)); print('top Frontend-B headers included by Engine-B:', hdr.most_common(8))
```

</details>
