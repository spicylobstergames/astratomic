use bevy::render::{render_asset::RenderAssetUsages, render_resource::*};
use noise::{Constant, NoiseFn, ScalePoint, Select};
use std::collections::HashSet;

use crate::{prelude::*, rigidbody};

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

struct YCoord;
impl NoiseFn<f64, 2> for YCoord {
    fn get(&self, point: [f64; 2]) -> f64 {
        point[1]
    }
}

impl Chunk {
    pub fn new(
        texture: Handle<Image>,
        index: IVec2,
        materials: &Materials,
        generator: &Generator,
    ) -> Chunk {
        let mut atoms = [Atom::default(); CHUNK_LEN];

        let air_limit = -100.;

        let (scale, cave) = (generator.1, generator.0.clone());

        let ground = ScalePoint::new(cave.clone()).set_y_scale(0.1);
        let air = Constant::new(-1.0);

        let generator = Select::new(ground, air, YCoord)
            .set_bounds(air_limit, 0.) // For y values within this range, use the flat surface.
            .set_falloff(0.1); // Smooth transition near the boundaries.
        let generator = Select::new(cave, generator, YCoord)
            .set_bounds(air_limit, 1.) // For y values within this range, use the flat surface.
            .set_falloff(0.6);

        for x in 0..CHUNK_LENGHT {
            for y in 0..CHUNK_LENGHT {
                let pos = [
                    (index.x as f64 * CHUNK_LENGHT as f64 + x as f64),
                    (index.y as f64 * CHUNK_LENGHT as f64 + y as f64),
                ];
                let gen_pos = [pos[0] / scale, pos[1] / scale];

                let noise = if false { -1. } else { generator.get(gen_pos) };

                let i = y * CHUNK_LENGHT + x;
                /*atoms[i].color = [
                    ((noise as f32 + 1.) / 2. * 255.) as u8,
                    ((noise as f32 + 1.) / 2. * 255.) as u8,
                    ((noise as f32 + 1.) / 2. * 255.) as u8,
                    255,
                ];*/
                let id = match (noise + 1.) / 2. {
                    ..0.2 => continue,
                    0.2..0.25 => 6,
                    0.25..0.3 => 2,
                    0.3..0.4 => 4,
                    0.4.. => 8,
                    _ => panic!("{noise}"),
                };

                atoms[i] = Atom::new(id, materials);
                /*atoms[i].color = [
                    ((noise as f32 + 1.) / 2. * 255.) as u8,
                    ((noise as f32 + 1.) / 2. * 255.) as u8,
                    ((noise as f32 + 1.) / 2. * 255.) as u8,
                    255,
                ];*/
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
            RenderAssetUsages::RENDER_WORLD,
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

    pub fn get_collider(&self) -> Option<Collider> {
        rigidbody::get_collider(&self.get_values(), CHUNK_LENGHT as u32, CHUNK_LENGHT as u32)
    }

    pub fn get_values(&self) -> Vec<f64> {
        let mut values = vec![];

        for row in self.atoms.chunks(CHUNK_LENGHT) {
            for atom in row {
                if atom.is_solid() {
                    values.push(1.)
                } else {
                    values.push(0.)
                }
            }
        }

        values
    }
}
