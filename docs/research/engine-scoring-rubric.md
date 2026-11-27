# Engine scoring rubric

How the two architecture analyses, [How modular is the Bright Nights v0.13.0 Engine?](https://github.com/adamico/Cataclysm-RD/issues/6) and [How modular is the CDDA 0.H-RELEASE Engine?](https://github.com/adamico/Cataclysm-RD/issues/7), score a Baseline's **Engine** for **Deep Model** fit and **Deep module** quality, so that their numbers compare. Decided in [How do we score Deep Model fit and Deep module quality?](https://github.com/adamico/Cataclysm-RD/issues/3).

Vocabulary: [GLOSSARY.md](../../GLOSSARY.md) for the Port language, `/codebase-design` for module, interface, depth, seam, adapter, leverage and locality. Say "seam", never "boundary".

## Ground rules

- **Unit of scoring:** each suspected bounded context, not each file. The Engine summary is built from the per-context scores.
- **Two separate axes:** Deep Model fit and Deep module quality are never merged into one number. Weighing them is the Baseline decision's job.
- **Same seed list for both Baselines:** score exactly these, in this order:
  1. Creatures
  2. Terrain & Map
  3. Fields
  4. Items
  5. Vehicles
  6. Time & Weather
  7. **Unclaimed**: Engine code that fits none of the above (game loop, data loading, save/load, utilities). It is scored like the others, but treat a low Deep Model fit here as expected, not as a finding.

  An analysis may *propose* splits, merges or new contexts (report section 7), but it still scores the seed list as given.
- **Static only:** the evidence comes from a shallow, read-only clone in the scratchpad. No build, no instrumented coverage, no clang tooling that needs `compile_commands.json`, and no git history.
- **Every score cites evidence:** a file, a symbol, or a count from the recipes below. A score with no citation is invalid.
- **Every score carries a confidence:** `low`, `med` or `high`. Use `low` when the evidence is a sample, or when the static view can't see the thing being judged.

## Assigning code to contexts

1. **Content anchor:** list the JSON `type` values under `data/json` (and `data/mods` for the core mods). Assign each `type` family to a seed context. The C++ that loads and owns a family belongs to that context.
2. **Include clusters:** in the `#include` graph between Engine files, a file joins the context its includes and includers cluster around.
3. **Name match:** where 1 and 2 don't settle it, use the file name and its main class.
4. **Leftovers** go to Unclaimed. List any file you assigned with doubt.

A file belongs to exactly one context. Where a file clearly serves two (for example `map.cpp` holding field code), assign it to its main context and note the split under the other context's evidence.

## Engine, Frontend and Content split

- **Content:** everything under `data/`.
- **Frontend:** files under `src/` that meet any of these (patched after calibration, see [engine-calibration.md](engine-calibration.md)):
  - names that mark them as UI or rendering (`*_ui.*`, `cata_tiles*`, `sdltiles*`, `output*`, `*menu*`, `ui_*`, `panels*`, `wincurse*`, `cursesport*`);
  - a stem on the shared display and input list: `cursesdef advanced_inv.* pixel_minimap_projectors font_loader mod_tileset sdlsound color color_loader popup uistate display widget animation drawing_primitives editmap game_inventory help descriptions bodygraph veh_interact crafting_gui iuse_software_.* dialogue_win magic_teleporter_list auto_note auto_pickup distraction_manager overlay_ordering music string_editor_window input action map_item_stack item_search iteminfo_query live_view ascii_art game_ui rotatable_symbols wish player_display vehicle_display newcharacter string_input_popup ui trade_win .*_display worldfactory title_screen armor_layers`;
  - an include of a curses, SDL or ImGui header **other than `cursesdef.h`**. A file whose use of SDL is incidental (clipboard, Android JNI, a single tiles hook, sound) or a compute API (BN's `compute/gpu_*`) stays Engine; list those files.
- **Engine:** everything else under `src/`, including `src/` subdirectories that aren't Frontend by the rule above.
- **Ambiguous:** a file the rule puts in Engine although it clearly mixes in UI code. Count it as Engine and list it.
- Third-party code vendored under `src/third-party` (or similar) is excluded and reported as its own line.

## Criteria

Each criterion is scored 1–5. The anchors below describe levels 1, 3 and 5; levels 2 and 4 sit between them.

### Deep Model fit: is there a clean context here?

**M1. Concept ownership.** Each domain concept of the context has one home in the code.
- **1:** the main concepts have no home. Their state and rules are spread over many files and classes, and no one place can answer "what is a X".
- **3:** the main concepts have a home class or file, but some of their rules live in other contexts (special cases in `game.cpp`, `map.cpp`, `character.cpp`, and so on).
- **5:** every main concept has one owner, and code elsewhere uses it rather than reimplementing its rules.

**M2. Language alignment.** The names in the code match the Content terms.
- **1:** the code and the JSON use different words for the same thing, or the same word for different things, across most of the context.
- **3:** the main concepts match, but secondary names drift (legacy names, abbreviations, one word meaning two things).
- **5:** the JSON `type` values, JSON fields and C++ names line up almost one to one.

**M3. Context isolation.** How much shared or global state reaches across the context.
- **1:** the context reads and writes global state freely (`g->`, `get_map()`, `get_player_character()`, `get_avatar()`, singletons, static mutable data), and other contexts reach into its internals the same way.
- **3:** global access is common, but it is mostly reads, or it is funnelled through a few call sites.
- **5:** the context gets what it needs through parameters or a narrow handle, and no other context touches its state except through its interface.

### Deep module quality: how good is the seam?

**Q1. Leverage.** The behaviour a caller can reach per unit of interface they have to learn.
- **1:** shallow. The interface is about as large as the implementation, with many getters, setters and pass-throughs, and callers do the real work.
- **3:** some deep entry points (one call does a lot), mixed with a wide shallow surface.
- **5:** a few entry points reach most of the context's behaviour. The deletion test says the complexity would reappear across many callers.

**Q2. Interface size.** Public symbols plus everything a caller must know: invariants, ordering rules, required setup, error modes.
- **1:** a very large public surface (hundreds of public members across the context's headers), with hidden ordering rules (caches that must be invalidated by hand, "call X before Y").
- **3:** a moderate surface, with a few ordering rules that are documented or obvious.
- **5:** a small surface, with no hidden ordering rules a caller can break.

**Q3. Locality.** Whether change and knowledge for the context stay inside it. Without history, judge this from structure: would a typical change to this context's rules touch only its files?
- **1:** a typical rule change has to touch many files outside the context. The context's knowledge is duplicated elsewhere (switches on its types in other contexts, copies of its constants).
- **3:** most changes stay inside, but a known set of outside files has to change with it.
- **5:** changes stay inside the context's files. Outside code depends only on its interface.

**Q4. Seam cleanliness.** Whether the context could be swapped behind its interface.
- **1:** no seam is possible without a large refactor: deep inheritance shared with other contexts, `friend` access, other contexts holding raw pointers into its storage, globals in both directions.
- **3:** a seam is possible, but one or two leaks have to be cut first (a shared base class, a cache other contexts poke at).
- **5:** a seam could be placed today: one interface, and nothing else crosses it.

**Q5. Testability.** Whether tests reach the context through its interface.
- **1:** no tests, or tests that need the whole game set up and assert on internals.
- **3:** tests exist and mostly go through public calls, but they need a large shared fixture (a full map, a full player).
- **5:** tests exercise the context through its interface with a small setup.

## Evidence recipes

Run the same recipes on both Baselines, and report the commands or scripts in the report header so the numbers can be reproduced.

| Evidence | How to measure | Feeds |
|---|---|---|
| Lines and files | `cloc` or `wc -l` over each split and each context, excluding blank lines and comments if `cloc` is available | stats, all |
| Include graph | grep `#include "..."` in `src/`; edges between contexts, counted in both directions | M3, Q3, Q4, context assignment |
| Fan-in and fan-out | ctags, or grep for the context's main public symbols; count the files outside the context that use them (fan-in), and the outside symbols the context uses (fan-out) | Q1, Q3 |
| Global access | count `g->`, `get_map()`, `get_player_character()`, `get_avatar()`, `get_weather()`, `calendar::turn`, and any `static` mutable or singleton accessors, per context, split into reads and writes where a sample allows | M3, Q4 |
| Public interface size | count the public members in the context's headers (classes and free functions) | Q1, Q2 |
| Inheritance and `friend` | the class hierarchies that cross contexts, and the `friend` declarations that cross contexts | Q4 |
| Content `type` families | the JSON `type` values per context, and how closely the C++ names follow them | M1, M2 |
| Tests per context | the files under `tests/` that mainly exercise the context, and how much setup they need (sample a few) | Q5 |
| Cross-context type switches | `switch` statements or `if` chains on another context's types or IDs | M1, Q3 |

Counts are evidence, not scores. Use them to place a context against the anchors, and cite them.

## FFI cut-cost notes

For each context, record (but don't score) what cutting a `cxx` seam there would cost. The prior-art research found `cxx` weak on these:

- STL containers crossing the would-be seam (`std::map`, `std::unordered_map`, `std::list`, nested containers);
- class hierarchies and virtual dispatch crossing it;
- shared mutable state crossing it (globals, raw pointers or references into another context's storage);
- hot loops that would cross it.

These notes feed [Incremental FFI port or rewrite?](https://github.com/adamico/Cataclysm-RD/issues/10).

## Calibration

The two analyses run independently, so their use of the anchors will drift. After both finish, one calibration pass reads both reports side by side, context by context. It adjusts any score where the anchors were applied unevenly and records every change with a reason. This pass is the first step of [Which Baseline: Bright Nights v0.13.0 or CDDA 0.H-RELEASE?](https://github.com/adamico/Cataclysm-RD/issues/9), not an analysis of its own.

## Report template

Each analysis hands back one Markdown report, with these sections in this order.

```markdown
# <Baseline> Engine analysis

## 1. Header
Baseline, commit, date of the analysis, and method: the clone command and the measurement commands or scripts used.

## 2. Code statistics
| Split | Files | Lines |
|---|---|---|
| Engine | | |
| Frontend | | |
| Content | | |
| Vendored (excluded) | | |

Engine lines and files per context. The classification rule, if it differs from the rubric anywhere. The ambiguous files.

## 3. Per-context scorecards
### <Context>
Files assigned (or a pointer to a list). Doubtful assignments.

| Criterion | Score (1–5) | Confidence | Evidence |
|---|---|---|---|
| M1 Concept ownership | | | |
| M2 Language alignment | | | |
| M3 Context isolation | | | |
| Q1 Leverage | | | |
| Q2 Interface size | | | |
| Q3 Locality | | | |
| Q4 Seam cleanliness | | | |
| Q5 Testability | | | |

(repeat for each of the seven contexts)

## 4. Summary matrix
| Context | M1 | M2 | M3 | Deep Model median | Q1 | Q2 | Q3 | Q4 | Q5 | Deep module median |
|---|---|---|---|---|---|---|---|---|---|---|

No grand total.

## 5. FFI cut-cost notes
Per context: containers, hierarchies, shared state and hot loops that would cross a seam.

## 6. Cross-cutting seams
BN: threading, GPU lighting, Lua. CDDA: the 2023–24 performance work (vision transparency cache, monster planning, pathfinder). How each cuts across the contexts.

## 7. Context proposals
Splits, merges or new contexts, each with evidence. These feed "Which bounded contexts make up the Engine?"

## 8. Open doubts
What the static view couldn't settle.
```
