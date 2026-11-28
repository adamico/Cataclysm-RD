# Game

The language of the game itself, as the Rusted Dawn Engine's **Deep Model** describes it: what the world is made of and how it changes, split into bounded contexts. It describes the model Rusted Dawn aims for, not how the Bright Nights code happens to be grouped. The project's own language (Engine, Baseline, Gameplay equivalence) is in the [Rusted Dawn glossary](../../GLOSSARY.md).

## Bounded contexts

### Creatures

**Monsters**:
The context of non-player creatures that act on instinct: monster types, monster behaviour and its turn-by-turn planning, monster special attacks, monster groups and hordes.
_Avoid_: Mobs, enemies, AI

**Characters**:
The context of people with bodies, whether the player or an NPC: anatomy, mutations, bionics, effects, skills, proficiencies, activities, and the scenario and profession a character starts with.
_Avoid_: Player (a character is not only the player), avatar

**NPC society**:
The context of what NPCs think and how they relate to the player and each other: NPC minds, dialogue, missions, factions and basecamps.
_Avoid_: NPC AI, dialogue system

**Combat**:
The context of attacks, and of **Harm** from any source: melee, ranged fire, martial arts, and how harm lands on a creature.
_Avoid_: Fighting, damage system

**Magic**:
The context of spells and enchantments, available to monsters, characters and items alike.
_Avoid_: Spells (only one part), mana system

### The world

**Reality bubble**:
The context of the loaded local world around the player, where Simulation happens: terrain, furniture, traps, examine actions, **Explosions** and utility grids.
_Avoid_: Local map, active area, map

**World generation**:
The context of creating the world beyond the Reality bubble: the overmap, cities, regions, and the local maps generated from them.
_Avoid_: Mapgen (only one part), worldgen, overmap

**Perception**:
The context of what a creature can sense: light, sight, noise and scent, and the player's memory of what they have seen.
_Avoid_: Vision, lighting, senses

**Fields**:
The context of spreading, decaying substances that occupy a tile, such as fire, smoke, gas, blood and acid.
_Avoid_: Effects, clouds, hazards

**Weather**:
The context of the outdoors over time: weather, temperature and wind.
_Avoid_: Climate, environment

### Things

**Items**:
The context of objects that can be carried, contained and used: items, item types, containers and item use.
_Avoid_: Inventory, objects, loot

**Crafting**:
The context of making things from requirements: recipes, uncrafting, tool qualities and construction.
_Avoid_: Recipes (only one part), building

**Vehicles**:
The context of vehicles and their parts, including how vehicles move and collide.
_Avoid_: Cars

### Running the game

**Turn loop**:
The context of advancing the world by one turn, and of deciding which creatures and processes act when.
_Avoid_: Game loop, main loop, tick

**Scripting**:
The context of the Lua hooks and bindings that Content scripts the game through.
_Avoid_: Lua API, modding API

**Records**:
The context of what is remembered about a playthrough: achievements, conducts, kills, the memorial log, the diary and statistics.
_Avoid_: Stats, scores, history

## Shared kernel

**Creature**:
Anything that occupies a tile, can act, can sense, and can be harmed. Every monster and every character is a Creature.
_Avoid_: Entity, actor, mob

**Time**:
The game's calendar, its turns and its durations, which every context measures in.
_Avoid_: Clock, calendar (only one part)

## Across contexts

**Harm**:
Damage landing on a creature, with its damage types, armour and resistance, and hit location, whatever caused it. Owned by **Combat**, and used by attacks, **Explosions**, traps and **Fields** alike.
_Avoid_: Damage (only one part), injury, attack

**Attack**:
Something a creature does on purpose to deal **Harm**. An attack can cause an **Explosion**, but is never one.
_Avoid_: Hit, strike

**Explosion**:
A physical event in the **Reality bubble**: a blast that spreads through terrain, damages it, and deals **Harm** and shrapnel to creatures in its path. It may leave **Fields** behind and be noticed through **Perception**. It is not an **Attack**, though an attack may cause one.
_Avoid_: Blast attack, detonation

**Horde**:
A group of monsters moving across the overmap, outside the Reality bubble. Owned by **Monsters**.
_Avoid_: Wandering spawn, mongroup
