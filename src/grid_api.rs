use std::mem;
use std::ops::Range;
use std::sync::Mutex;
use std::sync::{Arc, RwLock};
use std::thread;

use bevy::math::ivec2;
use bevy::prelude::*;

use rand::Rng;

use crate::atom::State;
use crate::chunk::*;
use crate::consts::*;

// Parallel reference for image and chunk data
pub type UpdateChunksType = Vec<Option<(Arc<RwLock<Chunk>>, Arc<Mutex<Image>>)>>;
pub type UpdateChunkType = (Arc<RwLock<Chunk>>, Arc<Mutex<Image>>);

/// Swap two atoms from the same chunk
fn swap_same_chunk(chunk: UpdateChunkType, pos1: IVec2, pos2: IVec2, dt: f32) {
    {
        let chunk = &mut chunk.0.write().unwrap();
        chunk.atoms.swap(pos1.d1(), pos2.d1());
        chunk.atoms[pos1.d1()].updated_at = dt;
        chunk.atoms[pos2.d1()].updated_at = dt;
    }
    let positions = vec![pos1, pos2];
    chunk
        .0
        .read()
        .unwrap()
        .update_image_positions(&mut chunk.1.lock().unwrap(), &positions);
}

/// Swap two atoms from different chunks
fn swap_diff_chunk(
    chunk1: UpdateChunkType,
    chunk2: UpdateChunkType,
    pos1: IVec2,
    pos2: IVec2,
    dt: f32,
) {
    {
        let mut chunk1 = chunk1.0.write().unwrap();
        let mut chunk2 = chunk2.0.write().unwrap();

        let atom1 = &mut chunk1.atoms[pos1.d1()];
        let atom2 = &mut chunk2.atoms[pos2.d1()];
        atom1.updated_at = dt;
        atom2.updated_at = dt;
        mem::swap(atom1, atom2);
    }

    let handle1 = thread::spawn(move || {
        chunk1
            .0
            .read()
            .unwrap()
            .update_image_positions(&mut chunk1.1.lock().unwrap(), &vec![pos1]);
    });
    let handle2 = thread::spawn(move || {
        chunk2
            .0
            .read()
            .unwrap()
            .update_image_positions(&mut chunk2.1.lock().unwrap(), &vec![pos2]);
    });

    for handle in [handle1, handle2] {
        handle.join().unwrap()
    }
}

/// Swap two atoms from global 3x3 chunks positions
pub fn swap(chunks: &UpdateChunksType, pos1: IVec2, pos2: IVec2, dt: f32) {
    let local1 = global_to_local(pos1);
    let local2 = global_to_local(pos2);

    if local1.1 != local2.1 {
        //Diff chunk
        swap_diff_chunk(
            chunks[local1.1 as usize].clone().unwrap(),
            chunks[local2.1 as usize].clone().unwrap(),
            local1.0,
            local2.0,
            dt,
        )
    } else {
        //Same chunk
        swap_same_chunk(
            chunks[local1.1 as usize].clone().unwrap(),
            local1.0,
            local2.0,
            dt,
        )
    }
}

/// Transforms global chunk position to local 3x3 chunks position
pub fn global_to_local(pos: IVec2) -> (IVec2, i32) {
    let range = 0..CHUNK_SIZE as i32 * 3;
    if !range.contains(&pos.x) || !range.contains(&pos.y) {
        panic!("Invalid position on global_to_local.")
    }

    let chunk_size = CHUNK_SIZE as i32;

    let chunk_x = pos.x % (chunk_size * 3) / chunk_size;
    let chunk_y = pos.y / chunk_size;

    let local_x = pos.x - chunk_x * chunk_size;
    let local_y = pos.y - chunk_y * chunk_size;

    let chunk_index = chunk_y * 3 + chunk_x;

    (IVec2::new(local_x, local_y), chunk_index)
}

/// Transforms local chunk position to global 3x3 chunks position
pub fn local_to_global(pos: (IVec2, i32)) -> IVec2 {
    let range = 0..CHUNK_SIZE as i32;
    if !range.contains(&pos.0.x) || !range.contains(&pos.0.y) || !(0..9).contains(&pos.1) {
        panic!("Invalid position on local_to_global.")
    }

    let chunk_size = CHUNK_SIZE as i32;

    let chunk_index = pos.1;

    let chunk_x = chunk_index % 3;
    let chunk_y = chunk_index / 3;

    let global_x = pos.0.x + chunk_size * chunk_x;
    let global_y = pos.0.y + chunk_size * chunk_y;

    IVec2::new(global_x, global_y)
}

/// Gets atom state from a global pos
pub fn _get_state(chunks: &UpdateChunksType, pos: IVec2) -> Option<State> {
    let local = global_to_local(pos);

    if let Some(chunk) = &chunks[local.1 as usize] {
        return Some(chunk.0.read().unwrap().atoms[local.0.d1()].state);
    } else {
        None
    }
}

/// See if position is swapable, that means it sees if the position is a void
/// or if it's a swapable state and has been not updated
pub fn swapable(chunks: &UpdateChunksType, pos: IVec2, states: &[(State, f32)], dt: f32) -> bool {
    let local = global_to_local(pos);

    if let Some(chunk) = &chunks[local.1 as usize] {
        let atom = chunk.0.read().unwrap().atoms[local.0.d1()];

        atom.state == State::Void
            || (states.iter().any(|&(state, prob)| {
                state == atom.state && rand::thread_rng().gen_range(0.0..1.0) < prob
            }) && atom.updated_at != dt)
    } else {
        false
    }
}

/// Gets down neighbours from a global pos
pub fn down_neigh(
    chunks: &UpdateChunksType,
    pos: IVec2,
    states: &[(State, f32)],
    dt: f32,
) -> [(bool, IVec2); 3] {
    let mut neigh = [(false, IVec2::ZERO); 3];

    for (neigh, x) in neigh.iter_mut().zip([0, -1, 1]) {
        neigh.0 = swapable(chunks, pos + IVec2::new(x, 1), states, dt);
        neigh.1 = IVec2::new(x, 1);
    }

    if rand::thread_rng().gen() {
        neigh.swap(1, 2)
    }

    neigh
}

/// Gets side neighbours from a global pos
pub fn side_neigh(
    chunks: &UpdateChunksType,
    pos: IVec2,
    states: &[(State, f32)],
    dt: f32,
) -> [(bool, IVec2); 2] {
    let mut neigh = [(false, IVec2::ZERO); 2];

    for (neigh, x) in neigh.iter_mut().zip([-1, 1]) {
        neigh.0 = swapable(chunks, pos + IVec2::new(x, 0), states, dt);
        neigh.1 = IVec2::new(x, 0);
    }

    if rand::thread_rng().gen() {
        neigh.swap(0, 1)
    }

    neigh
}

/// Gets velocity from a global pos
pub fn get_vel(chunks: &UpdateChunksType, pos: IVec2) -> Option<IVec2> {
    let local = global_to_local(pos);

    let mut value = None;

    if let Some(chunk) = chunks[local.1 as usize].clone() {
        value = Some(chunk.0.read().unwrap().atoms[local.0.d1()].velocity);
    }

    value.unwrap_or(None)
}

/// Sets velocity from a global pos
pub fn set_vel(chunks: &UpdateChunksType, pos: IVec2, velocity: IVec2) {
    let local = global_to_local(pos);

    if let Some(chunk) = chunks[local.1 as usize].clone() {
        chunk.0.write().unwrap().atoms[local.0.d1()].velocity = if velocity == IVec2::ZERO {
            None
        } else {
            Some(velocity)
        }
    }
}

/// Adds velocity from a global pos
pub fn _add_vel(chunks: &UpdateChunksType, pos: IVec2, velocity: IVec2) {
    let local = global_to_local(pos);

    if let Some(chunk) = chunks[local.1 as usize].clone() {
        let atom_vel = &mut chunk.0.write().unwrap().atoms[local.0.d1()].velocity;
        if let Some(atom_vel) = atom_vel {
            *atom_vel += velocity;
        } else {
            *atom_vel = Some(velocity)
        }

        if *atom_vel == Some(IVec2::ZERO) {
            *atom_vel = None
        }
    }
}

/// Gets fall speed from a global pos
pub fn get_fspeed(chunks: &UpdateChunksType, pos: IVec2) -> u8 {
    let local = global_to_local(pos);

    let mut value = None;

    if let Some(chunk) = chunks[local.1 as usize].clone() {
        value = Some(chunk.0.read().unwrap().atoms[local.0.d1()].fall_speed);
    }

    value.unwrap_or(0)
}

/// Sets fall speed from a global pos
pub fn set_fspeed(chunks: &UpdateChunksType, pos: IVec2, fall_speed: u8) {
    let local = global_to_local(pos);

    if let Some(chunk) = chunks[local.1 as usize].clone() {
        chunk.0.write().unwrap().atoms[local.0.d1()].fall_speed = fall_speed
    }
}

/// Sets atom dt from a global pos
pub fn _set_dt(chunks: &UpdateChunksType, pos: IVec2, dt: f32) {
    let local = global_to_local(pos);

    if let Some(chunk) = chunks[local.1 as usize].clone() {
        chunk.0.write().unwrap().atoms[local.0.d1()].updated_at = dt
    }
}

/// Checks if atom is able to update this frame from a global pos
pub fn dt_updatable(chunks: &UpdateChunksType, pos: IVec2, dt: f32) -> bool {
    let local = global_to_local(pos);

    if let Some(chunk) = &chunks[local.1 as usize] {
        let atom = chunk.0.read().unwrap().atoms[local.0.d1()];
        atom.updated_at != dt || atom.state == State::Void
    } else {
        false
    }
}

pub fn extend_rect_if_needed(rect: &mut Rect, pos: &Vec2) {
    if pos.x < rect.min.x {
        rect.min.x = pos.x
    } else if pos.x > rect.max.x {
        rect.max.x = pos.x
    }

    if pos.y < rect.min.y {
        rect.min.y = pos.y
    } else if pos.y > rect.max.y {
        rect.max.y = pos.y
    }
}

// Shuflles range
pub fn rand_range(vec: Range<usize>) -> Vec<usize> {
    let mut vec: Vec<usize> = vec.collect();
    fastrand::shuffle(&mut vec);
    vec
}

// Transform pos to chunk coords
pub fn transform_to_local(pos: Vec2, width_height: (usize, usize)) -> Option<(IVec2, i32)> {
    if pos.x < 0. || pos.y < 0. {
        return None;
    }

    let (chunk_x, chunk_y) = (
        (pos.x / (CHUNK_SIZE * ATOM_SIZE) as f32) as usize,
        (pos.y / (CHUNK_SIZE * ATOM_SIZE) as f32) as usize,
    );

    if chunk_x >= width_height.0 || chunk_y >= width_height.1 {
        return None;
    }

    let (atom_x, atom_y) = (
        ((pos.x / ATOM_SIZE as f32) % CHUNK_SIZE as f32) as i32,
        ((pos.y / ATOM_SIZE as f32) % CHUNK_SIZE as f32) as i32,
    );

    let local = (
        ivec2(atom_x, atom_y),
        (chunk_y * width_height.0 + chunk_x) as i32,
    );

    println!("{:?}", local);

    Some(local)
}

pub trait D1 {
    fn d1(&self) -> usize;
}

impl D1 for IVec2 {
    /// Transforms a IVec2 to a index for a chunk atoms vec
    fn d1(&self) -> usize {
        (self.y * CHUNK_SIZE as i32 + self.x) as usize
    }
}

impl D1 for UVec2 {
    /// Transforms a UVec2 to a index for a chunk atoms vec
    fn d1(&self) -> usize {
        (self.y * CHUNK_SIZE as u32 + self.x) as usize
    }
}
