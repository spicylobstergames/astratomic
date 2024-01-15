use crate::prelude::*;
use geo::{Simplify, TriangulateEarcut};

#[derive(Component)]
pub struct Rigidbody {
    pub atoms: Vec<Atom>,
    pub width: u8,
    pub height: u8,
}

#[derive(Resource, Default)]
pub struct TreeHandle(pub Handle<Image>);

#[derive(Component)]
pub struct Tree;

pub fn load_tree(
    mut commands: Commands,
    server: Res<AssetServer>,
    mut tree_handle: ResMut<TreeHandle>,
) {
    let image: Handle<Image> = server.load("tree.png");
    tree_handle.0 = image.clone();
    commands.spawn(Tree).insert(SpriteBundle {
        texture: image,
        sprite: Sprite {
            anchor: bevy::sprite::Anchor::TopLeft,
            ..Default::default()
        },
        ..Default::default()
    });
}

pub fn add_tree(
    mut commands: Commands,
    images: Res<Assets<Image>>,
    trees: Query<Entity, (With<Tree>, Without<Collider>)>,
    tree_handle: Res<TreeHandle>,
) {
    for tree in &trees {
        let image = images.get(tree_handle.0.clone()).unwrap();

        let collider = get_collider(&image_values(image), image.width(), image.height()).unwrap();
        commands
            .entity(tree)
            .insert(collider)
            .insert(bevy_rapier2d::prelude::RigidBody::Dynamic);
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
        app.init_resource::<TreeHandle>()
            .add_systems(Startup, load_tree)
            .add_systems(Update, add_tree);
    }
}
