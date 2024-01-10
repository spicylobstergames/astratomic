use bevy::render::render_resource::*;
use std::cmp::Ordering;
use std::collections::HashSet;

use crate::prelude::*;

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct Chunk {
    #[serde(with = "BigArray")]
    pub atoms: [Atom; CHUNK_LEN],

    #[serde(skip)]
    pub texture: Handle<Image>,
}

impl Default for Chunk {
    fn default() -> Self {
        Self {
            atoms: [Atom::default(); CHUNK_LEN],
            texture: Handle::default(),
        }
    }
}

impl Chunk {
    pub fn new(texture: Handle<Image>, index: IVec2) -> Chunk {
        let mut atoms = [Atom::default(); CHUNK_LEN];

        match index.y.cmp(&2) {
            Ordering::Less => {}
            Ordering::Equal => {
                for (i, atom) in atoms.iter_mut().enumerate() {
                    let id = match i {
                        0..=511 => 6,
                        512..=2815 => 7,
                        _ => 4,
                    };

                    *atom = Atom::new(id);
                }
            }

            Ordering::Greater => {
                for atom in &mut atoms {
                    *atom = Atom::new(8);
                }
            }
        }

        Chunk { atoms, texture }
    }

    pub fn new_image() -> Image {
        Image::new(
            Extent3d {
                height: CHUNK_LENGHT as u32,
                width: CHUNK_LENGHT as u32,
                ..Default::default()
            },
            TextureDimension::D2,
            vec![0; CHUNK_LENGHT.pow(2) * 4],
            TextureFormat::Rgba8UnormSrgb,
        )
    }

    //This uses the CPU and is not used in-game anymore
    pub fn update_image_positions(&self, image: &mut Image, positions: &HashSet<IVec2>) {
        for pos in positions {
            let pixel_index = (pos.y as usize * CHUNK_LENGHT + pos.x as usize) * 4;
            image.data[pixel_index..pixel_index + 4].copy_from_slice(&self.atoms[pos.d1()].color);
        }
    }

    pub fn update_all(&self, image: &mut Image) {
        let positions: HashSet<IVec2> = (0..CHUNK_LENGHT)
            .flat_map(|y| (0..CHUNK_LENGHT).map(move |x| IVec2::new(x as i32, y as i32)))
            .collect();

        self.update_image_positions(image, &positions)
    }
}
