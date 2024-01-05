use std::ops::Range;

use itertools::Itertools;
use rand::Rng;

use async_channel::Sender;

use crate::prelude::*;

pub struct UpdateChunksType<'a> {
    pub group: ChunkGroup<'a>,
    pub dirty_update_rect_send: &'a Sender<DeferredDirtyRectUpdate>,
    pub dirty_render_rect_send: &'a Sender<DeferredDirtyRectUpdate>,
}

/// Swap two atoms from global 3x3 chunks positions
pub fn swap(chunks: &mut UpdateChunksType, pos1: IVec2, pos2: IVec2, dt: u8) {
    let chunk_group = &mut chunks.group;
    {
        let temp = chunk_group[pos1];
        chunk_group[pos1] = chunk_group[pos2];
        chunk_group[pos2] = temp;

        chunk_group[pos1].updated_at = dt;
        chunk_group[pos2].updated_at = dt;
    }

    let local1 = global_to_local(pos1);
    let local2 = global_to_local(pos2);

    for (pos, idx) in [local1, local2] {
        let chunk = ChunkGroup::group_to_chunk(chunk_group.center_pos, idx);

        chunks
            .dirty_render_rect_send
            .try_send(DeferredDirtyRectUpdate {
                chunk_pos: ChunkPos::new(pos.try_into().unwrap(), chunk),
                ..Default::default()
            })
            .unwrap();
    }
}

/// Transforms a global manager pos to a chunk pos
pub fn global_to_chunk(mut pos: IVec2) -> ChunkPos {
    // This makes sure we don't have double 0 chunks.
    if pos.x < 0 {
        pos.x -= CHUNK_LENGHT as i32;
    }
    if pos.y < 0 {
        pos.y -= CHUNK_LENGHT as i32;
    }

    let (mut chunk_x, mut chunk_y) = (pos.x / CHUNK_LENGHT as i32, pos.y / CHUNK_LENGHT as i32);

    //Hacky stuff for neg coords, but it works
    let (x_off, y_off) = (
        (pos.x % CHUNK_LENGHT as i32 + CHUNK_LENGHT as i32) % CHUNK_LENGHT as i32,
        (pos.y % CHUNK_LENGHT as i32 + CHUNK_LENGHT as i32) % CHUNK_LENGHT as i32,
    );

    if pos.x < 0 && x_off == 0 {
        chunk_x += 1
    }
    if pos.y < 0 && y_off == 0 {
        chunk_y += 1
    }

    let (x, y) = (
        if pos.x >= 0 {
            pos.x as u32 % CHUNK_LENGHT as u32
        } else {
            x_off as u32
        },
        if pos.y >= 0 {
            pos.y as u32 % CHUNK_LENGHT as u32
        } else {
            y_off as u32
        },
    );

    ChunkPos::new(uvec2(x, y), ivec2(chunk_x, chunk_y))
}

/// Transforms a chunk pos to a global manager pos
pub fn chunk_to_global(pos: ChunkPos) -> IVec2 {
    let mut atom = pos.atom.as_ivec2();
    atom.x += pos.chunk.x * CHUNK_LENGHT as i32;
    atom.y += pos.chunk.y * CHUNK_LENGHT as i32;

    atom
}

/// See if position is swapable, that means it sees if the position is a void
/// or if it's a swapable state and has been not updated
pub fn swapable(
    chunks: &UpdateChunksType,
    pos: IVec2,
    states: &[(State, f32)],
    state: State,
    dt: u8,
) -> bool {
    if let Some(atom) = chunks.group.get_global(pos) {
        atom.state == State::Void
            || (states.iter().any(|&(state, prob)| {
                state == atom.state && rand::thread_rng().gen_range(0.0..1.0) < prob
            }) && atom.updated_at != dt)
            || (atom.state == State::Object && state == State::Liquid)
    } else {
        false
    }
}

/// Gets down neighbours from a global pos
pub fn down_neigh(
    chunks: &UpdateChunksType,
    pos: IVec2,
    states: &[(State, f32)],
    dt: u8,
) -> [(bool, IVec2); 3] {
    let mut neigh = [(false, IVec2::ZERO); 3];

    let state = get_state(chunks, pos);
    for (neigh, x) in neigh.iter_mut().zip([0, -1, 1]) {
        neigh.0 = swapable(chunks, pos + IVec2::new(x, 1), states, state, dt);
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
    dt: u8,
) -> [(bool, IVec2); 2] {
    let mut neigh = [(false, IVec2::ZERO); 2];

    let state = get_state(chunks, pos);
    for (neigh, x) in neigh.iter_mut().zip([-1, 1]) {
        neigh.0 = swapable(chunks, pos + IVec2::new(x, 0), states, state, dt);
        neigh.1 = IVec2::new(x, 0);
    }

    if rand::thread_rng().gen() {
        neigh.swap(0, 1)
    }

    neigh
}

/// Gets velocity from a global pos
pub fn get_vel(chunks: &UpdateChunksType, pos: IVec2) -> Option<IVec2> {
    let vel = chunks.group[pos].velocity;

    if vel == (0, 0) {
        None
    } else {
        Some(ivec2(vel.0 as i32, vel.1 as i32))
    }
}

/// Sets velocity from a global pos
pub fn set_vel(chunks: &mut UpdateChunksType, pos: IVec2, velocity: IVec2) {
    chunks.group[pos].velocity = if velocity == IVec2::ZERO {
        (0, 0)
    } else {
        (velocity.x as i8, velocity.y as i8)
    }
}

/// Sets mode from a global pos
pub fn set_mode(chunks: &mut UpdateChunksType, pos: IVec2, mode: bool) {
    chunks.group[pos].automata_mode = mode
}

/// Gets fall speed from a global pos
pub fn get_fspeed(chunks: &UpdateChunksType, pos: IVec2) -> u8 {
    chunks.group[pos].velocity.1.try_into().unwrap()
}

/// Gets state from a global pos
pub fn get_state(chunks: &UpdateChunksType, pos: IVec2) -> State {
    chunks.group[pos].state
}

/// Sets fall speed from a global pos
pub fn set_fspeed(chunks: &mut UpdateChunksType, pos: IVec2, fall_speed: u8) {
    chunks.group[pos].velocity.1 = fall_speed as i8
}

/// Checks if atom is able to update this frame from a global pos
pub fn dt_updatable(chunks: &UpdateChunksType, pos: IVec2, dt: u8) -> bool {
    if let Some(atom) = chunks.group.get_global(pos) {
        atom.updated_at != dt || atom.state == State::Void
    } else {
        false
    }
}

pub fn extend_rect_if_needed(rect: &mut URect, pos: &UVec2) {
    if pos.x < rect.min.x {
        rect.min.x = (pos.x).clamp(0, 63)
    } else if pos.x > rect.max.x {
        rect.max.x = (pos.x).clamp(0, 63)
    }

    if pos.y < rect.min.y {
        rect.min.y = (pos.y).clamp(0, 63)
    } else if pos.y > rect.max.y {
        rect.max.y = (pos.y).clamp(0, 63)
    }
}

// Shuflles range
pub fn rand_range(vec: Range<i32>) -> Vec<i32> {
    let mut vec: Vec<i32> = vec.collect();
    fastrand::shuffle(&mut vec);
    vec
}

pub trait D1 {
    fn d1(&self) -> usize;
}

impl D1 for IVec2 {
    /// Transforms a IVec2 to a index for a chunk atoms vec
    fn d1(&self) -> usize {
        (self.y * CHUNK_LENGHT as i32 + self.x) as usize
    }
}

impl D1 for UVec2 {
    /// Transforms a UVec2 to a index for a chunk atoms vec
    fn d1(&self) -> usize {
        (self.y * CHUNK_LENGHT as u32 + self.x) as usize
    }
}

pub fn update_dirty_rects(dirty_rects: &mut HashMap<IVec2, URect>, pos: ChunkPos) {
    if let Some(dirty_rects) = dirty_rects.get_mut(&pos.chunk) {
        extend_rect_if_needed(dirty_rects, &pos.atom)
    } else {
        dirty_rects.insert(
            pos.chunk,
            URect::new(pos.atom.x, pos.atom.y, pos.atom.x, pos.atom.y),
        );
    }
}

//This function gets a single ChunkPos and makes sure that we update the 3x3 surrounding atoms
pub fn update_dirty_rects_3x3(dirty_rects: &mut HashMap<IVec2, URect>, pos: ChunkPos) {
    let atom = pos.atom;
    let mut chunk = pos.chunk;

    if (1..62).contains(&atom.x) && (1..62).contains(&atom.y) {
        // Case where the 3x3 position area is within a chunk
        if let Some(rect) = dirty_rects.get_mut(&chunk) {
            extend_rect_if_needed(rect, &(atom + UVec2::ONE));
            extend_rect_if_needed(rect, &(atom - UVec2::ONE));
        } else {
            dirty_rects.insert(
                chunk,
                URect::new(atom.x - 1, atom.y - 1, atom.x + 1, atom.y + 1),
            );
        }
    } else if (atom.x == 0 || atom.x == 63) && (1..62).contains(&atom.y) {
        // Case where the 3x3 position area is in another chunk into the left or right
        if let Some(rect) = dirty_rects.get_mut(&chunk) {
            extend_rect_if_needed(
                rect,
                &(atom - if atom.x == 0 { UVec2::Y } else { UVec2::ONE }),
            );
            extend_rect_if_needed(
                rect,
                &(atom + if atom.x == 0 { UVec2::ONE } else { UVec2::Y }),
            );
        } else {
            dirty_rects.insert(
                chunk,
                URect::new(
                    atom.x - if atom.x == 0 { 0 } else { 1 },
                    atom.y - 1,
                    atom.x + if atom.x == 0 { 1 } else { 0 },
                    atom.y + 1,
                ),
            );
        }

        let x = if atom.x == 0 { 63 } else { 0 };
        if atom.x == 0 {
            chunk.x -= 1
        } else {
            chunk.x += 1
        }
        if let Some(rect) = dirty_rects.get_mut(&chunk) {
            extend_rect_if_needed(rect, &(uvec2(x, atom.y + 1)));
            extend_rect_if_needed(rect, &(uvec2(x, atom.y - 1)));
        } else {
            dirty_rects.insert(chunk, URect::new(x, atom.y - 1, x, atom.y + 1));
        }
    } else if (atom.y == 0 || atom.y == 63) && (1..62).contains(&atom.x) {
        // Case where the 3x3 position area is in another chunk into the up or down
        if let Some(rect) = dirty_rects.get_mut(&chunk) {
            extend_rect_if_needed(
                rect,
                &(atom - if atom.y == 0 { UVec2::X } else { UVec2::ONE }),
            );
            extend_rect_if_needed(
                rect,
                &(atom + if atom.y == 0 { UVec2::ONE } else { UVec2::X }),
            );
        } else {
            dirty_rects.insert(
                chunk,
                URect::new(
                    atom.x - 1,
                    atom.y - if atom.y == 0 { 0 } else { 1 },
                    atom.x + 1,
                    atom.y + if atom.y == 0 { 1 } else { 0 },
                ),
            );
        }

        let y = if atom.y == 0 { 63 } else { 0 };
        if atom.y == 0 {
            chunk.y -= 1
        } else {
            chunk.y += 1
        }
        if let Some(rect) = dirty_rects.get_mut(&chunk) {
            extend_rect_if_needed(rect, &(uvec2(atom.x + 1, y)));
            extend_rect_if_needed(rect, &(uvec2(atom.x - 1, y)));
        } else {
            dirty_rects.insert(chunk, URect::new(atom.x - 1, y, atom.x + 1, y));
        }
    } else {
        // Case where the 3x3 position is in the corner of a chunk
        for (x, y) in (-1..=1).cartesian_product(-1..=1) {
            let mut global = chunk_to_global(pos);
            global += ivec2(x, y);
            let pos = global_to_chunk(global);

            if let Some(rect) = dirty_rects.get_mut(&pos.chunk) {
                extend_rect_if_needed(rect, &pos.atom)
            } else {
                dirty_rects.insert(
                    pos.chunk,
                    URect::new(pos.atom.x, pos.atom.y, pos.atom.x, pos.atom.y),
                );
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct ChunkPos {
    pub atom: UVec2,
    pub chunk: IVec2,
}

impl ChunkPos {
    pub fn new(atom: UVec2, chunk: IVec2) -> Self {
        Self { atom, chunk }
    }
}

/// A deferred update message.
/// Indicates that an image or dirty rect should update.
#[derive(Debug, Default)]
pub struct DeferredDirtyRectUpdate {
    pub chunk_pos: ChunkPos,
    pub awake_surrouding: bool,
}
