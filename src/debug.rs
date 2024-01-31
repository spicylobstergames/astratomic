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
        atom = Atom::new(2);
    } else if input.1.pressed(KeyCode::ControlLeft) {
        atom = Atom::new(3);
    } else if input.1.pressed(KeyCode::ShiftLeft) {
        atom = Atom::new(8);
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

pub fn render_dirty_rects_gizmos(mut gizmos: Gizmos, dirty_rects: Res<DirtyRects>) {
    let (dirty_update, render_update) = (&dirty_rects.new, &dirty_rects.render);

    let mut i = false;
    for rect in [dirty_update, render_update] {
        for (chunk_pos, rect) in rect {
            let size =
                UVec2::new(rect.max.x - rect.min.x + 1, rect.max.y - rect.min.y + 1).as_vec2();
            let position = ivec2(
                chunk_pos.x * CHUNK_LENGHT as i32 + rect.min.x as i32,
                -(chunk_pos.y * CHUNK_LENGHT as i32) - rect.min.y as i32,
            )
            .as_vec2()
                + vec2(size.x, -(size.y)) / 2.;

            if !i {
                gizmos.rect_2d_gradient(position, 0., size);
            } else {
                gizmos.rect_2d(position, 0., size, Color::rgba(1., 0.25, 1., 1.));
            }
        }
        i = true;
    }
}

pub fn _render_dirty_rects_image(mut commands: Commands, dirty_rects: Res<DirtyRects>) {
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

pub fn _delete_image(mut commands: Commands, debug_images: Query<Entity, With<DeleteImage>>) {
    for image in debug_images.iter() {
        commands.entity(image).despawn();
    }
}

#[derive(Component)]
pub struct DeleteImage;

fn render_actors(mut gizmos: Gizmos, actors: Query<&Actor>) {
    for actor in actors.iter() {
        gizmos.rect_2d(
            vec2(actor.pos.x as f32, -actor.pos.y as f32)
                + vec2(actor.width as f32, -(actor.height as f32)) / 2.,
            0.,
            Vec2::new(actor.width as f32, actor.height as f32),
            Color::rgba(0.75, 0.25, 0.25, 0.2),
        );
    }
}

fn _camera(keys: Res<Input<KeyCode>>, mut camera_q: Query<&mut Transform, With<Camera>>) {
    let x = -(keys.pressed(KeyCode::A) as u8 as f32) + keys.pressed(KeyCode::D) as u8 as f32;
    let y = -(keys.pressed(KeyCode::S) as u8 as f32) + keys.pressed(KeyCode::W) as u8 as f32;

    let v = Vec2::new(x, y).normalize_or_zero().extend(0.);
    camera_q.single_mut().translation += v * _CAMERA_SPEED;
}

pub trait Rect2dGradient {
    fn rect_2d_gradient(&mut self, position: Vec2, rotation: f32, size: Vec2);
}

impl<'a> Rect2dGradient for Gizmos<'a> {
    fn rect_2d_gradient(&mut self, position: Vec2, rotation: f32, size: Vec2) {
        let rotation = Mat2::from_angle(rotation);
        let [tl, tr, br, bl] = rect_inner(size).map(|vec2| position + rotation * vec2);
        self.linestrip_gradient_2d([
            (tl, Color::RED),
            (tr, Color::GREEN),
            (br, Color::BLUE),
            (bl, Color::PURPLE),
            (tl, Color::GREEN),
        ]);
    }
}

fn rect_inner(size: Vec2) -> [Vec2; 4] {
    let half_size = size / 2.;
    let tl = Vec2::new(-half_size.x, half_size.y);
    let tr = Vec2::new(half_size.x, half_size.y);
    let bl = Vec2::new(-half_size.x, -half_size.y);
    let br = Vec2::new(half_size.x, -half_size.y);
    [tl, tr, br, bl]
}

pub fn grab_rigidbodies(
    mut commands: Commands,
    rigidbodies: Query<(&mut Transform, Entity, &Collider), With<Rigidbody>>,
    mut transforms: Query<&mut Transform, (With<ImpulseJoint>, Without<Rigidbody>)>,
    window: Query<&Window>,
    camera_q: Query<(&Camera, &GlobalTransform)>,
    mut grabbed: ResMut<Grabbed>,
    input: Res<Input<KeyCode>>,
) {
    let (camera, camera_transform) = camera_q.single();
    let window = window.single();

    if input.pressed(KeyCode::R) {
        if let Some(world_position) = window
            .cursor_position()
            .and_then(|cursor| camera.viewport_to_world(camera_transform, cursor))
            .map(|ray| ray.origin.truncate())
        {
            if let Some(grabbed_ent) = grabbed.0 {
                //Update pos
                let mut transform = transforms.get_mut(grabbed_ent).unwrap();
                transform.translation.x = world_position.x;
                transform.translation.y = world_position.y;
            } else {
                //Add joint
                for (transform, entity, collider) in &rigidbodies {
                    if collider.contains_point(
                        transform.translation.xy(),
                        transform.rotation.to_euler(EulerRot::XYZ).2,
                        world_position,
                    ) {
                        let anchor = (world_position - transform.translation.xy()).rotate(
                            Vec2::from_angle(-transform.rotation.to_euler(EulerRot::XYZ).2),
                        );
                        let joint = RevoluteJointBuilder::new()
                            .local_anchor1(anchor)
                            .local_anchor2(Vec2::ZERO);

                        let move_ent = commands
                            .spawn(RigidBody::KinematicPositionBased)
                            .insert(ImpulseJoint::new(entity, joint))
                            .insert(TransformBundle::from_transform(Transform::from_xyz(
                                world_position.x,
                                world_position.y,
                                0.,
                            )))
                            .id();

                        grabbed.0 = Some(move_ent);

                        break;
                    }
                }
            }
        }
    } else if let Some(ent) = grabbed.0.take() {
        commands.entity(ent).despawn_recursive()
    }
}

#[derive(Resource, Default)]
pub struct Grabbed(Option<Entity>);

pub struct DebugPlugin;
impl Plugin for DebugPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                render_dirty_rects_gizmos.after(update_actors),
                brush.after(chunk_manager_update),
                render_actors.after(update_actors),
                prev_mpos.after(brush),
                grab_rigidbodies,
                //_camera
            )
                .run_if(in_state(GameState::Game)),
        )
        .add_plugins(WorldInspectorPlugin::new())
        .add_plugins(RapierDebugRenderPlugin::default())
        //Frame on console
        .add_plugins((LogDiagnosticsPlugin::default(), FrameTimeDiagnosticsPlugin))
        .init_resource::<PreviousMousePos>()
        .init_resource::<Grabbed>();
    }
}
