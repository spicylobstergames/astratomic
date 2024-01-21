use crate::prelude::*;
use geo::{Simplify, TriangulateEarcut};
use itertools::Itertools;

#[derive(Component)]
pub struct Rigidbody {
    pub atoms: Vec<Atom>,
    pub width: u8,
    pub height: u8,
}

#[derive(Component, Default)]
pub struct RigidbodyHandle(pub Handle<Image>);

#[derive(Component)]
pub struct Hydrated;

//TODO Add rigidbody file
pub fn load_images(mut commands: Commands, server: Res<AssetServer>) {
    let image: Handle<Image> = server.load("tree.png");
    commands.spawn(RigidbodyHandle(image.clone()));
}

pub fn add_rigidbodies(
    mut commands: Commands,
    images: Res<Assets<Image>>,
    handles: Query<(Entity, &RigidbodyHandle), Without<Hydrated>>,
) {
    for (ent, handle) in &handles {
        let image = images.get(handle.0.clone()).unwrap();

        let collider = get_collider(&image_values(image), image.width(), image.height()).unwrap();
        let rigidbody = Rigidbody {
            atoms: image_atoms(image),
            height: image.height() as u8,
            width: image.width() as u8,
        };

        commands
            .spawn(collider)
            .insert(rigidbody)
            .insert(SpriteBundle {
                texture: handle.0.clone(),
                sprite: Sprite {
                    anchor: bevy::sprite::Anchor::TopLeft,
                    ..Default::default()
                },
                transform: Transform::from_xyz(0., 64., 0.),
                ..Default::default()
            })
            .insert(bevy_rapier2d::prelude::RigidBody::Dynamic);

        commands.entity(ent).insert(Hydrated);
    }
}

pub fn fill_rigidbodies(
    mut chunk_manager: ResMut<ChunkManager>,
    rigidbodies: Query<(&Transform, &Rigidbody)>,
    materials: (Res<Assets<Materials>>, Res<MaterialsHandle>),
) {
    let materials = materials.0.get(materials.1 .0.clone()).unwrap();

    for (transform, rigidbody) in &rigidbodies {
        for (x, y) in (0..rigidbody.width).cartesian_product(0..rigidbody.height) {
            if materials[rigidbody.atoms[y as usize * rigidbody.width as usize + x as usize].id]
                .is_solid()
            {
                let angle = vec2(transform.rotation.z.cos(), transform.rotation.z.sin());
                let pos = vec2(x as f32, y as f32).rotate(angle) + transform.translation.xy();
                //pos.y *= -1.;

                let chunk_pos = global_to_chunk(pos.as_ivec2());

                if let Some(atom) = chunk_manager.get_mut_atom(chunk_pos) {
                    if materials[atom.id].is_void() {
                        let mut new_atom = Atom::object();
                        new_atom.color = [255, 255, 255, 255];

                        *atom = new_atom;
                    }
                }
            }
        }
    }
}

pub fn unfill_rigidbodies(
    mut chunk_manager: ResMut<ChunkManager>,
    rigidbodies: Query<(&Transform, &Rigidbody)>,
    materials: (Res<Assets<Materials>>, Res<MaterialsHandle>),
) {
    let materials = materials.0.get(materials.1 .0.clone()).unwrap();

    for (transform, rigidbody) in &rigidbodies {
        for (x, y) in (0..rigidbody.width).cartesian_product(0..rigidbody.height) {
            let angle = vec2(-transform.rotation.z.cos(), -transform.rotation.z.sin());
            let pos = vec2(x as f32, y as f32).rotate(angle) + transform.translation.xy();
            //pos.y *= -1.;

            let chunk_pos = global_to_chunk(pos.as_ivec2());

            if let Some(atom) = chunk_manager.get_mut_atom(chunk_pos) {
                if materials[atom.id].is_object() {
                    *atom = Atom::default();
                }
            }
        }
    }
}

pub fn image_values(image: &Image) -> Vec<f64> {
    let mut values = vec![];
    for pixel in image.data.chunks_exact(4) {
        if pixel[3] > 0 {
            values.push(1.)
        } else {
            values.push(0.)
        }
    }

    values
}

pub fn image_atoms(image: &Image) -> Vec<Atom> {
    let mut atoms = vec![];
    for pixel in image.data.chunks_exact(4) {
        if pixel[3] > 0 {
            atoms.push(Atom {
                id: 8,
                color: pixel.try_into().unwrap(),
                ..Default::default()
            });
        } else {
            atoms.push(Atom {
                id: 0,
                color: pixel.try_into().unwrap(),
                ..Default::default()
            })
        }
    }

    atoms
}

pub fn get_collider(values: &[f64], width: u32, height: u32) -> Option<Collider> {
    let c = ContourBuilder::new(width, height, false);

    let res = c.contours(values, &[0.5]).unwrap();
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

pub struct RigidbodyPlugin;
impl Plugin for RigidbodyPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, load_images)
            .add_systems(Update, add_rigidbodies)
            .add_systems(
                FixedUpdate,
                (
                    fill_rigidbodies.before(chunk_manager_update),
                    unfill_rigidbodies.after(chunk_manager_update),
                ),
            );
    }
}
