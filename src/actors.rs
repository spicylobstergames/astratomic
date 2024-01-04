use crate::prelude::*;

#[derive(Component, Clone, Copy)]
pub struct Actor {
    pub width: u8,
    pub height: u8,
    pub pos: IVec2,
    pub vel: Vec2,
}

//Called before simulations
pub fn add_actors(mut chunk_manager: ResMut<ChunkManager>, actors: Query<&Actor>) {
    for actor in actors.iter() {
        for x_off in 0..actor.width as i32 {
            for y_off in 0..actor.height as i32 {
                let pos = global_to_chunk(actor.pos + ivec2(x_off, y_off));
                if let Some(atom) = chunk_manager.get_mut_atom(pos) {
                    if atom.state == State::Void {
                        *atom = Atom::object();
                    }
                }
            }
        }
    }
}

//Called after simulation, before actor update
pub fn remove_actors(mut chunk_manager: ResMut<ChunkManager>, actors: Query<&Actor>) {
    for actor in actors.iter() {
        for x_off in 0..actor.width as i32 {
            for y_off in 0..actor.height as i32 {
                let pos = global_to_chunk(actor.pos + ivec2(x_off, y_off));
                if let Some(atom) = chunk_manager.get_mut_atom(pos) {
                    if atom.state == State::Object {
                        *atom = Atom::default();
                    }
                }
            }
        }
    }
}

pub fn on_ground(chunk_manager: &ChunkManager, actor: &Actor) -> bool {
    for x_off in 0..actor.width {
        let chunk_pos = global_to_chunk(actor.pos + ivec2(x_off as i32, actor.height as i32));

        if let Some(atom) = chunk_manager.get_atom(&chunk_pos) {
            if atom.state != State::Void {
                return true;
            }
        } else {
            return true;
        }
    }

    false
}

pub fn update_actors(mut chunk_manager: ResMut<ChunkManager>, mut actors: Query<&mut Actor>) {
    for mut actor in actors.iter_mut() {
        let mut prev = actor.pos;
        for v in Line::new(actor.pos, actor.vel.as_ivec2()) {
            let move_hor = match (prev.x != v.x, prev.y != v.y) {
                (true, false) => true,
                (false, true) => false,
                (true, true) => fastrand::bool(),
                _ => unreachable!(),
            };

            if move_hor {
                let moved_x = move_x(&mut chunk_manager, &mut actor, (v.x - prev.x).signum());
                if on_ground(&chunk_manager, &actor) {
                    let starting_y = actor.pos.y;
                    match moved_x {
                        //If we can't move to the left or right
                        //Check if we can get up a stair-like structure
                        false => {
                            for i in 1..=UP_WALK_HEIGHT {
                                let moved_y = move_y(&mut chunk_manager, &mut actor, -1);
                                //Abort if we couldn't move up, or if we moved up but couldn't move sideways on the last step
                                if !moved_y
                                    || i == UP_WALK_HEIGHT
                                        && !move_x(
                                            &mut chunk_manager,
                                            &mut actor,
                                            (v.x - prev.x).signum(),
                                        )
                                {
                                    abort_stair(&mut chunk_manager, &mut actor, starting_y, 1);
                                    break;
                                }
                            }
                        }
                        //If we can move to the left or right
                        //Check if we can snap back to the ground
                        true => {
                            for i in 1..=DOWN_WALK_HEIGHT {
                                if !move_y(&mut chunk_manager, &mut actor, 1) {
                                    break;
                                } else if i == DOWN_WALK_HEIGHT {
                                    abort_stair(&mut chunk_manager, &mut actor, starting_y, -1);
                                }
                            }
                        }
                    }
                }
            } else {
                move_y(&mut chunk_manager, &mut actor, (v.y - prev.y).signum());
            }

            prev = v;
        }
    }
}

pub fn abort_stair(chunk_manager: &mut ChunkManager, actor: &mut Actor, starting_y: i32, dir: i32) {
    for _ in 0..(starting_y - actor.pos.y) {
        move_y(chunk_manager, actor, dir);
    }
}

pub fn move_x(chunk_manager: &mut ChunkManager, actor: &mut Actor, dir: i32) -> bool {
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

        // Check bounds and if it's free to move
        if let Some(atom) = chunk_manager.get_mut_atom(chunk_pos) {
            if atom.state == State::Powder || atom.state == State::Solid {
                actor.vel = Vec2::ZERO;
                return false;
            } else if atom.state == State::Liquid {
                //Water splashes
                atom.velocity = (0, -20)
            }
        } else {
            actor.vel = Vec2::ZERO;
            return false;
        }
    }

    actor.pos.x += dir;

    true
}

pub fn move_y(chunk_manager: &mut ChunkManager, actor: &mut Actor, dir: i32) -> bool {
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

        // Check bounds and if it's free to move
        if let Some(atom) = chunk_manager.get_mut_atom(chunk_pos) {
            if atom.state == State::Powder || atom.state == State::Solid {
                actor.vel = Vec2::ZERO;
                return false;
            } else if atom.state == State::Liquid {
                //Water splashes
                atom.velocity = (0, -20)
            }
        } else {
            actor.vel = Vec2::ZERO;
            return false;
        }
    }

    actor.pos.y += dir;

    true
}

pub struct ActorsPlugin;
impl Plugin for ActorsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                add_actors
                    .before(chunk_manager_update)
                    .before(remove_actors)
                    .before(update_actors),
                remove_actors
                    .after(chunk_manager_update)
                    .after(add_actors)
                    .before(update_actors),
                update_actors
                    .after(remove_actors)
                    .after(add_actors)
                    .after(chunk_manager_update),
            ),
        );
    }
}
