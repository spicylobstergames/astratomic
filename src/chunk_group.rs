use crate::prelude::*;

pub type ChunkCorners<'a> = [Option<[&'a mut Atom; CHUNK_LEN / 4]>; 4];
pub type ChunkSides<'a> = [Option<[&'a mut Atom; CHUNK_LEN / 2]>; 4];

pub struct ChunkGroup<'a> {
    pub center: &'a mut [Atom; CHUNK_LEN],
    pub corners: ChunkCorners<'a>,
    pub sides: ChunkSides<'a>,
    /// Position of the center chunk.
    pub center_pos: IVec2,
}

impl<'a> ChunkGroup<'a> {
    pub fn new(center: &'a mut [Atom; CHUNK_LEN], center_pos: IVec2) -> Self {
        Self {
            center,
            corners: [None, None, None, None],
            sides: [None, None, None, None],
            center_pos,
        }
    }

    pub fn group_to_chunk(center_pos: IVec2, group_idx: i32) -> IVec2 {
        let x_diff = group_idx % 3 - 1;
        let y_diff = group_idx / 3 - 1;

        let mut pos = center_pos;
        pos.x += x_diff;
        pos.y += y_diff;

        pos
    }

    #[inline]
    pub fn surrounding_idx(idx: i32) -> i32 {
        match idx {
            0 | 1 => 0,
            2 | 3 => 1,
            6 | 5 => 2,
            8 | 7 => 3,
            _ => panic!("Invalid surrounding idx."),
        }
    }

    pub fn get_local(&self, idx: (IVec2, i32)) -> Option<&Atom> {
        let mut pos = idx.0;
        match idx.1 {
            // Center
            4 => Some(&self.center[idx.0.d1()]),
            // Corners
            0 | 2 | 6 | 8 => {
                // Offset position
                if idx.1 == 0 || idx.1 == 6 {
                    pos.x -= HALF_CHUNK_LENGHT as i32;
                }
                if idx.1 == 0 || idx.1 == 2 {
                    pos.y -= HALF_CHUNK_LENGHT as i32;
                }

                let index = pos.y * HALF_CHUNK_LENGHT as i32 + pos.x;
                let corner_index = ChunkGroup::surrounding_idx(idx.1) as usize;

                if let Some(corners) = &self.corners[corner_index] {
                    Some(corners[index as usize])
                } else {
                    None
                }
            }
            // Sides
            1 | 3 | 5 | 7 => {
                // Offset position
                if idx.1 == 1 {
                    pos.y -= HALF_CHUNK_LENGHT as i32
                } else if idx.1 == 3 {
                    pos.x -= HALF_CHUNK_LENGHT as i32
                }

                // Gets width accordingly if side is a top/down or left/right
                let width = if idx.1 == 3 || idx.1 == 5 {
                    HALF_CHUNK_LENGHT
                } else {
                    CHUNK_LENGHT
                } as i32;

                let index = pos.y * width + pos.x;
                let side_index = ChunkGroup::surrounding_idx(idx.1) as usize;

                if let Some(sides) = &self.sides[side_index] {
                    Some(sides[index as usize])
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    pub fn get_mut_local(&mut self, idx: (IVec2, i32)) -> Option<&mut Atom> {
        let mut pos = idx.0;
        match idx.1 {
            // Center
            4 => Some(&mut self.center[idx.0.d1()]),
            // Corners
            0 | 2 | 6 | 8 => {
                // Offset position
                if idx.1 == 0 || idx.1 == 6 {
                    pos.x -= HALF_CHUNK_LENGHT as i32;
                }
                if idx.1 == 0 || idx.1 == 2 {
                    pos.y -= HALF_CHUNK_LENGHT as i32;
                }

                let index = pos.y * HALF_CHUNK_LENGHT as i32 + pos.x;
                let corner_index = ChunkGroup::surrounding_idx(idx.1) as usize;

                if let Some(corners) = &mut self.corners[corner_index] {
                    Some(corners[index as usize])
                } else {
                    None
                }
            }
            // Sides
            1 | 3 | 5 | 7 => {
                // Offset position
                if idx.1 == 1 {
                    pos.y -= HALF_CHUNK_LENGHT as i32
                } else if idx.1 == 3 {
                    pos.x -= HALF_CHUNK_LENGHT as i32
                }

                // Gets width accordingly if side is a top/down or left/right
                let width = if idx.1 == 3 || idx.1 == 5 {
                    HALF_CHUNK_LENGHT
                } else {
                    CHUNK_LENGHT
                } as i32;

                let index = pos.y * width + pos.x;
                let side_index = ChunkGroup::surrounding_idx(idx.1) as usize;

                if let Some(sides) = &mut self.sides[side_index] {
                    Some(sides[index as usize])
                } else {
                    None
                }
            }
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
}

// Two ways to index a ChunkGroup to get an atom, by a global(IVec2) and by a local(IVec2, i32) pos
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

//Chunk References

pub enum ChunkReference<'a> {
    //Not chopped
    Center(&'a mut [Atom; CHUNK_LEN]),
    //Chopped in two
    Side([Option<[&'a mut Atom; HALF_CHUNK_LEN]>; 2]),
    //Chopped in four
    Corner([Option<[&'a mut Atom; QUARTER_CHUNK_LEN]>; 4]),
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
