use core::f32::consts::PI;

use async_channel::Sender;
use bevy::prelude::*;

use crate::actors::*;
use crate::consts::*;
use crate::geom_tools::*;
use crate::grid_api::*;

use rand::Rng;

// Make smaller
#[derive(Clone, Copy, Default, PartialEq, Debug)]
pub struct Atom {
    pub color: [u8; 4],
    pub state: State,
    pub updated_at: f32,
    pub fall_speed: u8,
    // Used when thrown up, etc
    pub velocity: Option<IVec2>,
    // Frames idle
    pub f_idle: u8,
}

impl Atom {
    pub fn new() -> Self {
        Atom {
            color: [55, 55, 55, 255],
            state: State::Void,
            updated_at: 0.,
            fall_speed: 0,
            velocity: None,
            f_idle: 0,
        }
    }
}

#[derive(Default, Clone, Copy, PartialEq, Debug)]
pub enum State {
    Solid,
    Powder,
    Liquid,
    Gas,
    #[default]
    Void,
}

// Update different types of atoms

/// Updates powder and returns atoms awakened
pub fn update_powder(
    chunks: &UpdateChunksType,
    deferred_updates: &Sender<DeferredChunkUpdate>,
    pos: IVec2,
    dt: f32,
    _actors: &[(Actor, Transform)],
) -> Vec<IVec2> {
    let mut awakened = vec![];

    let mut cur_pos = pos;

    // Get fall speed
    let mut fall_speed = get_fspeed(chunks, cur_pos);
    if fall_speed < TERM_VEL {
        fall_speed += GRAVITY;
        set_fspeed(chunks, cur_pos, fall_speed);
    }

    for _ in 0..fall_speed {
        let neigh = down_neigh(chunks, cur_pos, &[(State::Liquid, 0.2)], dt);
        let mut swapped = false;
        for neigh in neigh {
            if neigh.0 {
                swap(chunks, deferred_updates, cur_pos, cur_pos + neigh.1, dt);
                awakened.push(cur_pos);
                cur_pos += neigh.1;
                awakened.push(cur_pos);
                swapped = true;

                break;
            }
        }

        if !swapped {
            let new_vel = Vec2::new(0.0, -(fall_speed as f32));

            set_vel(
                chunks,
                cur_pos,
                Vec2::from_angle(rand::thread_rng().gen_range(-PI / 2.0..PI / 2.))
                    .rotate(new_vel * 0.3)
                    .as_ivec2(),
            );

            set_fspeed(chunks, cur_pos, 0);

            break;
        }
    }

    awakened
}

/// Updates liquid and returns atoms awakened
pub fn update_liquid(
    chunks: &UpdateChunksType,
    deferred_updates: &Sender<DeferredChunkUpdate>,
    pos: IVec2,
    dt: f32,
    _actors: &[(Actor, Transform)],
) -> Vec<IVec2> {
    let mut awakened = vec![];
    let mut cur_pos = pos;

    // Get fall speed
    let mut fall_speed = get_fspeed(chunks, pos);
    if fall_speed < TERM_VEL {
        fall_speed += GRAVITY;
        set_fspeed(chunks, pos, fall_speed);
    }

    let mut swapped = false;
    for _ in 0..fall_speed {
        let neigh = down_neigh(chunks, cur_pos, &[], dt);
        for neigh in neigh {
            if neigh.0 {
                swap(chunks, deferred_updates, cur_pos, cur_pos + neigh.1, dt);
                awakened.push(cur_pos);
                cur_pos += neigh.1;
                awakened.push(cur_pos);
                swapped = true;

                break;
            }
        }
    }

    if !swapped {
        set_fspeed(chunks, cur_pos, 0);

        let neigh = side_neigh(chunks, cur_pos, &[], dt);
        let side = if neigh[0].0 {
            Some(neigh[0].1.x)
        } else if neigh[1].0 {
            Some(neigh[1].1.x)
        } else {
            None
        };

        if let Some(side) = side {
            for _ in 0..5 {
                if !swapable(chunks, cur_pos + IVec2::new(side, 0), &[], dt) {
                    break;
                }

                swap(
                    chunks,
                    deferred_updates,
                    cur_pos,
                    cur_pos + IVec2::new(side, 0),
                    dt,
                );
                awakened.push(cur_pos);
                cur_pos += IVec2::new(side, 0);
                awakened.push(cur_pos);
            }
        }
    }

    awakened
}

/// Updates particle and returns atoms awakened
pub fn update_particle(
    chunks: &UpdateChunksType,
    deferred_updates: &Sender<DeferredChunkUpdate>,
    pos: IVec2,
    dt: f32,
    _actors: &[(Actor, Transform)],
) -> Vec<IVec2> {
    let mut awakened = vec![];
    let mut cur_pos = pos;

    // Add gravity
    let mut vel = get_vel(chunks, cur_pos).unwrap_or(IVec2::ZERO);
    if vel.y < TERM_VEL as i32 {
        vel += GRAVITY as i32 * IVec2::Y;
        set_vel(chunks, cur_pos, vel);
    }

    // Move
    for pos in Line::new(cur_pos, vel) {
        awakened.push(cur_pos);
        if swapable(chunks, pos, &[], dt) {
            swap(chunks, deferred_updates, cur_pos, pos, dt);
            cur_pos = pos;
            awakened.push(cur_pos);
        } else {
            if vel.abs().x > 4 && vel.abs().y > 4 {
                set_vel(
                    chunks,
                    cur_pos,
                    (Vec2::from_angle(PI).rotate(vel.as_vec2()) * 0.5).as_ivec2(),
                );
            } else {
                set_vel(chunks, cur_pos, IVec2::ZERO);
            }
            break;
        }
    }

    awakened
}
