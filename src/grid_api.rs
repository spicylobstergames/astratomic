use std::collections::{HashMap, HashSet};
use std::ops::Range;
use std::sync::Arc;
use std::sync::Mutex;

use async_channel::Sender;
use atomicell::AtomicCell;
use bevy::math::ivec2;
use bevy::prelude::*;

use rand::Rng;

use crate::atom::{Atom, State};
use crate::chunk::*;
use crate::consts::*;

// Parallel reference for image and chunk data
pub type TexturesHash = HashMap<usize, HashSet<IVec2>>;
pub type ParTexturesHash = Arc<Mutex<TexturesHash>>;
pub type AtomicChunk = AtomicCell<Chunk>;
pub type UpdateChunksType<'a> = (Vec<Option<&'a AtomicChunk>>, ParTexturesHash);

/// Swap two atoms from the same chunk
fn swap_same_chunk(chunk: &AtomicChunk, pos1: IVec2, pos2: IVec2, dt: f32, hash: &ParTexturesHash) {
    {
        let mut chunk = chunk.borrow_mut();
        chunk.atoms.swap(pos1.d1(), pos2.d1());
        chunk.atoms[pos1.d1()].updated_at = dt;
        chunk.atoms[pos2.d1()].updated_at = dt;
    }

    let index = chunk.borrow().index;
    let mut hash = hash.lock().unwrap();
    if let Some(set) = hash.get_mut(&index) {
        set.extend([pos1, pos2].iter());
    } else {
        hash.insert(index, HashSet::from([pos1, pos2]));
    }
}

/// A deferred chunk update message. Indicates that an atom in a chunk should be
/// set to a specific value.
#[derive(Debug)]
pub enum DeferredChunkUpdate {
    SetAtom {
        chunk_idx: usize,
        atom_idx: usize,
        atom: Atom,
    },
    UpdateDirtyRect {
        chunk_idx: usize,
        pos: Vec2,
    },
}

/// Swap two atoms from different chunks
fn swap_diff_chunk(
    chunk1: &AtomicChunk,
    chunk2: &AtomicChunk,
    deferred_updates: &Sender<DeferredChunkUpdate>,
    pos1: (IVec2, i32),
    pos2: (IVec2, i32),
    dt: f32,
    hash: &ParTexturesHash,
) {
    let mut chunk1 = chunk1.borrow_mut();
    let chunk2 = chunk2.borrow();

    {
        let atom1 = &mut chunk1.atoms[pos1.0.d1()];
        let atom2 = chunk2.atoms[pos2.0.d1()];
        deferred_updates
            .try_send(DeferredChunkUpdate::SetAtom {
                chunk_idx: chunk2.index,
                atom_idx: pos2.0.d1(),
                atom: *atom1,
            })
            .unwrap();
        atom1.updated_at = dt;
        *atom1 = atom2;
    }

    let index1 = chunk1.index;
    let index2 = chunk2.index;

    let mut hash = hash.lock().unwrap();
    hash.entry(index1).or_default().insert(pos1.0);
    hash.entry(index2).or_default().insert(pos2.0);
}

/// Swap two atoms from global 3x3 chunks positions
pub fn swap(
    chunks: &UpdateChunksType,
    deferred_updates: &Sender<DeferredChunkUpdate>,
    pos1: IVec2,
    pos2: IVec2,
    dt: f32,
) {
    let local1 = global_to_local(pos1);
    let local2 = global_to_local(pos2);

    if local1.1 != local2.1 {
        // Diff chunk
        swap_diff_chunk(
            chunks.0[local1.1 as usize].unwrap(),
            chunks.0[local2.1 as usize].unwrap(),
            deferred_updates,
            local1,
            local2,
            dt,
            &chunks.1,
        )
    } else {
        //Same chunk
        swap_same_chunk(
            chunks.0[local1.1 as usize].unwrap(),
            local1.0,
            local2.0,
            dt,
            &chunks.1,
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

    if let Some(chunk) = &chunks.0[local.1 as usize] {
        return Some(chunk.borrow().atoms[local.0.d1()].state);
    } else {
        None
    }
}

/// See if position is swapable, that means it sees if the position is a void
/// or if it's a swapable state and has been not updated
pub fn swapable(chunks: &UpdateChunksType, pos: IVec2, states: &[(State, f32)], dt: f32) -> bool {
    let local = global_to_local(pos);

    if let Some(chunk) = &chunks.0[local.1 as usize] {
        let atom = chunk.borrow().atoms[local.0.d1()];

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

    if let Some(chunk) = chunks.0[local.1 as usize] {
        value = Some(chunk.borrow().atoms[local.0.d1()].velocity);
    }

    value.unwrap_or(None)
}

/// Sets velocity from a global pos
pub fn set_vel(chunks: &UpdateChunksType, pos: IVec2, velocity: IVec2) {
    let local = global_to_local(pos);

    if let Some(chunk) = chunks.0[local.1 as usize] {
        chunk.borrow_mut().atoms[local.0.d1()].velocity = if velocity == IVec2::ZERO {
            None
        } else {
            Some(velocity)
        }
    }
}

/// Adds velocity from a global pos
pub fn _add_vel(chunks: &UpdateChunksType, pos: IVec2, velocity: IVec2) {
    let local = global_to_local(pos);

    if let Some(chunk) = chunks.0[local.1 as usize] {
        let atom_vel = &mut chunk.borrow_mut().atoms[local.0.d1()].velocity;
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

    if let Some(chunk) = chunks.0[local.1 as usize] {
        value = Some(chunk.borrow().atoms[local.0.d1()].fall_speed);
    }

    value.unwrap_or(0)
}

/// Sets fall speed from a global pos
pub fn set_fspeed(chunks: &UpdateChunksType, pos: IVec2, fall_speed: u8) {
    let local = global_to_local(pos);

    if let Some(chunk) = chunks.0[local.1 as usize] {
        chunk.borrow_mut().atoms[local.0.d1()].fall_speed = fall_speed
    }
}

/// Sets atom dt from a global pos
pub fn _set_dt(chunks: &UpdateChunksType, pos: IVec2, dt: f32) {
    let local = global_to_local(pos);

    if let Some(chunk) = chunks.0[local.1 as usize] {
        chunk.borrow_mut().atoms[local.0.d1()].updated_at = dt
    }
}

/// Checks if atom is able to update this frame from a global pos
pub fn dt_updatable(chunks: &UpdateChunksType, pos: IVec2, dt: f32) -> bool {
    let local = global_to_local(pos);

    if let Some(chunk) = &chunks.0[local.1 as usize] {
        let atom = chunk.borrow().atoms[local.0.d1()];
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
pub fn transform_to_chunk(pos: Vec2, width_height: (usize, usize)) -> Option<(IVec2, i32)> {
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
