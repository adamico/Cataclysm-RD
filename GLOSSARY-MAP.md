# Glossary Map

## Contexts

- [Rusted Dawn](./GLOSSARY.md): the language of the effort itself: what is optimized and measured, how the codebase is judged, and what Rusted Dawn guarantees.
- [Game](./docs/game/GLOSSARY.md): the language of the game: the Engine's Deep Model, split into sixteen bounded contexts plus a shared kernel.

## Relationships

- **Rusted Dawn → Game**: Rusted Dawn's **Deep Model** is the Game glossary. Its **Engine** is made of the Game's bounded contexts plus **Machinery**, which has no Game language.
- **Turn loop → Monsters, Characters, Fields, Vehicles, Weather**: the Turn loop decides which of them act each turn.
- **Monsters, Characters, Reality bubble, Fields → Combat**: every source of Harm (attacks, Explosions, traps, Fields) deals it through Combat.
- **Perception → Monsters, Characters, NPC society**: creatures act on what they can sense.
- **Reality bubble ↔ World generation**: World generation makes the local world the Reality bubble loads.
- **Records ← every context**: Records only listens to what happens in the game.
- **Scripting ↔ every context**: Content scripts the game through Scripting.
- **Game contexts → each other**: a context reaches a neighbour only through one of its own **Ports**, fulfilled by an **Adapter**.
- **Shared kernel**: **Creature** (Monsters and Characters) and **Time** (every context).
