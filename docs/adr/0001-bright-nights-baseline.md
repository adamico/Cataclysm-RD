# Bright Nights v0.13.0 is the Baseline

Rusted Dawn ports from Cataclysm: Bright Nights v0.13.0 (`e7b4976`), not CDDA 0.H-RELEASE.

After calibration, the two Engines score the same for Deep Model fit and Deep module quality, and they are the same size: 422k lines against 427k, or about 394k for BN without its GPU, Lua and grid extras. So porting cost was close to even, and the choice came down to what each Baseline brings.

BN brings a multithreaded Simulation design and a Lua scripting layer, and we want both. CDDA's larger Content (1.5M lines against 1.06M) did not weigh much. Its single-threaded Simulation would have been simpler to check Gameplay equivalence against, but that was not decisive, because BN can be run with parallelism off as the reference.

Decided in [Which Baseline: Bright Nights v0.13.0 or CDDA 0.H-RELEASE?](https://github.com/adamico/Cataclysm-RD/issues/9). The scores are in [engine-calibration.md](../research/engine-calibration.md).

## Consequences

- **The plan/apply split is a design principle, not code to port.** BN's monster planning splits into a read-only plan step on worker threads and an apply step on the main thread. The Rust Simulation adopts that split and extends it to other per-turn work. BN's thread pool itself is rewritten in Rust, not ported.
- **GPU lighting is not a porting obligation.** It is prior art for later optimization.
- **The Gameplay equivalence reference is BN with parallelism off:** `PARALLEL_MONSTER_PLANNING`, `PARALLEL_MAP_CACHE` and `PARALLEL_SCENT_UPDATE` all disabled. Parallel runs then only need to match the reference statistically.
- **Monster LOD is an Approximation (D6).** In the port it stays behind an option that is off by default, and Benchmark save measurements are taken with it off.
- **Lua stays the Content scripting language.** Under D7, its API may be reshaped to follow the Deep Model instead of mirroring BN's roughly 3.9k raw C++ bindings, and BN's roughly 9k lines of Lua Content migrate onto the new API.
