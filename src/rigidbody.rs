use crate::prelude::*;
use geo::{SimplifyVwPreserve, TriangulateEarcut};
use itertools::Itertools;

#[derive(Component)]
pub struct Rigidbody {
    pub atoms: Vec<Atom>,
    pub width: u8,
    pub height: u8,
    // Positions to remove Object atom later
    pub filled: Vec<ChunkPos>,
}

#[derive(Component, Default)]
pub struct RigidbodyHandle(Handle<Image>, Vec2);

#[derive(Component)]
pub struct Hydrated;

//TODO Add rigidbody file
pub fn load_images(mut commands: Commands, server: Res<AssetServer>) {
    let image: Handle<Image> = server.load("tree.png");
    commands.spawn(RigidbodyHandle(image.clone(), vec2(64., -64.)));

    let image: Handle<Image> = server.load("player/player_tool.png");
    commands.spawn(RigidbodyHandle(image.clone(), vec2(0., 0.)));
    let image: Handle<Image> = server.load("player/jetpack.png");
    commands.spawn(RigidbodyHandle(image.clone(), vec2(-64., 0.)));
}

pub fn add_rigidbodies(
    mut commands: Commands,
    images: Res<Assets<Image>>,
    handles: Query<(Entity, &RigidbodyHandle), Without<Hydrated>>,
) {
    for (ent, handle) in &handles {
        let image = images.get(handle.0.clone()).unwrap().clone();

        let rigidbody = Rigidbody {
            atoms: image_atoms(&image),
            height: image.height() as u8,
            width: image.width() as u8,
            filled: vec![],
        };

        let collider = get_collider(&image_values(&image), image.width(), image.height()).unwrap();

        commands
            .spawn(collider)
            .insert(rigidbody)
            .insert(bevy_rapier2d::prelude::RigidBody::Dynamic)
            .insert(SpriteBundle {
                texture: handle.0.clone(),
                sprite: Sprite {
                    anchor: bevy::sprite::Anchor::TopLeft,
                    ..Default::default()
                },
                transform: Transform::from_xyz(handle.1.x, -handle.1.y, RIGIDBODY_LAYER),
                ..Default::default()
            });

        commands.entity(ent).insert(Hydrated);
    }
}

pub fn update_rigidibodies(
    mut chunk_manager: ResMut<ChunkManager>,
    mut rigidbodies: Query<(&Transform, &mut Rigidbody)>,
    materials: (Res<Assets<Materials>>, Res<MaterialsHandle>),
    mut dirty_rects: ResMut<DirtyRects>,
) {
    puffin::profile_function!();

    let materials = materials.0.get(materials.1 .0.clone()).unwrap();

    for (transform, mut rigidbody) in &mut rigidbodies {
        let (width, height) = (rigidbody.width as usize, rigidbody.height as usize);
        let angle = -transform.rotation.to_euler(EulerRot::XYZ).2;
        let mut top_left = transform.translation.xy();
        top_left.y *= -1.;

        //This fills the chunks with Object atoms
        for (y, x) in (0..height).cartesian_product(0..width) {
            let pos = top_left + vec2(x as f32, y as f32).rotate(Vec2::from_angle(angle));
            let chunk_pos = global_to_chunk(pos.round().as_ivec2());

            let rotated_atom = rigidbody.atoms[y * width + x];
            if materials[&rotated_atom].is_solid() {
                update_dirty_rects_3x3(&mut dirty_rects.current, chunk_pos);

                if let Some(atom) = chunk_manager.get_mut_atom(chunk_pos) {
                    if materials[atom.id].is_void() {
                        *atom = Atom::object();
                        rigidbody.filled.push(chunk_pos);
                    }
                }
            }
        }
    }
}

pub fn unfill_rigidbodies(
    mut chunk_manager: ResMut<ChunkManager>,
    mut rigidbodies: Query<&mut Rigidbody>,
    materials: (Res<Assets<Materials>>, Res<MaterialsHandle>),
) {
    let materials = materials.0.get(materials.1 .0.clone()).unwrap();

    for mut rigidbody in &mut rigidbodies {
        while let Some(chunk_pos) = rigidbody.filled.pop() {
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
            atoms.push(Atom::default())
        }
    }

    atoms
}

pub fn get_collider(values: &[f64], width: u32, height: u32) -> Option<Collider> {
    let c = ContourBuilder::new(width, height, false);

    let res = c.contours(values, &[0.5]).unwrap();
    let mut colliders = vec![];
    for countour in res {
        let geometry = countour.geometry().simplify_vw_preserve(&1.5);

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
            .add_systems(Update, add_rigidbodies.run_if(in_state(GameState::Game)))
            .add_systems(
                FixedUpdate,
                (
                    update_rigidibodies.before(chunk_manager_update),
                    unfill_rigidbodies.after(chunk_manager_update),
                )
                    .run_if(in_state(GameState::Game)),
            );
    }
}
