use crate::prelude::*;

pub type ChunkCorners<'a> = [Option<[&'a mut Atom; CHUNK_LEN / 4]>; 4];
pub type ChunkSides<'a> = [Option<[&'a mut Atom; CHUNK_LEN / 2]>; 4];

pub struct ChunkGroup<'a> {
    pub center: [&'a mut Atom; CHUNK_LEN],
    pub corners: ChunkCorners<'a>,
    pub sides: ChunkSides<'a>,
    // The index of the center chunk on the chunk manager vec
    pub center_index: i32,
}

impl<'a> ChunkGroup<'a> {
    pub fn new(center: [&'a mut Atom; CHUNK_LEN], center_index: usize) -> Self {
        Self {
            center,
            corners: [None, None, None, None],
            sides: [None, None, None, None],
            center_index: center_index as i32,
        }
    }

    pub fn group_to_manager_idx(center_manager_idx: i32, group_idx: i32) -> usize {
        if group_idx == 4 {
            center_manager_idx as usize
        } else {
            let x_diff = group_idx % 3 - 1;
            let y_diff = group_idx / 3 - 1;

            let mut index = center_manager_idx;
            index += y_diff * CHUNKS_WIDTH as i32 + x_diff;

            index as usize
        }
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
            4 => Some(self.center[idx.0.d1()]),
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
