# Astratomic

[![Discord](https://img.shields.io/discord/865004050357682246?logo=discord&logoColor=white)](https://discord.gg/JFhxYBvxR8) 

[astrademo.webm](https://github.com/Zac8668/astratomic/assets/78173025/f079acdc-9e1f-4636-b39b-6b358de71f11)


Astratomic is a survival game that draws inspiration from the likes of Noita and Starbound.
Powered by a similar engine as Noita, made from scratch with Rust.

## Current State and Goals

**We currently implemented:**
  - Chunk system
  - Smart updating of only what's needed
  - Multithreaded sand simulation
  - GPU rendering optimizations of the sim
  - AABB colliders for the player and mobs
  - Player with special gun to shoot and suck atoms, and also a jetpack

**What's next:**
  - More types of atoms! Like lava, acid, seeds, ...
  - Rapier2d rigidbodies (with simulated pixels) for things you can throw!
  - Bigger map, with saving/loading
  - Procedurally generated worlds
  
In the bigger picture we also aim to achieve intergalactic travel like Starbound, procedurally generated animations inspired by Rainworld, all the cool mechanics you see in Noita, multiplayer and modding, the last two made possible by a future engine migration to [Bones](https://github.com/fishfolk/bones/).

## Controls
Left Mouse button to suck atoms, Right Mouse button to shoot atoms.

## Last Update Changelog

We added:
- Actors
  - AABB colliders that interact with the atom world.
- Player
  - The player is pretty cool! It has some animations, a jetpack and a pretty nice tool!
 - Player tool
   - The tool can suck atoms(Left Mouse Button), and also shoot(Right Mouse Button) the atoms you sucked!
- Engine Changes
  - We changed the Chunk Manager approach to store Chunks in a Vec to storing them in a HashMap, this lays the ground for future updates, like Save/Load worlds, and big explorable worlds. It was needed to simplify addind the other additions and unfortunately took some time to do.

## Licensing

In the future we aim to separate the engine code from the game code, making the former licensed under MIT and the latter under PolyForm NonCommercial v1.0.

#

The player assets are from Penzilla, https://penzilla.itch.io/protagonist-character.
