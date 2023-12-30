use bevy::render::render_resource::*;
use rand::Rng;
use std::cmp::Ordering;
use std::collections::HashSet;

use crate::prelude::*;

pub struct Chunk {
    pub atoms: [Atom; CHUNK_LEN],
    pub texture: Handle<Image>,
    pub index: IVec2,
}

impl Chunk {
    pub fn new(texture: Handle<Image>, index: IVec2) -> Chunk {
        let mut atoms = [Atom::new(); CHUNK_LEN];

        match index.y.cmp(&2) {
            Ordering::Less => {}
            _ => {
                for atom in &mut atoms {
                    atom.state = crate::prelude::State::Powder;
                    atom.color = [
                        (230 + rand::thread_rng().gen_range(-20_i16..20_i16)) as u8,
                        (197 + rand::thread_rng().gen_range(-20_i16..20_i16)) as u8,
                        (92 + rand::thread_rng().gen_range(-20_i16..20_i16)) as u8,
                        255,
                    ];
                }
            }
        }

        Chunk {
            atoms,
            texture,
            index,
        }
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
