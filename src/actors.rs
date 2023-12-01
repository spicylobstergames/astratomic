use crate::atom::State;
use crate::prelude::*;

#[derive(Component, Clone, Copy)]
pub struct Actor {
    pub width: u8,
    pub height: u8,
    pub pos: IVec2,
    pub vel: Vec2,
}

pub fn add_actor(chunk_manager: &mut ChunkManager, actor: &Actor) {
    let chunks = &mut chunk_manager.chunks;

    for x_off in 0..actor.width as i32 {
        for y_off in 0..actor.height as i32 {
            let pos = global_to_chunk(actor.pos + ivec2(x_off, y_off));
            chunks[pos.1 as usize].atoms[pos.0.d1()] = Atom {
                state: State::Actor,
                color: [55, 55, 55, 255],
                ..Default::default()
            };
        }
    }
}

pub fn on_ground(chunk_manager: &ChunkManager, actor: &Actor) -> bool {
    let chunks = &chunk_manager.chunks;
    for x_off in 0..actor.width {
        let chunk_pos = global_to_chunk(actor.pos + ivec2(x_off as i32, actor.height as i32));

        if let Some(chunk) = chunks.get(chunk_pos.1 as usize) {
            if let Some(atom) = chunk.atoms.get(chunk_pos.0.d1()) {
                if atom.state != State::Void {
                    return true;
                }
            } else {
                return true;
            }
        } else {
            return true;
        }
    }

    false
}

pub fn update_actors(mut chunk_manager: Query<&mut ChunkManager>, mut actors: Query<&mut Actor>) {
    let mut chunk_manager = chunk_manager.single_mut();

    for mut actor in actors.iter_mut() {
        let mut prev = actor.pos;
        for v in Line::new(actor.pos, actor.vel.as_ivec2()) {
            if prev.x != v.x {
                if !move_x(&mut chunk_manager, &mut actor, v.x - prev.x) {
                    break;
                }
            } else if !move_y(&mut chunk_manager, &mut actor, v.y - prev.y) {
                break;
            }
            prev = v;
        }
    }
}

pub fn move_x(chunk_manager: &mut ChunkManager, actor: &mut Actor, dir: i32) -> bool {
    let chunks = &mut chunk_manager.chunks;

    //Check if we can move
    for y_off in 0..actor.height as i32 {
        let pos = actor.pos
            + if dir > 0 {
                // Moving right
                ivec2(actor.width as i32, y_off)
            } else {
                // Moving left
                ivec2(-1, y_off)
            };
        let chunk_pos = global_to_chunk(pos);
        if chunk_pos.1 < 0 {
            return false;
        }

        // Check bounds and if it's free to swap
        if let Some(chunk) = chunks.get(chunk_pos.1 as usize) {
            if chunk.atoms[chunk_pos.0.d1()].state != State::Void {
                actor.vel = Vec2::ZERO;
                return false;
            }
        } else {
            actor.vel = Vec2::ZERO;
            return false;
        }
    }

    //Move
    for y_off in 0..actor.height as i32 {
        let pos1 = actor.pos
            + if dir > 0 {
                // Moving right
                ivec2(actor.width as i32, y_off)
            } else {
                // Moving left
                ivec2(-1, y_off)
            };

        let pos2 = actor.pos
            + if dir > 0 {
                // Moving right
                ivec2(0, y_off)
            } else {
                // Moving left
                ivec2(actor.width as i32 - 1, y_off)
            };

        let chunk_pos1 = global_to_chunk(pos1);
        let chunk_pos2 = global_to_chunk(pos2);
        let temp = chunks[chunk_pos1.1 as usize].atoms[chunk_pos1.0.d1()];
        chunks[chunk_pos1.1 as usize].atoms[chunk_pos1.0.d1()] =
            chunks[chunk_pos2.1 as usize].atoms[chunk_pos2.0.d1()];
        chunks[chunk_pos2.1 as usize].atoms[chunk_pos2.0.d1()] = temp;
    }
    actor.pos.x += dir;

    true
}

pub fn move_y(chunk_manager: &mut ChunkManager, actor: &mut Actor, dir: i32) -> bool {
    let chunks = &mut chunk_manager.chunks;

    //Check if we can move
    for x_off in 0..actor.width as i32 {
        let pos = actor.pos
            + if dir > 0 {
                // Moving down
                ivec2(x_off, actor.height as i32)
            } else {
                // Moving up
                ivec2(x_off, -1)
            };
        let chunk_pos = global_to_chunk(pos);
        if chunk_pos.1 < 0 {
            return false;
        }

        // Check bounds and if it's free to swap
        if let Some(chunk) = chunks.get(chunk_pos.1 as usize) {
            if chunk.atoms[chunk_pos.0.d1()].state != State::Void {
                actor.vel = Vec2::ZERO;
                return false;
            }
        } else {
            actor.vel = Vec2::ZERO;
            return false;
        }
    }

    //Move
    for x_off in 0..actor.width as i32 {
        let pos1 = actor.pos
            + if dir > 0 {
                // Moving down
                ivec2(x_off, actor.height as i32)
            } else {
                // Moving up
                ivec2(x_off, -1)
            };

        let pos2 = actor.pos
            + if dir > 0 {
                // Moving down
                ivec2(x_off, 0)
            } else {
                // Moving up
                ivec2(x_off, actor.height as i32 - 1)
            };

        let chunk_pos1 = global_to_chunk(pos1);
        let chunk_pos2 = global_to_chunk(pos2);
        let temp = chunks[chunk_pos1.1 as usize].atoms[chunk_pos1.0.d1()];
        chunks[chunk_pos1.1 as usize].atoms[chunk_pos1.0.d1()] =
            chunks[chunk_pos2.1 as usize].atoms[chunk_pos2.0.d1()];
        chunks[chunk_pos2.1 as usize].atoms[chunk_pos2.0.d1()] = temp;
    }

    actor.pos.y += dir;

    true
}

pub struct ActorsPlugin;
impl Plugin for ActorsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, update_actors.after(chunk_manager_update));
    }
}
