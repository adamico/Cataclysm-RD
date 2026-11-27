# Bounded contexts are drawn by language, for the target model

The Engine's Deep Model has sixteen bounded contexts (see the [Game glossary](../game/GLOSSARY.md)). They are drawn by language: each context has its own concepts, rules and Content families. They describe the model Rusted Dawn aims for, not how Bright Nights v0.13.0 groups its files, and not what is cheap to port as one chunk.

Where the Baseline's code disagrees with the model, the disagreement is recorded as a **Tangle**, a porting cost, and the context stays where it is. A context is not a port unit. One context may be ported in several pieces, or two in one, and port order is decided separately.

Decided in [Which bounded contexts make up the Engine?](https://github.com/adamico/Cataclysm-RD/issues/8). The evidence is in the two Engine analyses and [engine-calibration.md](../research/engine-calibration.md).

## Considered options

- **Follow the code as it is.** Fields would fold into the map, because the map stores fields and processes them. Vehicle movement would stay with the map too. That would make the seams match today's C++ and hide the cost of untangling it.
- **Draw contexts as port units, by size and cut cost.** That mixes two questions. A context's language outlives the port, and port chunks don't.

## Consequences

- **Size doesn't make a context.** Fields (about 4k lines) and Records stand on their own, and Characters stays large.
- **Known Tangles in BN v0.13.0,** to cost in the estimate and to weigh in the FFI-or-rewrite decision:
  - field storage lives in the map's submaps;
  - vehicle movement is written as map functions;
  - hordes are stored on the overmap;
  - other contexts' rules live inside the examine actions;
  - Weather's rules leak into Items and Creatures;
  - the `game` god object holds state from every context and is dissolved across all of them.
- **Machinery is not a context.** Content loading, save/load, options, geometry, units, RNG and the thread pool have no game language, so they stay outside the Deep Model.
- **Harm is one language.** Combat owns Harm from any source, including Explosions, traps and Fields, even though most of those are not Attacks.
