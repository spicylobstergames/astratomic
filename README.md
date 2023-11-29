# Astratomic

[![Discord](https://img.shields.io/discord/865004050357682246?logo=discord&logoColor=white)](https://discord.gg/JFhxYBvxR8) 

Astratomic is a survival game that draws inspiration from the likes of Noita and Starbound.
Powered by a similar engine as Noita, made from scratch with Rust.

## Current State and Goals

**We currently implemented:**
  - Chunk system
  - Smart updating of only what's needed
  - Multithreaded sand simulation
  - GPU rendering optimizations of the sim

**What's next:**
  - AABB colliders for the player and mobs
  - More types of atoms! Like lava, acid, seeds, ...
  - Rapier2d rigidbodies (with simulated pixels) for things you can throw!
  - Bigger map, with saving/loading
  - Procedurally generated worlds
  
In the bigger picture we also aim to achieve intergalactic travel like Starbound, procedurally generated animations inspired by Rainworld, all the cool mechanics you see in Noita, multiplayer and modding, the last two made possible by a future engine migration to [Bones](https://github.com/fishfolk/bones/).
