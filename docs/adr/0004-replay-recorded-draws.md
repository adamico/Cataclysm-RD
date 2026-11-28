# Check Gameplay equivalence by replaying recorded draws

Each translated rule is checked exactly against the Reference build, including rules that make random draws. A C++ harness records each rule's inputs, outputs and random draws, and the Rust rule is replayed against them. Statistical comparison is kept only for whole-game runs and for restructured code whose draws really do diverge.

Most Engine rules draw random numbers (Fields makes about 50 draws per pass), so comparing one C++ run with one Rust run means nothing. BN's dice helpers sit on `std::` distributions whose output depends on the C++ standard library, and D5 and D8 allow draw order to change. A purely statistical oracle would be slow and weak, and it would miss the most likely translation bug (`one_in(4)` for `one_in(5)`) unless it ran a very large number of games. Replaying draws at the helper level ("`one_in(5)` returned true") makes such a rule as checkable as one that draws nothing, and it catches that bug as a mismatched call.

Decided in [How do we check Gameplay equivalence against the C++?](https://github.com/adamico/Cataclysm-RD/issues/13).

## Considered options

- **Keep the draw order and compare byte for byte, Ladybird-style.** Rust would have to copy libstdc++'s distribution internals, and D8's restructuring would break it.
- **Statistical comparison only.** It is the natural reading of D5, but it is flaky unless tuned, slow, and blind to small biases.
- **A test-only `cxx` bridge** calling C++ rules next to their Rust versions. It gives unlimited live fuzzing, but brings a C++ toolchain and per-rule bindings back into the Rust build, which [ADR 0003](0003-rewrite-with-reference.md) had dropped.

## Consequences

- **Two tiers for each translated rule.** A rule that draws nothing must match the Reference build exactly on the same inputs. A rule that draws must match exactly when given the recorded draws. Replay tests run on every commit.
- **The Rust workspace never compiles C++.** The recorder is a separate C++ program built from the Baseline's untouched sources, and it counts as part of the Reference build. It links its own recording copy of `rng.cpp` in place of BN's. Code that bypasses the helpers (`std::shuffle` on the engine, `weather_gen.cpp`, `distribution.cpp`, `rand()` in overmap) is listed per context and handled case by case.
- **Each bounded context declares its Observables and its replay unit before it is ported.** The unit is the smallest piece with a fixed draw sequence: for Fields, one field entry on one tile in one turn, with its neighbours in the case. Order between units is free (D8).
- **Replay tests drive a context's public interface, never its internals.** A test feeds a recorded case through the context's interface (for Fields, one pass over the area) and compares the Observables. The replay unit is how draws are routed, not how big a test is: the recorder tags each draw with its unit key (for Fields, submap, tile, field type and turn), and the replay dice source serves each unit's draws from that key, whatever order the units run in. A test that calls a rule directly is allowed only while that rule is being debugged, and is deleted afterwards. That way the tests survive the refactor pass, and the person driving the port reads interfaces, not implementations.
- **The oracle is out of the translating agents' reach.** Cases come only from the Reference build, and translating agents can't change them except by re-recording. Replay tests are written from the cases by a separate agent, never by the one translating the rules. A mutation gate in CI (change a constant or comparison, and some replay test must fail) decides when a context is covered. It covers replay's blind spot on the result side of a draw.
- **No Rust rule reads a global RNG.** Each rule receives a dice source: a real RNG in play, the recorded draws in tests.
- **Whole-game runs stay statistical.** They load a save with an explicit `--seed` (BN does not save RNG state) in the plain-file world format, run N turns, and compare the two Engines' Observable dumps. Each run uses a fixed seed set and a written detection target, and runs nightly or at milestones. Parallel runs are checked against the single-threaded reference the same way.
- **BN's batch field decay is an Approximation.** `batch_turns.cpp` ages fields on submaps that come back into the Simulation with a linear formula instead of dice. The Fields port flags it and puts it behind an option (opt-in, per D6). The Reference build cannot turn it off, so it gets no equivalence check.
