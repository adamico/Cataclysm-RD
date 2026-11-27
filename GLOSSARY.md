# Rust Port

The language of Cataclysm: Rusted Dawn, the effort to port the Cataclysm engine to Rust, and, along the way, to make it simulate dense areas (labs above all) faster without changing how the game plays.

## Language

### Measuring

**Turn throughput**:
The number of game turns simulated per wall-clock second while game time passes without player input (waiting, sleeping, long activities). The one metric this effort optimizes.
_Avoid_: Speed, FPS, latency, processing time

**Benchmark save**:
A fixed saved game, with a fixed seed and a scripted time-passing action, against which every **Turn throughput** measurement is taken.
_Avoid_: Test save, fixture

### What is being optimized

**Simulation**:
Everything the game computes to advance the world by one turn: creatures, fields, items, vehicles and the world-state caches they depend on. Rendering, UI and save/load are not **Simulation**.
_Avoid_: Backend, game logic, processing

**Dense area**:
A part of the map that makes **Simulation** costly because it is crowded with any of creatures, fields, items/furniture, or frequently changing light and visibility. Which kind dominates is decided by profiling, not assumed.
_Avoid_: Busy area, heavy area

### The codebase

**Engine**:
The part of the game that holds and advances the world: **Simulation**, loading game data, and save/load. The first target of the port.
_Avoid_: Backend, core

**Frontend**:
The part of the game the player sees and touches: rendering (terminal and tiles), menus and input. Ported last.
_Avoid_: UI layer, client

**Content**:
The game's data and mods, written as JSON. Migrated rather than frozen: the game's content carries over to the port, but its schema may change where the **Deep Model** calls for it.
_Avoid_: Assets, data files

**Baseline**:
The single upstream release the port starts from. Upstream changes enter only through a deliberate re-baseline, never by routine syncing.
_Avoid_: Upstream, master

**Reference build**:
The **Baseline** built unmodified, as a separate program, with its parallelism and its **Approximations** turned off. It is what **Gameplay equivalence** is checked against, and it is never a host for ported code.
_Avoid_: Oracle (that is the check, not the program), old build, C++ side

### Judging the codebase

**Deep Model**:
In Evans' sense: a model of the game's domain that captures what its concepts really are, beyond surface nouns, and that is expressed as a ubiquitous language and bounded contexts. It decides *where* the **Engine**'s seams belong. It is distilled from **Content** and code, not from separate prose docs.
_Avoid_: Domain model (too loose), deep module

**Deep module**:
After Ousterhout, with depth measured as leverage (behaviour reachable per unit of interface), not as a lines ratio: a module whose small interface hides a large implementation. It judges *how good* each seam placed by the **Deep Model** is.
_Avoid_: Deep Model, component

**Machinery**:
The parts of the **Engine** with no game language of their own, and so outside the **Deep Model**: loading **Content**, save/load, options, geometry, units, randomness, threading.
_Avoid_: Infrastructure, utilities, core

**Tangle**:
A place where the **Baseline**'s code disagrees with the **Deep Model**, such as one context's state or rules living in another context's code. Recorded as a porting cost, never as a reason to redraw a context.
_Avoid_: Leak, coupling, tech debt

### Guarantees

**Port fidelity**:
The requirement that a ported part of the **Engine** meets **Gameplay equivalence** with the C++ it replaces. Save and mod compatibility are not part of it.
_Avoid_: Parity, compatibility

**Gameplay equivalence**:
An optimization's promise that no player-visible behaviour changes, even if the order of random draws shifts. The default bar for every change.
_Avoid_: Identical behaviour, determinism

**Approximation**:
A change that deliberately alters **Simulation** results for speed, such as thinking less often for unseen creatures. Always flagged explicitly and kept behind an option; never passed off as **Gameplay equivalence**.
_Avoid_: Shortcut, LOD
