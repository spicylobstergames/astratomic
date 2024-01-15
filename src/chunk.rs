use bevy::render::render_resource::*;
use geo::{Simplify, TriangulateEarcut};
use std::collections::HashSet;

use crate::prelude::*;

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct Chunk {
    #[serde(with = "BigArray")]
    pub atoms: [Atom; CHUNK_LEN],

    #[serde(skip)]
    pub texture: Handle<Image>,
    #[serde(skip)]
    pub entity: Option<Entity>,
}

impl Default for Chunk {
    fn default() -> Self {
        Self {
            atoms: [Atom::default(); CHUNK_LEN],
            texture: Handle::default(),
            entity: None,
        }
    }
}

impl Chunk {
    pub fn new(texture: Handle<Image>, index: IVec2) -> Chunk {
        let mut atoms = [Atom::default(); CHUNK_LEN];

        match index.y {
            i32::MIN..=0 => {}
            1 => {
                for (i, atom) in atoms.iter_mut().enumerate() {
                    let id = match i {
                        0..=511 => 6,
                        _ => 7,
                    };

                    *atom = Atom::new(id);
                }
            }
            2 => {
                for atom in &mut atoms {
                    *atom = Atom::new(4);
                }
            }
            3..=i32::MAX => {
                for atom in &mut atoms {
                    *atom = Atom::new(8);
                }
            }
        }

        Chunk {
            atoms,
            texture,
            entity: None,
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

    pub fn get_collider(&self, materials: &Materials) -> Option<Collider> {
        let c = ContourBuilder::new(CHUNK_LENGHT as u32, CHUNK_LENGHT as u32, false);

        let res = c.contours(&self.get_values(materials), &[0.5]).unwrap();
        let mut colliders = vec![];
        for countour in res {
            let geometry = countour.geometry().simplify(&4.0);

            for polygon in geometry {
                let triangles = polygon.earcut_triangles();
                for triangle in triangles {
                    let collider = Collider::triangle(
                        vec2(triangle.0.x as f32, -triangle.0.y as f32),
                        vec2(triangle.1.x as f32, -triangle.1.y as f32),
                        vec2(triangle.2.x as f32, -triangle.2.y as f32),
                    );
                    colliders.push((Vec2::ZERO, 0.0_f32, collider));
                }
            }
        }
        if !colliders.is_empty() {
            Some(Collider::compound(colliders))
        } else {
            None
        }
    }
    pub fn get_values(&self, materials: &Materials) -> Vec<f64> {
        let mut values = vec![];

        for row in self.atoms.chunks(CHUNK_LENGHT) {
            for atom in row {
                if materials[atom].is_solid() {
                    values.push(1.)
                } else {
                    values.push(0.)
                }
            }
        }

        values
    }
}
