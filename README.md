# Astratomic

[![Discord](https://img.shields.io/discord/865004050357682246?logo=discord&logoColor=white)](https://discord.gg/JFhxYBvxR8) 

[![Screenshot from 2024-01-13 09-08-15](https://github.com/spicylobstergames/astratomic/assets/78173025/175de10d-fc7b-4fa3-813e-04705c480662)](https://youtu.be/bChefWqOmh0)

Astratomic is a survival game that draws inspiration from the likes of Noita and Starbound.
Powered by a similar engine as Noita, made from scratch with Rust.

## Controls
- You can walk with WASD and jump with Space. When on air press space again to activate your jetpack!
- Left Mouse button to pull atoms, Right Mouse button to push atoms.
- Mouse wheel to zoom in/out.

## Where to play it

- Go to our latest release page https://github.com/spicylobstergames/astratomic/releases/tag/v0.2.0
- Download the zip for your platform, unzip it and run the executable, it's that simple!

## Current State and Goals

**We currently implemented:**
  - Chunk system
  - Smart updating of only what's needed
  - Multithreaded sand simulation
  - GPU rendering optimizations of the sim
  - AABB colliders for the player and mobs
  - Player with special gun to push and pull atoms, and also a jetpack
  - Bigger map, with saving/loading
  - More types of atoms! Like lava, acid, seeds, ...

**What's next:**
  - Rapier2d rigidbodies (with simulated pixels) for things you can throw!
  - Procedurally generated worlds
  - Procedural Animations
  
In the bigger picture we also aim to achieve intergalactic travel like Starbound, procedurally generated animations inspired by Rainworld, all the cool mechanics you see in Noita, multiplayer and modding, the last two made possible by a future engine migration to [Bones](https://github.com/fishfolk/bones/).

## Last Update Changelog

We added:
- Actors
  - AABB colliders that interact with the atom world.
- Player
  - The player is pretty cool! It has some animations, a jetpack and a pretty nice tool!
 - Player tool
   - The tool can pull atoms(Left Mouse Button), and also push(Right Mouse Button) the atoms you pulled!
- Engine Changes
  - We changed the Chunk Manager approach to store Chunks in a Vec to storing them in a HashMap, this lays the ground for future updates, like Save/Load worlds, and big explorable worlds. It was needed to simplify addind the other additions and unfortunately took some time to do.

## Licensing

In the future we aim to separate the engine code from the game code, making the former licensed under Apache v2.0 and the latter under PolyForm NonCommercial v1.0.

### Contributor License Compromise

Independent contributions (i.e. individual pull requests) from anyone other than 'Isaac Turci @Zac8668' are dual-licensed as [Polyform NonCommercial](https://polyformproject.org/licenses/noncommercial/1.0.0/) (granted to Spicy Lobster & Isaac Turci as the _licensor_) and [Apache 2.0](https://www.apache.org/licenses/LICENSE-2.0), meaning all independent contributors retain ownership of their own contributions, albeit non-exclusively.

More specifics soon to come in [CONTRIBUTING.md](/CONTRIBUTING.md).

### Assets

The player assets are from Penzilla, https://penzilla.itch.io/protagonist-character.
