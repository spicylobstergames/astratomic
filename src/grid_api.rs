use std::collections::{HashMap, HashSet};
use std::ops::Range;
use std::panic;
use std::sync::Arc;
use std::sync::Mutex;

use bevy::math::ivec2;
use bevy::prelude::*;

use rand::Rng;

use crate::atom::{Atom, State};
use crate::chunk::*;
use crate::consts::*;

// Parallel reference for image and chunk data
pub type TexturesHash = HashMap<usize, HashSet<IVec2>>;
pub type ParTexturesHash = Arc<Mutex<TexturesHash>>;
pub type UpdateChunksType<'a> = (ChunkGroup<'a>, &'a ParTexturesHash);

pub type ChunkCorners<'a> = [&'a mut [Atom; CHUNK_SIZE * CHUNK_SIZE / 4]; 4];
pub type ChunkSides<'a> = [&'a mut [Atom; CHUNK_SIZE * CHUNK_SIZE / 2]; 4];

pub struct ChunkGroup<'a> {
    pub center: &'a mut Chunk,
    pub corners: ChunkCorners<'a>,
    pub sides: ChunkSides<'a>,
}

impl<'a> ChunkGroup<'a> {
    pub fn new(chunk: &'a mut Chunk, corners: ChunkCorners<'a>, sides: ChunkSides<'a>) -> Self {
        Self {
            center: chunk,
            corners,
            sides,
        }
    }

    #[inline]
    pub fn is_center(&self, idx: usize) -> bool {
        idx == 4
    }

    pub fn get_local(&self, idx: (IVec2, i32)) -> Option<&Atom> {
        match idx.1 {
            4 => Some(&self.center.atoms[idx.0.d1()]),
            0 | 2 | 6 | 8 => todo!(),
            1 | 3 | 5 | 7 => todo!(),
            _ => None,
        }
    }

    pub fn get_mut_local(&mut self, idx: (IVec2, i32)) -> Option<&mut Atom> {
        match idx.1 {
            4 => Some(&mut self.center.atoms[idx.0.d1()]),
            0 | 2 | 6 | 8 => todo!(),
            1 | 3 | 5 | 7 => todo!(),
            _ => None,
        }
    }

    pub fn get_global(&self, idx: IVec2) -> Option<&Atom> {
        let local_idx = global_to_local(idx);
        self.get_local(local_idx)
    }

    pub fn get_mut_global(&mut self, idx: IVec2) -> Option<&mut Atom> {
        let local_idx = global_to_local(idx);
        self.get_mut_local(local_idx)
    }

    //TODO Add way to access chunk group dirty rects
}

// Two ways to index a ChunkGroup to get an atom, by a global(IVec2) and by a local pos (IVec2, i32)
// Use get_* if you want an Option instead
impl std::ops::Index<IVec2> for ChunkGroup<'_> {
    type Output = Atom;
    #[track_caller]
    fn index(&self, idx: IVec2) -> &Self::Output {
        self.get_global(idx).expect("Invalid index position.")
    }
}
impl std::ops::IndexMut<IVec2> for ChunkGroup<'_> {
    #[track_caller]
    fn index_mut(&mut self, idx: IVec2) -> &mut Self::Output {
        self.get_mut_global(idx).expect("Invalid index position.")
    }
}

impl std::ops::Index<(IVec2, i32)> for ChunkGroup<'_> {
    type Output = Atom;
    #[track_caller]
    fn index(&self, idx: (IVec2, i32)) -> &Self::Output {
        self.get_local(idx).expect("Invalid index position.")
    }
}
impl std::ops::IndexMut<(IVec2, i32)> for ChunkGroup<'_> {
    #[track_caller]
    fn index_mut(&mut self, idx: (IVec2, i32)) -> &mut Self::Output {
        self.get_mut_local(idx).expect("Invalid index position.")
    }
}

/// Swap two atoms from the same chunk
fn swap_same_chunk(chunk: &mut Chunk, pos1: IVec2, pos2: IVec2, dt: f32, hash: &ParTexturesHash) {
    {
        chunk.atoms.swap(pos1.d1(), pos2.d1());
        chunk.atoms[pos1.d1()].updated_at = dt;
        chunk.atoms[pos2.d1()].updated_at = dt;
    }

    let mut hash = hash.lock().unwrap();
    hash.entry(chunk.index).or_default().extend([pos1, pos2]);
}

/// Swap two atoms from different chunks
fn swap_diff_chunk(
    chunk1: &mut Chunk,
    chunk2: &Chunk,
    pos1: (IVec2, i32),
    pos2: (IVec2, i32),
    dt: f32,
    hash: &ParTexturesHash,
) {
    {
        let atom1 = &mut chunk1.atoms[pos1.0.d1()];
        let atom2 = chunk2.atoms[pos2.0.d1()];
        atom1.updated_at = dt;
        *atom1 = atom2;
    }

    let mut hash = hash.lock().unwrap();
    hash.entry(chunk1.index).or_default().insert(pos1.0);
    hash.entry(chunk2.index).or_default().insert(pos2.0);
}

/// Swap two atoms from global 3x3 chunks positions
pub fn swap(chunks: &mut UpdateChunksType, pos1: IVec2, pos2: IVec2, dt: f32) {
    let local1 = global_to_local(pos1);
    let local2 = global_to_local(pos2);

    if local1.1 == local2.1 {
        //Same chunk
        swap_same_chunk(chunks.0.center, local1.0, local2.0, dt, chunks.1)
    } else {
        todo!()
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

/// See if position is swapable, that means it sees if the position is a void
/// or if it's a swapable state and has been not updated
pub fn swapable(chunks: &UpdateChunksType, pos: IVec2, states: &[(State, f32)], dt: f32) -> bool {
    if let Some(atom) = chunks.0.get_global(pos) {
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
    chunks.0[pos].velocity
}

/// Sets velocity from a global pos
pub fn set_vel(chunks: &mut UpdateChunksType, pos: IVec2, velocity: IVec2) {
    chunks.0[pos].velocity = if velocity == IVec2::ZERO {
        None
    } else {
        Some(velocity)
    }
}

/// Gets fall speed from a global pos
pub fn get_fspeed(chunks: &UpdateChunksType, pos: IVec2) -> u8 {
    chunks.0[pos].fall_speed
}

/// Sets fall speed from a global pos
pub fn set_fspeed(chunks: &mut UpdateChunksType, pos: IVec2, fall_speed: u8) {
    chunks.0[pos].fall_speed = fall_speed
}

/// Checks if atom is able to update this frame from a global pos
pub fn dt_updatable(chunks: &UpdateChunksType, pos: IVec2, dt: f32) -> bool {
    if let Some(atom) = chunks.0.get_global(pos) {
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
