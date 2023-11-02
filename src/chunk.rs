use bevy::prelude::*;
use bevy::render::render_resource::*;

use crate::atom::*;
use crate::consts::*;
use crate::grid_api::*;

pub struct Chunk {
    pub atoms: Vec<Atom>,
    pub texture: Handle<Image>,
    pub dirty_rect: Option<Rect>,
}

impl Chunk {
    pub fn new(texture: Handle<Image>) -> Chunk {
        Chunk {
            atoms: vec![Atom::new(); CHUNK_SIZE * CHUNK_SIZE],
            texture,
            dirty_rect: None,
        }
    }

    pub fn new_image() -> Image {
        Image::new(
            Extent3d {
                height: (CHUNK_SIZE * ATOM_SIZE) as u32,
                width: (CHUNK_SIZE * ATOM_SIZE) as u32,
                ..Default::default()
            },
            TextureDimension::D2,
            vec![0; (CHUNK_SIZE * ATOM_SIZE).pow(2) * 4],
            TextureFormat::Rgba8UnormSrgb,
        )
    }

    pub fn update_image_positions(&self, image: &mut Image, positions: &Vec<IVec2>) {
        for pos in positions {
            let atom_color = &self.atoms[pos.d1()].color;

            for y in 0..ATOM_SIZE {
                for x in 0..ATOM_SIZE {
                    let pixel_index = ((pos.y as usize * ATOM_SIZE + y) * CHUNK_SIZE * ATOM_SIZE
                        + (pos.x as usize * ATOM_SIZE + x))
                        * 4;

                    image.data[pixel_index..pixel_index + 4].copy_from_slice(atom_color);
                }
            }
        }
    }

    pub fn update_all(&self, image: &mut Image) {
        let positions: Vec<IVec2> = (0..CHUNK_SIZE)
            .flat_map(|y| (0..CHUNK_SIZE).map(move |x| IVec2::new(x as i32, y as i32)))
            .collect();

        self.update_image_positions(image, &positions)
    }
}
