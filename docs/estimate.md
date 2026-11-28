# Phase 0 estimate

Status: written 2026-11-28 for [Write the Phase 0 estimate](https://github.com/adamico/Cataclysm-RD/issues/11). It feeds [Go or no-go?](https://github.com/adamico/Cataclysm-RD/issues/12). Terms are in the [Rusted Dawn glossary](../GLOSSARY.md) and the [Game glossary](game/GLOSSARY.md).

It costs a **rewrite** of the Bright Nights v0.13.0 Engine ([ADR 0001](adr/0001-bright-nights-baseline.md), [ADR 0003](adr/0003-rewrite-with-reference.md)). The rewrite goes context by context ([ADR 0002](adr/0002-contexts-by-language.md)) and is checked by replaying recorded draws ([ADR 0004](adr/0004-replay-recorded-draws.md)). It stops at a headless Engine that holds Gameplay equivalence. The Frontend, profiling and the Benchmark save are not in it.

## The two figures (D3)

|                                                                | Point                                             | Range     | What it means                                                                                                                                                                                                                                                                                                                                   |
| -------------------------------------------------------------- | ------------------------------------------------- | --------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Experienced human team** (counterfactual)                    | **≈ 660 person-months**                           | 450–1,000 | Five to nine years for a team of eight. It is the baseline the AI figure is measured against, not a plan.                                                                                                                                                                                                                                       |
| **AI-assisted:** one person and a swarm of agents in a harness | **≈ 9.5 person-months** (≈ 1,335 h) of human time | 7–21      | Calibrated on the Fields spike. The person decides, and reads architecture, interfaces and oracle reports, never function bodies. It also needs about 360+ agent-hours. Token cost was not measured.                                                                                                                                            |
| AI-assisted, pessimistic bound                                 | ≈ 340 person-months                               | —         | The human figure with the published per-kind factors applied ([AI speedup research](https://github.com/adamico/Cataclysm-RD/blob/research/ai-assisted-port-speedup/docs/research/ai-assisted-port-speedup.md)). This is what you get if, past Fields, agents can't carry the work behind the oracle and humans have to write and read the code. |

The AI figure and its pessimistic bound are about 35x apart, and that gap is the main finding. The published factors (about 1.5–3x overall) come from 2024–25 tools and assume that humans still do most of the work. The spike measured current agents doing all of the translation, oracle and design drafting, with the human only reviewing. That was **one easy context**. Before the AI figure can be trusted, it has to be confirmed on a hard one.

| Hours a month on the port | Calendar time at 9.5 person-months |
| ------------------------- | ---------------------------------- |
| 140 (full-time)           | ≈ 10 months                        |
| 60                        | ≈ 22 months                        |
| 30                        | ≈ 45 months                        |

### What a 60 h month buys

The go is one month at a time, at 60 h a month ([ADR 0005](adr/0005-gated-go-on-monsters.md)). At the human-hour rates below, plus the 30% for running the harness and integration:

| Piece                                                             | Human hours | Months at 60 h                                |
| ----------------------------------------------------------------- | ----------- | --------------------------------------------- |
| An easy context (Weather, Magic, Records, Crafting)               | 7–12 each   | ≈ ⅙                                           |
| A medium context                                                  | 40–110      | 1–2                                           |
| **Monsters** (hard)                                               | **≈ 120**   | **≈ 2**                                       |
| Reality bubble (hard)                                             | ≈ 250       | ≈ 4                                           |
| Building the harness                                              | 10          | ⅙                                             |
| Profiling and the Benchmark save (outside this estimate; a guess) | ≈ 12        | ⅕                                             |
| Deep Model skeleton, whole                                        | 120         | 2, but built slice by slice with each context |

- **Month 1:** profiling and the Benchmark save (≈ 12 h); the spike's Rust Fields against BN's Fields on a field-dense lab, as field-processing time per turn (≈ 6 h); the harness, and the recorder tagging draws by replay unit (≈ 13 h); the Monsters design decisions and their slice of the skeleton (≈ 29 h).
- **Month 2:** Monsters' translation, oracle and interfaces. Its rates are the gate.

## Size of the job

Lines are `wc -l`, comments included, following the [calibration](research/engine-calibration.md) split rule. The calibration's seven seed contexts were recounted into the sixteen contexts of ADR 0002, by file stem. The recount comes to 755 files and 398k lines, about 6% under the calibrated 422k, because vendored Lua and fmtlib are left out. It is accurate to about ±10% per context: some files serve two contexts and were assigned to their main one.

| Context          | k lines | Tier   | Why that tier                                                                         |
| ---------------- | ------- | ------ | ------------------------------------------------------------------------------------- |
| Fields           | 3.9     | easy   | Measured by the spike. One Tangle (storage in `submap`).                              |
| Weather          | 2.3     | easy   | Deep module median 3. `weather_gen` bypasses the dice helpers (statistical-only).     |
| Records          | 8.1     | easy   | Leaf code that observes events.                                                       |
| Magic            | 10.0    | easy   | Self-contained, driven by Content.                                                    |
| Crafting         | 11.0    | easy   | Requirements and recipes, few Tangles.                                                |
| Items            | 53.1    | medium | About 3.4k public declarations and 500 `g->` uses.                                    |
| Characters       | 64.0    | medium | `character.cpp` alone is 13k lines. Weather rules leak in.                            |
| NPC society      | 21.0    | medium | Dialogue and missions are mostly data-driven.                                         |
| Combat           | 13.4    | medium | Harm from every source gathers here.                                                  |
| Perception       | 10.5    | medium | Visibility caches are the worst cross-cutting seam.                                   |
| World generation | 30.9    | medium | Overmap `rand()` bypasses the helpers. About half is statistical-only.                |
| Reality bubble   | 47.6    | hard   | `map`/`submap` own other contexts' state. Excludes 8.6k of GPU lighting.              |
| Monsters         | 24.6    | hard   | Plan/apply restructuring, hordes on the overmap (Tangle).                             |
| Vehicles         | 20.7    | hard   | Vehicle movement written as `map::` functions (Tangle).                               |
| Turn loop        | 9.1     | hard   | Dissolves the `game` god object.                                                      |
| Machinery        | 43.4    | —      | JSON, filesystem, strings, save/load: mostly replaced by Rust crates, not translated. |
| Scripting        | 16.0    | —      | Not translated: the Lua API is redesigned onto the Deep Model (fixed lines below).    |

That is about 390k lines to port, about 334k without comments. A tier is a whole-context multiplier: medium costs about 1.5x easy per line, and hard about 2.4x.

## Human team figure

Per 1,000 BN lines, in person-months, for an experienced team without AI:

| Kind of work                                                                                    | easy     | medium   | hard     | Machinery |
| ----------------------------------------------------------------------------------------------- | -------- | -------- | -------- | --------- |
| Translation, reading included                                                                   | 0.25     | 0.30     | 0.35     | 0.15      |
| Oracle, per context (Observables, recorded cases, replay tests, code that bypasses the helpers) | 0.40     | 0.55     | 0.70     | 0.10      |
| Untangling and design (Tangles, ports, Deep Model shape)                                        | 0.06     | 0.20     | 0.60     | 0.05      |
| Refactor pass (dedupe, structure instead of comments)                                           | 0.12     | 0.18     | 0.25     | 0.05      |
| Review                                                                                          | 0.05     | 0.07     | 0.10     | 0.03      |
| Debugging behavioural differences                                                               | 0.08     | 0.15     | 0.30     | 0.05      |
| **Total**                                                                                       | **0.96** | **1.45** | **2.30** | **0.43**  |

- **Translation** is about 4,000 BN lines a person-month (200 a day). It covers only a literal translation into Rust that compiles, in the shape the context's ports dictate. Reshaping to the Deep Model is in design, and deduplication is in the refactor pass. Ladybird's author estimated about 5–8k lines a month on code he wrote himself, and a team new to BN would be slower.
- **Oracle per context** is 1.6x translation, which is the spike's ratio (27 min against 17). The spike found the oracle costs more than the translation, and nothing here suggests humans would reverse that.
- **Design** grows fastest with the tier. Each Tangle is costed inside its context's design line: field storage (Fields), vehicle movement (Vehicles), hordes (Monsters), the `iexamine` rules (Reality bubble), Weather leaks (Characters, Items) and `game` (Turn loop).

| Line                                                                                                              | Person-months |
| ----------------------------------------------------------------------------------------------------------------- | ------------- |
| Sixteen contexts (table above × the rates)                                                                        | 567           |
| Workspace and Deep Model skeleton: crates, ports, dice source, plan/apply, thread pool                            | 6             |
| Oracle, fixed part: recorder per context, Observable dumps, statistical whole-game runner, Reference build upkeep | 8             |
| Lua API design onto the Deep Model                                                                                | 6             |
| Lua API implementation (replaces 15.9k lines of bindings)                                                         | 4             |
| Lua Content migration (about 9k lines)                                                                            | 3             |
| JSON Content schema migration tooling (D7)                                                                        | 3             |
| Headless runner and debug tools                                                                                   | 3             |
| Cross-context integration and whole-game equivalence, 10% of contexts                                             | 57            |
| **Total**                                                                                                         | **≈ 657**     |

That works out to about 600 BN lines per person-month overall. **Range:** −30% if the rates are generous; +50% for overrun, since fish-shell ran 2.5x over its schedule on a port a tenth this size ([prior art](https://github.com/adamico/Cataclysm-RD/blob/research/incremental-cpp-to-rust-ports/docs/research/incremental-cpp-to-rust-ports.md)). Hence 450–1,000.

## AI-assisted figure

### Who does what

The human side is **one person**: there are no other contributors (D1), so calendar time equals person-months. The rest is a swarm of agent sessions in a harness built from the Matt Pocock skills (wayfinder, tdd, code-review, research, the issue tracker) and [`cargo-modules`](https://github.com/regexident/cargo-modules).

- **The person decides.** Each design decision is a grilling ticket, at the speed of this map: each decision ticket took 10 minutes to 5 hours.
- **The person reads architecture and interfaces, never function bodies.** That means the deep-module layout and each context's public types and port traits, shown through `cargo-modules` views. `cargo-modules` also gates layering in CI: no dependency cycles, and no context reaching past another context's ports.
- **The person reads the oracle's evidence:** each context's Observables and replay unit, its coverage report (replayed, statistical-only and stubbed rules), its mutation gate result and soak results at milestones.
- **Agents do everything else:** translation, recording cases, replay tests, the refactor pass, code review against the context's ADR, and debugging. They escalate to the person only when stuck, or when the cause is a decision ("is this BN behaviour an Approximation or a rule?").
- **The oracle is out of the translating agents' reach** ([ADR 0004](adr/0004-replay-recorded-draws.md)). Cases come only from the Reference build. Replay tests drive each context's public interface and are written by a separate agent, and the mutation gate runs in CI. This is what makes "nobody reads the Rust" safe, and without it the pessimistic bound applies.

### Which kinds of work AI speeds up

| Kind of work                                                                   | What the spike showed                                                                          | Who does it                                                                                 |
| ------------------------------------------------------------------------------ | ---------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------- |
| Translation                                                                    | About 14,000 BN lines an hour of agent time, one bug in 4,300 Rust lines, and replay caught it | Agents. The person steers through the harness.                                              |
| Oracle, per context                                                            | 44 layouts and 48 replay tests in 27 min                                                       | Agents. The person reads the coverage and mutation reports.                                 |
| Oracle, fixed part                                                             | 31 min of work plus 13 min of build waits                                                      | Agents, paid once. Routing draws by replay unit adds a recorder change (below).             |
| Untangling and design                                                          | Crates and ports drafted in 4 min. The real decisions were made in HITL grilling               | The agent drafts and **the person decides at 1x**. This is the largest share of human time. |
| Refactor pass                                                                  | Not measured. Review asked for one                                                             | Agents, behind interface-level replay tests that survive it. No human review.               |
| Code review                                                                    | 23 min for about 4,300 Rust lines                                                              | A review agent, against the context's ADR. The person reads only interfaces.                |
| Debugging                                                                      | One bug, found by replay                                                                       | Agents. The person comes in on escalation.                                                  |
| Statistical-only code (restructured draws: plan/apply, overmap, `weather_gen`) | Not measured                                                                                   | No exact oracle, so it costs more, through the tier and integration.                        |

Interface-level replay needs a change to the spike's harness. 29 of its 36 layouts already drive Fields through one whole pass and compare Observables. The 7 creature cases call `creature_in_field` directly and move to Fields' Creature-facing port. `ReplayDice` hands out draws from one stream in BN's order, so the recorder must tag each draw with its unit key and the dice source look it up by key. That is agent work, in the fixed oracle line.

### Human hours

Weighted by tier, the Engine is about 589 "easy-equivalent" k lines.

| Line                                                       | Rate                                           | Hours                                                |
| ---------------------------------------------------------- | ---------------------------------------------- | ---------------------------------------------------- |
| Oracle reports (coverage, mutation, soak)                  | 0.1 h per easy-equivalent k line               | 59                                                   |
| Steering the swarm                                         | 0.15 h                                         | 88                                                   |
| Debugging on escalation                                    | 0.05 h                                         | 29                                                   |
| Per-context checks: Observables, replay unit, gate results | 2 h easy, 4 h medium, 8 h hard                 | 68                                                   |
| Reading and approving interfaces                           | 2 h easy, a day medium, 3 days hard            | 138                                                  |
| Design decisions, plus escalated decisions                 | 2 h easy, a day medium, a week hard, plus 30 h | 229                                                  |
| Deep Model skeleton and Lua API design                     | four weeks each                                | 240                                                  |
| Fixed oracle, Content migration, headless runner           | —                                              | 150                                                  |
| Building the harness                                       | from existing bricks                           | 10                                                   |
| Running the harness: drift, merge conflicts, model changes | 10%                                            | 101                                                  |
| Integration and whole-game runs                            | 20%                                            | 223                                                  |
| **Total**                                                  |                                                | **≈ 1,335 h ≈ 9.5 person-months** (at 140 h a month) |

- **Agent time:** the spike's agent rate (about 23 min per easy-equivalent k line for reading, translating, recording and testing) gives about 230 agent-hours. With the refactor pass, the unreplayed gaps and retries, budget **about 360 agent-hours or more**. That doesn't set the calendar, because sessions run in parallel. But **token cost was not measured**, and Rubin's 475k-line port was held back by token limits. Measuring it is a job for the first porting phase.
- **Range 7–21:** −30% at the low end. The top uses design at the old rates (a day for easy, a week for medium, three weeks for hard: ≈ 1,970 h, 14 person-months), plus fish-shell's overrun at 1.5x.
- **Cross-check:** Rubin's C-to-Rust port, about 475k lines with coding agents and the closest case to our size, is still unfinished after about 7 months with a small team. That fits a figure in the tens of person-months, not in the hundreds.

### The pessimistic bound

The published per-kind factors (translation 6x, tests and oracle 3x, refactor 3x, debugging 1.5x, design and review 1x, statistical-only translation 2x), applied to the human figure, give **≈ 340 person-months (1.9x)**. This is the figure to use if, in hard contexts, agents can't carry the work behind the oracle and a human has to write and read the code, as in the published reports.

## What the spike doesn't cover

These are the reasons the range is wide. The first porting phase should measure them:

1. **A hard context.** Fields is mostly self-contained, never bypasses the dice helpers and has one Tangle. The tier multipliers (1.5x, 2.4x) are judgement, not measurement.
2. **The refactor pass.** Review asked for one, and it isn't timed.
3. **Unreplayed gaps.** `player_in_field`, the largest function, and the bubble-map rules were not replayed, so the bug rate (one in 4,300) only holds where the cases reach.
4. **Replay's blind spot.** A threshold that is slightly off on the result side of a draw (25 changed to 24) slipped through 8 seeds. Soak runs catch it, at a cost in recording time.
5. **Cross-context integration.** The spike stubbed every neighbour of Fields (seven ports).
6. **Token cost and agent drift** at the size of the whole Engine.
7. **Inherited BN quirks:** small-seed `minstd_rand0` streams, UB (the gas remover crashes the Reference build), libstdc++ heap tie order, and clang's argument evaluation order.

## Not in this estimate

- The Frontend phase (D4). Nothing is playable until it is done.
- Profiling and choosing the Benchmark save. This is the first step after a "go".
- The optimization work that the port exists for (D16, D17).
- Writing the port's code conventions. Only the refactor pass they imply is budgeted.
