# Rewrite the Engine, with Bright Nights as the reference

Rusted Dawn's Engine is rewritten in Rust, not ported incrementally behind an FFI seam. The rewrite is a translation, context by context: the Bright Nights v0.13.0 C++ is the specification of behaviour, rule by rule, and only the structure is new, shaped by the Deep Model (D8, D9). The C++ Baseline is never a host for Rust code. It runs only as the **Reference build**.

BN v0.13.0 has no seam that is coarse, owns its data and carries little traffic, apart from Time & Weather and Fields (and Fields only once it is cut out of `map`). The Simulation's hot loops (monster planning against `map` queries, per-tile cache builds, vehicle movement) all run through `map`. So the first useful step of an incremental port would already be rewriting the Reality bubble and everything it owns in one chunk, while paying for a `cxx` seam that handles the Engine's hierarchies, `std::map` fields, `location_ptr` templates and shared mutable state worst. Staying playable, the main gain of an incremental port, isn't needed by a fork with no players waiting (D1) and a headless metric (Turn throughput).

Decided in [Incremental FFI port or rewrite?](https://github.com/adamico/Cataclysm-RD/issues/10). The evidence is in [engine-calibration.md](../research/engine-calibration.md), the [BN Engine analysis](https://github.com/adamico/Cataclysm-RD/blob/research/bn-engine-analysis/docs/research/bn-engine-analysis.md) (FFI cut-cost notes) and the [prior-art research](https://github.com/adamico/Cataclysm-RD/blob/research/incremental-cpp-to-rust-ports/docs/research/incremental-cpp-to-rust-ports.md).

## Considered options

- **Incremental `cxx` port, fish-style.** It stays shippable and gives a running differential oracle, but fish kept the C++ structure to make that work, which D8 and ADR 0002 reject. Every seam would carry opaque handles and throwaway shims, hot loops couldn't cross the seam without cross-language LTO and a clang-only toolchain, and two build systems would last for the life of the port.
- **Greenfield redesign** from the Game glossary and Content, using the C++ only to settle disputes. It's the Netscape failure mode, and it would void the AI speedup factors, which hold for translation, not design.
- **The Rust Engine driven by BN's C++ Frontend over a bridge.** BN's Frontend reads `game`, `map` and `Character` directly, so the bridge would sit on the same tangle.

## Consequences

- **The Rust Engine links no C++.** There is no shared build. D18's note that an FFI port could build the C++ alongside from a submodule no longer applies. The Baseline stays a pinned, read-only reference.
- **The Engine runs headless until the Frontend phase:** it loads Content and a save, runs the Turn loop, and is measured on the Benchmark save, with only the debug tools the port needs.
- **Rules carry over unless flagged.** Every Baseline rule is kept unless it is a flagged Approximation (D6).
- **The one spike (D2) is a from-scratch slice of Fields,** checked against the Reference build and timed as the AI pilot.
- **A test-only bridge stays possible.** Whether one is worth having, calling individual C++ rules next to their Rust versions, is the Gameplay-equivalence oracle's decision. It never ships.
