# Baseline candidates for the Rust port

Status: research, 2026-11-27. Repo metadata, docs, changelogs and git-tree sizes only. No source-code architecture review yet; that comes with the architecture analysis. Terms are defined in [GLOSSARY.md](../../GLOSSARY.md).

**Shortlist (decided 2026-11-27):** Bright Nights v0.13.0 and CDDA 0.H-RELEASE go to the architecture analysis. 0.G and TLG are dropped: 0.H-RELEASE beats 0.G on every point we care about, and TLG is kept only as a content reference.

Question: which upstream release should be the **Baseline** for a Rust port whose main goals are porting the **Engine** and raising **Turn throughput** in **Dense areas** (labs)?

## At a glance

|                         | Bright Nights v0.13.0                                                             | CDDA 0.G "Gaiman"                      | CDDA 0.H-RELEASE "Herbert" _(new)_ | TLG rolling build                                   |
| ----------------------- | --------------------------------------------------------------------------------- | -------------------------------------- | ---------------------------------- | --------------------------------------------------- |
| Frozen commit           | `e7b4976` (2026-09-25)                                                            | `d6ec466140` (2023-02-28)              | `08f04fd07f` (2024-11-23)          | `f50031b` (2026-11-23)                              |
| Release model           | stable every 2–6 months + nightlies                                               | stable, no point releases              | stable                             | rolling only, no stables                            |
| src `.cpp`/`.h`         | 996 files / 21.5 MB                                                               | 784 / 19.4 MB                          | 816 / 20.6 MB                      | 860 / 22.5 MB (excl. third-party)                   |
| data/json               | 31.6 MB                                                                           | 33.4 MB                                | n/a                                | 35.6 MB                                             |
| Mods                    | 80                                                                                | ~45                                    | n/a                                | 14                                                  |
| C++ standard            | C++23                                                                             | C++14                                  | C++17 [inference]                  | C++17                                               |
| Frontend                | SDL3 (+ Vulkan GPU path), curses                                                  | SDL2, curses                           | SDL2, curses, ImGui starting       | SDL3, curses, ImGui removed                         |
| Scripting               | Lua 5.3 + sol2, deep bindings                                                     | EOC (JSON)                             | EOC                                | EOC                                                 |
| Tests (Catch2)          | 204 test files                                                                    | 1,201 TEST_CASEs                       | more than 0.G                      | **none** (suite deleted 2025-11)                    |
| Labs                    | yes, modular and redesigned (72 files)                                            | yes, data-driven, modular              | yes                                | yes, "old-style" restored, fewer subway-lab zombies |
| Dense-area perf work    | heavy: thread pool, monster AI LOD, async chunk loader, GPU lighting, 89 perf PRs | **little**: predates most DDA perf PRs | includes the 2023–24 DDA perf wave | some DDA perf backports                             |
| Active (last 12 months) | 2,616 commits, 117 authors                                                        | frozen                                 | frozen                             | 5,234 commits, 71% by one author                    |
| AI policy upstream      | **allowed with disclosure** (`Assisted-by:` trailer, human-written PR summary)    | banned                                 | banned                             | banned                                              |
| License                 | CC-BY-SA 3.0 game; AGPL-3.0 docs/scripts; CC-BY-SA 4.0 Magical Nights mod         | CC-BY-SA 3.0                           | CC-BY-SA 3.0                       | CC-BY-SA 3.0                                        |

## Bright Nights

- **What it is:** a 2020 fork of CDDA taken just after 0.E, before the pocket system. It emphasises gameplay over realism and leans more sci-fi. It is 9,880 commits ahead of its fork point, with a small core team (scarf005, Coolthulhu, chaosvolt) and 117 authors in the last year.
- **Its engine has already moved toward this project's goal:**
  - a dynamic reality bubble with a chunk loader and async mapgen (#8252, #8355);
  - a thread pool and a level of detail for monster AI (#8203);
  - GPU lighting and transparency with a CPU fallback;
  - SQLite world storage and save format v2.
  - One PR made waiting near fire up to 10x faster (#11049).
- **A Rust attempt already exists:** #3766, a "rewrite it in rust" proof of concept from 2023 by its lead maintainer. It was closed with "won't happen till build system is unified".
- **Content:** a different item and container model, since it predates pockets. Mutations, food, ranged combat and explosives are reworked. It adds electric and fluid grids, dimensions, and sound in decibels.
- **Risks:**
  - **Moving target:** about 1,000 PRs per release, including breaking changes to the engine.
  - **Port fidelity is harder:** multithreading, GPU compute and Lua hooks make **Gameplay equivalence** harder to check.
  - **License:** the signals are mixed, with AGPL for docs and scripts and a Flathub listing that says AGPL code.
  - **Style:** its AGENTS.md rules are strict (C++23 idioms, Conventional Commits).
- **What makes it unique:** it is the only candidate whose upstream openly accepts AI-assisted work. Fixes the port finds could go back upstream, with disclosure.

## CDDA 0.G "Gaiman"

- **What it is:** the smallest modern DDA. EOCs, proficiencies, power grids and data-driven modular labs are all in place.
- **Its main weakness:** it predates the 2023–24 dense-area performance work. Examples include #66889 ("multi% cpu wins in congested places"), #67535 (waiting near NPCs up to 2x faster), monster planning (#69176), creature iteration (#69574), the pathfinder (#70639) and the vision transparency cache (#72542). A port from 0.G would have to rediscover all of it.
- **Other drawbacks:**
  - 3D vision was still buggy.
  - It's C++14 with ad-hoc curses/SDL UI code.
  - There were no point releases, so known 0.G bugs stay unfixed.
  - `src/` is flat, with Frontend and Engine files side by side, so any split will be by file, not by directory.
- **New option, 0.H-RELEASE:** about 6% bigger than 0.G, and it carries the "massive performance improvements" and reliable 3D vision. It is a strictly better DDA Baseline than 0.G for about the same porting cost.

## The Last Generation

- **What it is:** a fork of CDDA `2ba772a1dd` (2024-03-04), plus cherry-picked DDA backports. Its goal is depth, plausibility and life-sim progression.
- **What it cut is content, not engine:** portal storms, Exodii, power armor, fungaloids, skill rust and most mods are gone. The C++ engine is still DDA-2024, and it is no smaller than DDA's.
- **Red flags:**
  - **Bus factor:** about 1 (71% of commits by worm-girl).
  - **No tests:** the whole test suite was deleted, which removes the reference a port would check against.
  - **No stable releases:** rolling builds only.
  - **Backports:** some arrived out of order.
  - **AI:** a written policy refuses all AI-generated submissions.
- **What it offers:** content lessons. One is that lab density was relieved by having fewer zombies in subway labs, not by faster algorithms.

## Reading across the candidates

1. **Engine size barely varies:** 19.4–22.5 MB of source across all four. No candidate is meaningfully cheaper to port by size, so the choice turns on _modularity_ (the architecture analysis) and _what the engine already does well_.
2. **0.H-RELEASE dominates 0.G:** about the same size, with the performance work and vision fixes included.
3. **TLG falls behind on every engine-relevant axis:** same engine, no tests, a single maintainer, and a hostile AI policy. It is worth keeping as a reference for content, not as a Baseline.
4. **Bright Nights is the strongest on the project's goals:** dense-area performance, an open stance on AI, modern C++, and a team that has already considered Rust. Its threading, GPU paths and Lua raise the cost of proving **Port fidelity**, and it moves fast. Those are reasons to freeze hard on v0.13.0, not reasons to reject it.
5. **Still open:** none of this measures modularity. The scores for **Deep Model** fit and **Deep module** quality come from the architecture analysis.

## Sources

Sources are the `gh api` calls on `cataclysmbn/Cataclysm-BN`, `CleverRaven/Cataclysm-DDA` and `Cataclysm-TLG/Cataclysm-TLG` (repos, releases, tags, compare, git trees, pulls, search), the local git history (tags 0.A–0.G), [Flathub: BN](https://flathub.org/en/apps/org.cataclysmbn.CataclysmBN), [DDA development process](https://docs.cataclysmdda.org/development_process.html), BN `docs/en/contribute/contributing.md`, TLG `doc/CHANGELOG.md`, `doc/AI_POLICY_STATEMENT.md` and `doc/BACKPORTING.md`, and DDA `CONTRIBUTING.md` and `CODE_OF_CONDUCT.md`.
