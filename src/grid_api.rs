use std::mem;
use std::sync::Mutex;
use std::sync::{Arc, RwLock};
use std::thread;

use bevy::prelude::*;

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
        chunk.active = true;
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
        chunk1.active = true;
        chunk2.active = true;

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

    for handle in vec![handle1, handle2] {
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
pub fn get_state(chunks: &UpdateChunksType, pos: IVec2) -> Option<State> {
    let local = global_to_local(pos);

    if let Some(chunk) = &chunks[local.1 as usize] {
        return Some(chunk.0.read().unwrap().atoms[local.0.d1()].state);
    } else {
        None
    }
}

/// See if position is swapable, that means it sees if the position is a void
/// or if it's a swapable state and has been not updated
pub fn swapable(chunks: &UpdateChunksType, pos: IVec2, states: Vec<State>, dt: f32) -> bool {
    let local = global_to_local(pos);

    if let Some(chunk) = &chunks[local.1 as usize] {
        let atom = chunk.0.read().unwrap().atoms[local.0.d1()];
        let state = atom.state;

        state == State::Void || (states.contains(&state) && atom.updated_at != dt)
    } else {
        false
    }
}

pub fn void(chunks: &UpdateChunksType, pos: IVec2) -> bool {
    let local = global_to_local(pos);

    if let Some(chunk) = &chunks[local.1 as usize] {
        chunk.0.read().unwrap().atoms[local.0.d1()].state == State::Void
    } else {
        false
    }
}

/// Gets atom density from a global pos
pub fn get_density(chunks: &UpdateChunksType, pos: IVec2) -> f32 {
    let local = global_to_local(pos);

    let mut value = None;

    if let Some(chunk) = chunks[local.1 as usize].clone() {
        value = Some(chunk.0.read().unwrap().atoms[local.0.d1()].density);
    }

    if let Some(value) = value {
        value
    } else {
        0.
    }
}

/// Gets atom sand simulation velocity from a global pos
pub fn get_svel(chunks: &UpdateChunksType, pos: IVec2) -> u8 {
    let local = global_to_local(pos);

    let mut value = None;

    if let Some(chunk) = chunks[local.1 as usize].clone() {
        value = Some(chunk.0.read().unwrap().atoms[local.0.d1()].sim_velocity);
    }

    if let Some(value) = value {
        value
    } else {
        0
    }
}

/// Sets atom sand simulation velocity from a global pos
pub fn set_svel(chunks: &UpdateChunksType, pos: IVec2, svel: u8) {
    let local = global_to_local(pos);

    if let Some(chunk) = chunks[local.1 as usize].clone() {
        chunk.0.write().unwrap().atoms[local.0.d1()].sim_velocity = svel
    }
}

/// Sets atom dt from a global pos
pub fn set_dt(chunks: &UpdateChunksType, pos: IVec2, dt: f32) {
    let local = global_to_local(pos);

    if let Some(chunk) = chunks[local.1 as usize].clone() {
        chunk.0.write().unwrap().atoms[local.0.d1()].density = dt
    }
}

/// Checks if atom is able to update this frame from a global pos
pub fn dt_upable(chunks: &UpdateChunksType, pos: IVec2, dt: f32) -> bool {
    let local = global_to_local(pos);

    if let Some(chunk) = &chunks[local.1 as usize] {
        let atom = chunk.0.read().unwrap().atoms[local.0.d1()];
        atom.updated_at != dt || atom.state == State::Void
    } else {
        false
    }
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
