use std::ops::Range;
use std::panic;

use bevy::math::{ivec2, vec2};
use rand::Rng;

use async_channel::Sender;

use crate::prelude::*;

pub struct UpdateChunksType<'a> {
    pub group: ChunkGroup<'a>,
    pub dirty_update_rect_send: &'a Sender<DeferredDirtyRectUpdate>,
    pub dirty_render_rect_send: &'a Sender<DeferredDirtyRectUpdate>,
}

/// Swap two atoms from global 3x3 chunks positions
pub fn swap(chunks: &mut UpdateChunksType, pos1: IVec2, pos2: IVec2, dt: f32) {
    let local1 = global_to_local(pos1);
    let local2 = global_to_local(pos2);

    let chunk_group = &mut chunks.group;
    {
        let temp = *chunk_group.get_local(local1).unwrap();
        chunk_group[local1] = chunk_group[local2];
        chunk_group[local2] = temp;

        chunk_group[local1].updated_at = dt;
        chunk_group[local2].updated_at = dt;
    }

    for (global_pos, (pos, idx)) in [(pos1, local1), (pos2, local2)] {
        let chunk_idx = ChunkGroup::group_to_manager_idx(chunk_group.center_index, idx);

        chunks
            .dirty_render_rect_send
            .send_blocking(DeferredDirtyRectUpdate {
                chunk_idx,
                pos: pos.as_vec2(),
                global_pos,
                center_idx: chunk_group.center_index,
            })
            .unwrap();
    }
}

/// Transforms global 3x3 chunk position to local 3x3 chunks position
pub fn global_to_local(pos: IVec2) -> (IVec2, i32) {
    let range = 0..CHUNK_LENGHT as i32 * 3;
    if !range.contains(&pos.x) || !range.contains(&pos.y) {
        panic!("Invalid position on global_to_local.")
    }

    let chunk_lenght = CHUNK_LENGHT as i32;

    let chunk_x = pos.x % (chunk_lenght * 3) / chunk_lenght;
    let chunk_y = pos.y / chunk_lenght;

    let local_x = pos.x - chunk_x * chunk_lenght;
    let local_y = pos.y - chunk_y * chunk_lenght;

    let chunk_index = chunk_y * 3 + chunk_x;

    (IVec2::new(local_x, local_y), chunk_index)
}

/// Transforms local 3x3 chunk position to global 3x3 chunks position
pub fn local_to_global(pos: (IVec2, i32)) -> IVec2 {
    let range = 0..CHUNK_LENGHT as i32;
    if !range.contains(&pos.0.x) || !range.contains(&pos.0.y) || !(0..9).contains(&pos.1) {
        panic!("Invalid position on local_to_global.")
    }

    let chunk_size = CHUNK_LENGHT as i32;

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
    if let Some(atom) = chunks.group.get_global(pos) {
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
    chunks.group[pos].velocity
}

/// Sets velocity from a global pos
pub fn set_vel(chunks: &mut UpdateChunksType, pos: IVec2, velocity: IVec2) {
    chunks.group[pos].velocity = if velocity == IVec2::ZERO {
        None
    } else {
        Some(velocity)
    }
}

/// Gets fall speed from a global pos
pub fn get_fspeed(chunks: &UpdateChunksType, pos: IVec2) -> u8 {
    chunks.group[pos].fall_speed
}

/// Sets fall speed from a global pos
pub fn set_fspeed(chunks: &mut UpdateChunksType, pos: IVec2, fall_speed: u8) {
    chunks.group[pos].fall_speed = fall_speed
}

/// Checks if atom is able to update this frame from a global pos
pub fn dt_updatable(chunks: &UpdateChunksType, pos: IVec2, dt: f32) -> bool {
    if let Some(atom) = chunks.group.get_global(pos) {
        atom.updated_at != dt || atom.state == State::Void
    } else {
        false
    }
}

pub fn extend_rect_if_needed(rect: &mut Rect, pos: &Vec2) {
    if pos.x < rect.min.x {
        rect.min.x = (pos.x).clamp(0., 63.)
    } else if pos.x > rect.max.x {
        rect.max.x = (pos.x).clamp(0., 63.)
    }

    if pos.y < rect.min.y {
        rect.min.y = (pos.y).clamp(0., 63.)
    } else if pos.y > rect.max.y {
        rect.max.y = (pos.y).clamp(0., 63.)
    }
}

// Shuflles range
pub fn rand_range(vec: Range<usize>) -> Vec<usize> {
    let mut vec: Vec<usize> = vec.collect();
    fastrand::shuffle(&mut vec);
    vec
}

// Transform pos to chunk coords
pub fn transform_to_chunk(pos: Vec2) -> Option<(IVec2, i32)> {
    if pos.x < 0. || pos.y < 0. {
        return None;
    }

    let (width, height) = (CHUNKS_WIDTH, CHUNKS_HEIGHT);

    let (chunk_x, chunk_y) = (
        (pos.x / (CHUNK_LENGHT * ATOM_SIZE) as f32) as usize,
        (pos.y / (CHUNK_LENGHT * ATOM_SIZE) as f32) as usize,
    );

    if chunk_x >= width || chunk_y >= height {
        return None;
    }

    let (atom_x, atom_y) = (
        ((pos.x / ATOM_SIZE as f32) % CHUNK_LENGHT as f32) as i32,
        ((pos.y / ATOM_SIZE as f32) % CHUNK_LENGHT as f32) as i32,
    );

    let local = (ivec2(atom_x, atom_y), (chunk_y * width + chunk_x) as i32);

    Some(local)
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

pub fn split_left_right(
    array: &mut [Atom],
) -> ([&mut Atom; CHUNK_LEN / 2], [&mut Atom; CHUNK_LEN / 2]) {
    let (left, right): (Vec<_>, Vec<_>) = array
        .chunks_mut(CHUNK_LENGHT)
        .flat_map(|chunk| {
            let (left, right) = chunk.split_at_mut(HALF_CHUNK_LENGHT);
            left.iter_mut().zip(right.iter_mut()).collect::<Vec<_>>()
        })
        .unzip();

    (left.try_into().unwrap(), right.try_into().unwrap())
}

pub fn updown_to_leftright(
    array: &mut [Atom],
) -> ([&mut Atom; CHUNK_LEN / 4], [&mut Atom; CHUNK_LEN / 4]) {
    let (left, right): (Vec<_>, Vec<_>) = array
        .chunks_mut(CHUNK_LENGHT)
        .flat_map(|chunk| {
            let (left, right) = chunk.split_at_mut(HALF_CHUNK_LENGHT);
            left.iter_mut().zip(right.iter_mut()).collect::<Vec<_>>()
        })
        .unzip();

    (left.try_into().unwrap(), right.try_into().unwrap())
}

pub fn get_mutable_references<'a>(
    chunks: &'a mut [Chunk],
    mutable_references: &mut MutableReferences<'a>,
    thread_off: (usize, usize),
) {
    chunks.iter_mut().enumerate().for_each(|(idx, chunk)| {
        let chunk_x = idx % CHUNKS_WIDTH;
        let chunk_y = idx / CHUNKS_WIDTH;

        let same_x = (chunk_x + thread_off.0) % 2 == 0;
        let same_y = (chunk_y + thread_off.1) % 2 == 0;

        match (same_x, same_y) {
            (true, true) => mutable_references.centers.push(Some(
                chunk
                    .atoms
                    .iter_mut()
                    .collect::<Vec<_>>()
                    .try_into()
                    .unwrap(),
            )),
            (true, false) => {
                let (up, down) = chunk.atoms.split_at_mut(CHUNK_LEN / 2);

                mutable_references.sides[0]
                    .push(Some(up.iter_mut().collect::<Vec<_>>().try_into().unwrap()));
                mutable_references.sides[3].push(Some(
                    down.iter_mut().collect::<Vec<_>>().try_into().unwrap(),
                ));
            }
            (false, true) => {
                let (left, right) = split_left_right(&mut chunk.atoms);

                mutable_references.sides[1].push(Some(left));
                mutable_references.sides[2].push(Some(right));
            }

            (false, false) => {
                let (up, down) = chunk.atoms.split_at_mut(CHUNK_LEN / 2);

                let (up_left, up_right) = updown_to_leftright(up);
                let (down_left, down_right) = updown_to_leftright(down);

                mutable_references.corners[0].push(Some(up_left));
                mutable_references.corners[1].push(Some(up_right));
                mutable_references.corners[2].push(Some(down_left));
                mutable_references.corners[3].push(Some(down_right));
            }
        }
    });
}

// TODO make function less verbose
pub fn update_dirty_rects(
    pos: Vec2,
    new_dirty_rects: &mut [Option<Rect>],
    chunk_idx: usize,
    global_pos: IVec2,
    center_idx: i32,
) {
    if (1.0..62.0).contains(&pos.x) && (1.0..62.0).contains(&pos.y) {
        // Case where the 3x3 position area is within a chunk
        let rect = &mut new_dirty_rects[chunk_idx];
        if let Some(rect) = rect {
            extend_rect_if_needed(rect, &(pos + Vec2::ONE));
            extend_rect_if_needed(rect, &(pos + Vec2::NEG_ONE));
        } else {
            *rect = Some(Rect::new(pos.x - 1., pos.y - 1., pos.x + 1., pos.y + 1.));
        }
    } else if (pos.x == 0. || pos.x == 63.) && (1.0..62.0).contains(&pos.y) {
        // Case where the 3x3 position area is in another chunk into the left or right
        if let Some(rect) = &mut new_dirty_rects[chunk_idx] {
            extend_rect_if_needed(
                rect,
                &(pos
                    + if pos.x == 0. {
                        Vec2::NEG_Y
                    } else {
                        Vec2::NEG_ONE
                    }),
            );
            extend_rect_if_needed(rect, &(pos + if pos.x == 0. { Vec2::ONE } else { Vec2::Y }));
        } else {
            new_dirty_rects[chunk_idx] = Some(Rect::new(
                pos.x + if pos.x == 0. { 0. } else { -1. },
                pos.y - 1.,
                pos.x + if pos.x == 0. { 1. } else { 0. },
                pos.y + 1.,
            ));
        }

        let x = if pos.x == 0. { 63. } else { 0. };
        if (pos.x == 0. && chunk_idx % CHUNKS_WIDTH > 0)
            || (pos.x == 63. && chunk_idx % CHUNKS_WIDTH < CHUNKS_WIDTH - 1)
        {
            let rect = &mut new_dirty_rects[if pos.x == 0. {
                chunk_idx - 1
            } else {
                chunk_idx + 1
            }];
            if let Some(rect) = rect {
                extend_rect_if_needed(rect, &(vec2(x, pos.y + 1.)));
                extend_rect_if_needed(rect, &(vec2(x, pos.y - 1.)));
            } else {
                *rect = Some(Rect::new(x, pos.y - 1., x, pos.y + 1.));
            }
        }
    } else if (pos.y == 0. || pos.y == 63.) && (1.0..62.0).contains(&pos.x) {
        // Case where the 3x3 position area is in another chunk into the up or down
        if let Some(rect) = &mut new_dirty_rects[chunk_idx] {
            extend_rect_if_needed(
                rect,
                &(pos
                    + if pos.y == 0. {
                        Vec2::NEG_X
                    } else {
                        Vec2::NEG_ONE
                    }),
            );
            extend_rect_if_needed(rect, &(pos + if pos.y == 0. { Vec2::ONE } else { Vec2::X }));
        } else {
            new_dirty_rects[chunk_idx] = Some(Rect::new(
                pos.x - 1.,
                pos.y + if pos.y == 0. { 0. } else { -1. },
                pos.x + 1.,
                pos.y + if pos.y == 0. { 1. } else { 0. },
            ));
        }

        let y = if pos.y == 0. { 63. } else { 0. };
        if (pos.y == 0. && chunk_idx / CHUNKS_WIDTH > 0)
            || (pos.y == 63. && chunk_idx / CHUNKS_WIDTH < CHUNKS_HEIGHT - 1)
        {
            let rect = &mut new_dirty_rects[if pos.y == 0. {
                chunk_idx - CHUNKS_WIDTH
            } else {
                chunk_idx + CHUNKS_WIDTH
            }];
            if let Some(rect) = rect {
                extend_rect_if_needed(rect, &(vec2(pos.x + 1., y)));
                extend_rect_if_needed(rect, &(vec2(pos.x - 1., y)));
            } else {
                *rect = Some(Rect::new(pos.x - 1., y, pos.x + 1., y));
            }
        }
    } else {
        // Case where the 3x3 position are is in the corner of a chunk
        for y in -1..=1 {
            for x in -1..=1 {
                let local = global_to_local(global_pos + ivec2(x, y));
                let pos = local.0.as_vec2();
                let chunk_manager_idx = ChunkGroup::group_to_manager_idx(center_idx, local.1);

                if let Some(rect) = new_dirty_rects.get_mut(chunk_manager_idx) {
                    if let Some(rect) = rect {
                        extend_rect_if_needed(rect, &pos)
                    } else {
                        *rect = Some(Rect::new(pos.x, pos.y, pos.x, pos.y));
                    }
                }
            }
        }
    }
}

#[derive(Default)]
pub struct MutableReferences<'a> {
    pub centers: Vec<Option<[&'a mut Atom; CHUNK_LEN]>>,
    pub sides: [Vec<Option<[&'a mut Atom; HALF_CHUNK_LEN]>>; 4],
    pub corners: [Vec<Option<[&'a mut Atom; QUARTER_CHUNK_LEN]>>; 4],
}

/// A deferred update message.
/// Indicates that an image or dirty rect should udpate.
#[derive(Debug)]
pub struct DeferredDirtyRectUpdate {
    pub chunk_idx: usize,
    pub pos: Vec2,
    pub global_pos: IVec2,
    pub center_idx: i32,
}
