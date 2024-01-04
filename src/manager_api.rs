use std::ops::Range;
use std::panic;

use itertools::Itertools;
use rand::Rng;

use async_channel::Sender;

use crate::prelude::*;

pub struct UpdateChunksType<'a> {
    pub group: ChunkGroup<'a>,
    pub colliders: &'a ChunkColliders,
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

    let (chunk_x, chunk_y) = (pos.x / CHUNK_LENGHT as i32, pos.y / CHUNK_LENGHT as i32);

    let (x_off, y_off) = (
        (CHUNK_LENGHT as i32 - 1) - (pos.x % CHUNK_LENGHT as i32).abs(),
        (CHUNK_LENGHT as i32 - 1) - (pos.y % CHUNK_LENGHT as i32).abs(),
    );

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

/// Transforms global 3x3 chunk position to local 3x3 chunks position
/// Used for chunk multithreaded updates
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
/// Used for chunk multithreaded updates
pub fn local_to_global(pos: (IVec2, i32)) -> IVec2 {
    let range = 0..CHUNK_LENGHT as i32;
    if !range.contains(&pos.0.x) || !range.contains(&pos.0.y) || !(0..9).contains(&pos.1) {
        panic!(
            "Invalid position on local_to_global. {} {} {}",
            pos.0.x, pos.0.y, pos.1
        )
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
pub fn swapable(
    chunks: &UpdateChunksType,
    pos: IVec2,
    states: &[(State, f32)],
    dt: u8,
    state: State,
) -> bool {
    let local_pos = global_to_local(pos);
    let collidable = chunks
        .colliders
        .get_collider(&ChunkPos::new(
            local_pos.0.as_uvec2(),
            chunks.group.center_pos,
        ))
        .is_some();

    if let Some(atom) = chunks.group.get_global(pos) {
        (atom.state == State::Void
            || (states.iter().any(|&(state, prob)| {
                state == atom.state && rand::thread_rng().gen_range(0.0..1.0) < prob
            }) && atom.updated_at != dt))
            && (!collidable || state == State::Liquid)
    } else {
        false
    }
}

/// Gets down neighbours from a global pos
pub fn down_neigh(
    chunks: &UpdateChunksType,
    pos: IVec2,
    states: &[(State, f32)],
    state: State,
    dt: u8,
) -> [(bool, IVec2); 3] {
    let mut neigh = [(false, IVec2::ZERO); 3];

    for (neigh, x) in neigh.iter_mut().zip([0, -1, 1]) {
        neigh.0 = swapable(chunks, pos + IVec2::new(x, 1), states, dt, state);
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
    state: State,
    dt: u8,
) -> [(bool, IVec2); 2] {
    let mut neigh = [(false, IVec2::ZERO); 2];

    for (neigh, x) in neigh.iter_mut().zip([-1, 1]) {
        neigh.0 = swapable(chunks, pos + IVec2::new(x, 0), states, dt, state);
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

/// Gets fall speed from a global pos
pub fn get_fspeed(chunks: &UpdateChunksType, pos: IVec2) -> u8 {
    chunks.group[pos].fall_speed
}

/// Gets state from a global pos
pub fn get_state(chunks: &UpdateChunksType, pos: IVec2) -> State {
    chunks.group[pos].state
}

/// Sets fall speed from a global pos
pub fn set_fspeed(chunks: &mut UpdateChunksType, pos: IVec2, fall_speed: u8) {
    chunks.group[pos].fall_speed = fall_speed
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

//This splits up our chunks for the update step, while also mutably borrowing them, making a `ChunkReference`
//Some chunks are not chopped others are chopped up/down, left/right, and also in four corners.
//We do this because each center chunk needs half of the adjacent chunks
//So it needs up/down/left/right halves, and also four corners
//TODO Decrease individual atoms iterations to get a &mut Atom;
pub fn get_mutable_references<'a>(
    chunks: &'a mut HashMap<IVec2, Chunk>,
    mutable_references: &mut HashMap<IVec2, ChunkReference<'a>>,
    (x_toff, y_toff): (i32, i32),
    dirty_rects: &HashMap<IVec2, URect>,
    manager_pos: IVec2,
) {
    chunks
        .iter_mut()
        .filter(|(chunk_pos, _)| {
            let same_x = (chunk_pos.x + x_toff + manager_pos.x.abs() % 2) % 2 == 0;
            let same_y = (chunk_pos.y + y_toff + manager_pos.y.abs() % 2) % 2 == 0;
            let step_as_center = same_x && same_y;
            if step_as_center && dirty_rects.contains_key(*chunk_pos) {
                return true;
            } else if !step_as_center {
                let to_check = match (same_x, same_y) {
                    (false, false) => vec![ivec2(-1, -1), ivec2(-1, 1), ivec2(1, -1), ivec2(1, 1)],
                    (true, false) => vec![ivec2(0, -1), ivec2(0, 1)],
                    (false, true) => vec![ivec2(-1, 0), ivec2(1, 0)],
                    _ => unreachable!(),
                };
                for vec in to_check {
                    if dirty_rects.contains_key(&(**chunk_pos + vec)) {
                        return true;
                    }
                }
            }

            false
        })
        .for_each(|(chunk_pos, chunk)| {
            let same_x = (chunk_pos.x + x_toff + manager_pos.x.abs() % 2) % 2 == 0;
            let same_y = (chunk_pos.y + y_toff + manager_pos.y.abs() % 2) % 2 == 0;

            match (same_x, same_y) {
                (true, true) => {
                    mutable_references.insert(*chunk_pos, ChunkReference::Center(&mut chunk.atoms));
                }
                (true, false) => {
                    let (up, down) = chunk.atoms.split_at_mut(CHUNK_LEN / 2);

                    mutable_references.insert(
                        *chunk_pos,
                        ChunkReference::Side([
                            Some(up.iter_mut().collect::<Vec<_>>().try_into().unwrap()),
                            Some(down.iter_mut().collect::<Vec<_>>().try_into().unwrap()),
                        ]),
                    );
                }
                (false, true) => {
                    let (left, right) = split_left_right(&mut chunk.atoms);

                    mutable_references
                        .insert(*chunk_pos, ChunkReference::Side([Some(left), Some(right)]));
                }

                (false, false) => {
                    let (up, down) = chunk.atoms.split_at_mut(CHUNK_LEN / 2);

                    let (up_left, up_right) = updown_to_leftright(up);
                    let (down_left, down_right) = updown_to_leftright(down);

                    mutable_references.insert(
                        *chunk_pos,
                        ChunkReference::Corner([
                            Some(up_left),
                            Some(up_right),
                            Some(down_left),
                            Some(down_right),
                        ]),
                    );
                }
            }
        });
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

#[derive(Clone, Copy, Debug)]
pub struct ChunkPos {
    pub atom: UVec2,
    pub chunk: IVec2,
}

impl ChunkPos {
    pub fn new(atom: UVec2, chunk: IVec2) -> Self {
        Self { atom, chunk }
    }
}

pub enum ChunkReference<'a> {
    //Not chopped
    Center(&'a mut [Atom; CHUNK_LEN]),
    //Chopped in two
    Side([Option<[&'a mut Atom; HALF_CHUNK_LEN]>; 2]),
    //Chopped in four
    Corner([Option<[&'a mut Atom; QUARTER_CHUNK_LEN]>; 4]),
}

/// A deferred update message.
/// Indicates that an image or dirty rect should update.
#[derive(Debug)]
pub struct DeferredDirtyRectUpdate {
    pub chunk_pos: ChunkPos,
}
