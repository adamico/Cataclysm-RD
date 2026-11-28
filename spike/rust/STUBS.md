# Fields spike — Rust workspace, rule coverage and stubs

Session 2 of issue #14. Baseline BN `e7b4976`. Build with `cargo build --offline`;
`cargo test --offline` reads BN's `data/json/field_type.json` from `$BN_ROOT` and
skips the Content tests if it is missing.

| crate | what |
|---|---|
| `bn_dice` | `Dice` trait, one method per BN helper (`rng`, `rng_float`, `one_in`, `x_in_y`, `dice`, `roll_remainder`), plus `random_entry*`. `BnDice` composes them like `rng.cpp` over a toy engine (not mt19937). |
| `fields` | the translation. No global RNG anywhere: every drawing function takes `&mut dyn Dice`. |
| `bubble_stub` | Reality-bubble stub (`Region`), terrain catalogue, items, caches, bubble-map geometry, smoke tests. |
| `combat_stub` | `RecordingCombat`: records every call that harms or changes a creature, does nothing else. |

## Rules: what was translated

Every rule in the Fields files, in BN order. "Translated" means rule by rule,
same branches, same order of dice calls with the same arguments.

| BN rule (file:line at `e7b4976`) | Rust | status |
|---|---|---|
| `field_type::load`, `finalize`, ids by load order (`field_type.cpp`) | `content::FieldTypes::load_json` | translated (gameplay members; see "not translated") |
| `field_entry` accessors, `set_field_intensity` clamp + `is_alive` (`field.cpp`) | `field::FieldEntry` | translated |
| `field::find_field` (guarded by displayed type), `add_field` (intensity / duration stacking), `remove_field` (displayed recompute) | `field::Field` | translated |
| `process_fields_in_submap`: dead-entry cleanup, transparency dirtying, `has_fire`, newborn suppression (`map_field.cpp:1020–1075`) | `process::process_fields_in_submap` | translated |
| intensity upgrade (`:1075`) | same | translated |
| underwater age speed-up (`:1086`) | same | translated |
| fire: age clamp, item burning loop, terrain fuel, ash/dirt conversion, age bookkeeping, z-rise, flashpoint, intensity growth, spreading, z-fall (`:1091–1324`) | same | translated; item internals stubbed (Items) |
| gas spreading: fall first, then 8 dirs, `gas_can_spread_sub`, `gas_spread_sub`, outdoor speed-up (`:1327–1360`, `:977–1013`) | same | translated |
| fungicidal gas on fungal terrain/furniture (`:1363`) | same | translated |
| electricity: arcing through impassable tiles, grounding, random spread (`:1374–1422`) | same | translated |
| fire vent, flame burst, incendiary, gas remover, shock vent bolts, acid vent, push items, bees, radiation, wandering fields (`:1425–1634`) | same | translated (gas remover: see divergences) |
| aging + half-life decay `dice(2, age) > half_life` (`:1637`) | same | translated |
| field cache compaction (`:1677`) | `process::compact_field_cache` | translated |
| `neighbor_tile`, `sub_add_field`, `sub_passable`, `mark_field_cache_dirty` (`:940–974`, `:926`) | `SubmapCtx` methods | translated |
| `map::create_hot_air` (`:146`) | `map_rules::create_hot_air` | translated |
| `map::emit_field` (`:831`) | `map_rules::emit_field` | translated; `emit` data passed in (`Emit`), not loaded |
| `map::propagate_field` (`:841`) | `map_rules::propagate_field` | translated, with libstdc++ heap order (`heap.rs`) |
| `map::get_wind_blockers` (`:800`) | `map_rules::get_wind_blockers` | translated (returns points, not `maptile`s) |
| `map::add_field`, `mod_field_intensity`, `get_field_intensity` → `mapbuffer::add_field / set_field_intensity / remove_field` (`map.cpp:6028–6122`, `mapbuffer.cpp:2209–2271`) | `MapCtx` methods | translated (these live outside the Fields files, but the map-level rules go through them) |
| `map::player_in_field`, `burn_body_part` (`:116`, `:183–526`) | `creatures::player_in_field` | translated; harm via Combat |
| `map::creature_in_field` (`:528`) | `creatures::creature_in_field` | translated; harm via Combat |
| `map::monster_in_field` (`:587`) | `creatures::monster_in_field` | translated; harm via Combat |
| `batch_turns_field`, `compute_field_decay` (`batch_turns.cpp`) | `batch::batch_turns_field` | translated, **Approximation, opt-in** (`BatchDecay::Approximate`; default `Off`) |
| `ter_furn_has_flag`, `check_flammable` (`:134–141`) | inlined | translated |

### Not translated (no gameplay rule, or not Fields)

- `field_entry::color/symbol/light_emitted/local_light_override/translucency/convection_temperature_mod/monster_spawn_*`
  accessors that no Fields rule calls: data is loaded (except colour, symbol and light colour), accessors not written.
- `field::displayed_description_affix`, `description_affixes`, `io::enum_to_string` specialisations, `field_type::check`, `bash_info`,
  `npc_complain`, `looks_like`, `get_field_type_by_legacy_enum`: UI, debug or other contexts.
- `create_burnproducts` (`:100`): an item/material rule. Behind `Items::burnproducts`.
- `batch_turns_items`, `run_submap_batch_turns`: items and vehicles, not Fields (and `batch_turns_items` is disabled in BN).
- What BN itself skips outside the bubble (vehicle fire damage, hot air from vents, monster spawning, fungal haze spread, item detonation): nothing to translate.

## Stubs (narrow traits in `fields/src/ports.rs`)

| port | stands in for | stub implementation | what the stub does not do |
|---|---|---|---|
| `SubmapStore` | `submap` + `mapbuffer::lookup_submap_in_memory` (terrain, furniture, radiation, `floor_cache`, `is_uniform`) | `bubble_stub::Region` (`BTreeMap<SmPos, SubmapSlot>`) | no Reality bubble, no loading, no dimensions; `floor_cache` is never rebuilt (all false, as zero-initialised in BN) |
| `MapData` | `ter_t` / `furn_t` flags and `movecost`, `t_pit`, `t_dirt`, `f_ash`, `f_null` | `StubMapData`: 11 terrains, 5 furniture, hand-set | not loaded from BN JSON; flags and costs are illustrative |
| `Items` | `submap::get_items`, `item::burn`, `weight`, `flammable`, `typeId() == rock`, `age`, `set_age`, `contents.clear_items`, `create_burnproducts` | `StubItems` | `burn` is recorded only: never destroys, produces no fuel or smoke, makes **no draws** (BN's `item::burn` draws) |
| `BubbleCaches` | `submap_loader.is_properly_requested`, `map::set_transparency_cache_dirty`, `set_seen_cache_dirty`, `mapbuffer::invalidate_active_field_*_caches` | `StubCaches` (records) | nothing is requested by default, matching the harness (outside the bubble) |
| `BubbleMap` | `map::impassable`, `has_flag(PERMEABLE)`, `obstructed_by_vehicle_rotation`, `rl_dist` (+ `trigdist` option), `veh_at` | `StubBubbleMap`: hand-set point sets | impassable/permeable are not derived from terrain (see "awkward" in the log); vehicle rotation never obstructs |
| `Combat` | every creature query and every harm in `player_in_field`, `monster_in_field`, `creature_in_field` | `combat_stub::RecordingCombat` | records sinks; `deal_damage` returns the amount, `add_env_effect` and `irradiate` return true, `make_fungus` false, `get_random_body_part` "torso"; **no draws** (BN's `add_env_effect`, `deal_damage`, `get_random_body_part`, `teleport`, `irradiate` can draw) |
| `Clock` | `calendar::turn`, `action_time_scale::calendar_turns_this_tick`, `once_every_this_tick` | plain struct | — |
| `Emit` | `emit_id` data | plain struct | emits are not loaded |

## Divergences and BN quirks kept on purpose

- **Kept:** bees spread to *every* free neighbour (BN keeps looping after the source is spent; later copies get the clamped intensity 1).
- **Kept:** `player_in_field` bees re-roll `rng(1, 7)` on every loop test.
- **Kept:** `fd_acid_vent`'s refill never fires (`acid->get_field_intensity() == 0` is never true, since stored intensity is clamped to ≥ 1).
- **Kept:** `ter`/`frn` are bound before the fire rule; later checks in the same entry see the *old* terrain after `set_ter`.
- **Kept:** `fd_gas_remover` removes entries without decrementing `field_count`.
- **Kept:** `creature_in_field` rolls an environmental effect's duration twice (`get_effect()` then `get_duration()`).
- **Kept:** `one_in(chance <= 1)` is still a helper call (recorded by `bn-draws/1` with no nested draw), so Rust calls `dice.one_in(0)` too.
- **Kept:** `create_hot_air` reads `point(rng(-1, 1), rng(-1, 1))` left to right, which is clang's order (the Reference build); C++ leaves it unspecified.
- **Diverges (UB in BN):** `fd_gas_remover` erases from a `std::map` inside a range-for over it, then increments the erased iterator. Rust removes every dangerous gas on the tile, which is what the code means; BN's behaviour after the erase depends on allocator state.
- `propagate_field` depends on the tie order of `std::priority_queue`; `heap.rs` re-implements libstdc++'s `__push_heap`/`__adjust_heap`, checked against clang++/libstdc++ output in `heap_tie_order_matches_libstdcxx`.

## Session 3: stubs under replay

The replay tests (`fields_harness/tests/replay.rs`, findings in
`REPLAY_FINDINGS.md`) answer every cross-context call from the recorded case:

| port | live (soak, smoke tests) | replay |
|---|---|---|
| `MapData` | the hand-made `StubMapData` | built from the case's `ter_def` / `furn_def` marks, so flags are BN's own. The hand-made catalogue was wrong for `t_grass`: BN `e7b4976` gives it no flammable flag. |
| `Items` | `burn` records, destroys nothing, no products | `burn` checks its inputs against the recorded `item::burn` call, then takes `destroyed`, the weight after, `fuel_produced`, `smoke_produced` and the burn products from it. Its draws are consumed. |
| `Combat` | `RecordingCombat` | `fields_harness::creature::ReplayCombat`. Queries come from `creature_def`. Every wrapped sink must be the next recorded call with the same arguments. `deal_damage` (virtual in BN) consumes the draws and calls whose BN call site isn't Fields code. `teleport` moves the creature where BN put it. |
| `Dice` | `BnDice` (SplitMix64) | `fields_harness::replay::ReplayDice` (`d == 0` draws, exact arguments). `Dice::foreign` is the new hook that hands a stub its recorded call. |

Ports changed in session 3:
- `Items::burnproducts` takes `&mut self`.
- `Dice` gained `foreign()`, defaulting to `None`.

Translation fix: `field_effect.is_environmental` defaults to false
(`content.rs`), as BN's loader does.

Correction to session 2: BN's engine is `std::minstd_rand0` (`rng.h:26`), not
mt19937. Replay is helper-level, so nothing depends on it. It does matter for
soak seeds (`spike/soak/SOAK_RESULT.md`).
