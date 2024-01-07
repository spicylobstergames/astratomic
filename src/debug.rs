use crate::prelude::*;
use bevy::sprite::Anchor;

use bevy::diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin};
use bevy_inspector_egui::quick::WorldInspectorPlugin;

//TODO add a debug mode

fn brush(
    window: Query<&Window>,
    camera_q: Query<(&Camera, &GlobalTransform)>,
    mut chunk_manager: ResMut<ChunkManager>,
    mut dirty_rects: ResMut<DirtyRects>,
    prev_mpos: Res<PreviousMousePos>,
    input: (Res<Input<MouseButton>>, Res<Input<KeyCode>>),
) {
    let atom;
    if input.0.pressed(MouseButton::Middle) {
        atom = Atom::new(0);
    } else if input.1.pressed(KeyCode::ControlLeft) {
        atom = Atom::new(1);
    } else if input.1.pressed(KeyCode::ShiftLeft) {
        atom = Atom::new(2);
    } else {
        return;
    }

    let (camera, camera_transform) = camera_q.single();
    let window = window.single();

    if let Some(mut world_position) = window
        .cursor_position()
        .and_then(|cursor| camera.viewport_to_world(camera_transform, cursor))
        .map(|ray| ray.origin.truncate())
    {
        world_position.y *= -1.;

        for v in Line::new(
            prev_mpos.0.unwrap().as_ivec2(),
            world_position.as_ivec2() - prev_mpos.0.unwrap().as_ivec2(),
        ) {
            let pos = global_to_chunk(v);

            //Checks if there is a atom at the pos
            if chunk_manager.get_atom(&pos).is_none() {
                continue;
            }

            chunk_manager[pos] = atom;

            // Update simultation rect
            if let Some(dirty_rect) = dirty_rects.current.get_mut(&pos.chunk) {
                extend_rect_if_needed(dirty_rect, &pos.atom)
            } else {
                dirty_rects.current.insert(
                    pos.chunk,
                    URect::new(pos.atom.x, pos.atom.y, pos.atom.x, pos.atom.y),
                );
            }

            // Update render rect
            if let Some(dirty_rect) = dirty_rects.render.get_mut(&pos.chunk) {
                extend_rect_if_needed(dirty_rect, &pos.atom)
            } else {
                dirty_rects.render.insert(
                    pos.chunk,
                    URect::new(pos.atom.x, pos.atom.y, pos.atom.x, pos.atom.y),
                );
            }
        }
    }
}

#[derive(Resource, Default)]
pub struct PreviousMousePos(pub Option<Vec2>);

fn prev_mpos(
    window: Query<&Window>,
    camera_q: Query<(&Camera, &GlobalTransform)>,
    mut prev_mpos: ResMut<PreviousMousePos>,
) {
    let (camera, camera_transform) = camera_q.single();
    let window = window.single();

    if let Some(mut world_position) = window
        .cursor_position()
        .and_then(|cursor| camera.viewport_to_world(camera_transform, cursor))
        .map(|ray| ray.origin.truncate())
    {
        world_position.y *= -1.;

        prev_mpos.0 = Some(world_position);
    }
}

//Debug Render systems

pub fn render_dirty_rects(mut commands: Commands, dirty_rects: Res<DirtyRects>) {
    let (dirty_update, render_update) = (&dirty_rects.new, &dirty_rects.render);

    let mut i = 0.;
    for rect in [dirty_update, render_update] {
        for (chunk_pos, rect) in rect {
            commands
                .spawn(SpriteBundle {
                    sprite: Sprite {
                        color: Color::rgba(i, 0.25, if i == 0. { 1. } else { 0. }, 0.1),
                        custom_size: Some(
                            UVec2::new(rect.max.x - rect.min.x + 1, rect.max.y - rect.min.y + 1)
                                .as_vec2(),
                        ),
                        anchor: Anchor::TopLeft,
                        ..default()
                    },
                    transform: Transform::from_translation(
                        IVec3::new(
                            chunk_pos.x * CHUNK_LENGHT as i32 + rect.min.x as i32,
                            -(chunk_pos.y * CHUNK_LENGHT as i32) - rect.min.y as i32,
                            1,
                        )
                        .as_vec3(),
                    ),
                    ..default()
                })
                .insert(DeleteImage);
        }
        i += 1.;
    }
}

fn render_actors(mut commands: Commands, actors: Query<&Actor>) {
    for actor in actors.iter() {
        commands
            .spawn(SpriteBundle {
                sprite: Sprite {
                    color: Color::rgba(0.75, 0.25, 0.25, 0.2),
                    custom_size: Some(Vec2::new(actor.width as f32, actor.height as f32)),
                    anchor: Anchor::TopLeft,
                    ..default()
                },
                transform: Transform::from_translation(Vec3::new(
                    actor.pos.x as f32,
                    -actor.pos.y as f32,
                    1.,
                )),
                ..default()
            })
            .insert(DeleteImage);
    }
}

pub fn delete_image(mut commands: Commands, debug_images: Query<Entity, With<DeleteImage>>) {
    for image in debug_images.iter() {
        commands.entity(image).despawn();
    }
}

#[derive(Component)]
pub struct DeleteImage;

fn _camera(keys: Res<Input<KeyCode>>, mut camera_q: Query<&mut Transform, With<Camera>>) {
    let x = -(keys.pressed(KeyCode::A) as u8 as f32) + keys.pressed(KeyCode::D) as u8 as f32;
    let y = -(keys.pressed(KeyCode::S) as u8 as f32) + keys.pressed(KeyCode::W) as u8 as f32;

    let v = Vec2::new(x, y).normalize_or_zero().extend(0.);
    camera_q.single_mut().translation += v * _CAMERA_SPEED;
}

pub struct DebugPlugin;
impl Plugin for DebugPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                render_dirty_rects.after(update_actors),
                brush.after(chunk_manager_update),
                render_actors.after(update_actors),
                prev_mpos.after(brush),
                //_camera
            ),
        )
        .add_systems(PreUpdate, delete_image)
        .add_plugins(WorldInspectorPlugin::new())
        //Frame on console
        .add_plugins((LogDiagnosticsPlugin::default(), FrameTimeDiagnosticsPlugin))
        .init_resource::<PreviousMousePos>();
    }
}
