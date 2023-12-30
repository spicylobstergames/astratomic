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

#

The player assets are from Penzilla, https://penzilla.itch.io/protagonist-character.
