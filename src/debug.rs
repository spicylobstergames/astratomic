use crate::prelude::*;
use bevy::sprite::Anchor;
use rand::Rng;

//TODO add a debug mode
fn _camera(keys: Res<Input<KeyCode>>, mut camera_q: Query<&mut Transform, With<Camera>>) {
    let x = -(keys.pressed(KeyCode::A) as u8 as f32) + keys.pressed(KeyCode::D) as u8 as f32;
    let y = -(keys.pressed(KeyCode::S) as u8 as f32) + keys.pressed(KeyCode::W) as u8 as f32;

    let v = Vec2::new(x, y).normalize_or_zero().extend(0.);
    camera_q.single_mut().translation += v * CAMERA_SPEED;
}

fn brush(
    window: Query<&Window>,
    camera_q: Query<(&Camera, &GlobalTransform)>,
    mut chunk_manager: Query<&mut ChunkManager>,
    mut dirty_rects: Query<&mut DirtyRects>,
    prev_mpos: Query<&PreviousMousePos>,
    input: (Res<Input<MouseButton>>, Res<Input<KeyCode>>),
) {
    let (state, color);

    if input.1.pressed(KeyCode::L) {
        state = State::Gas;
        color = [255, 255, 255, 255];
    } else if input.0.pressed(MouseButton::Middle) {
        state = State::Powder;
        color = [
            (230 + rand::thread_rng().gen_range(-20_i16..20_i16)) as u8,
            (197 + rand::thread_rng().gen_range(-20_i16..20_i16)) as u8,
            (92 + rand::thread_rng().gen_range(-20_i16..20_i16)) as u8,
            255,
        ];
    } else if input.1.pressed(KeyCode::ControlLeft) {
        state = State::Liquid;
        color = [
            (20 + rand::thread_rng().gen_range(-20_i16..20_i16)) as u8,
            (125 + rand::thread_rng().gen_range(-20_i16..20_i16)) as u8,
            (204 + rand::thread_rng().gen_range(-20_i16..20_i16)) as u8,
            255,
        ];
    } else if input.1.pressed(KeyCode::ShiftLeft) {
        state = State::Solid;
        color = [127, 131, 134, 255];
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
        let mut chunk_manager = chunk_manager.single_mut();
        let mut dirty_rects = dirty_rects.single_mut();
        world_position.y *= -1.;
        let prev_mpos = prev_mpos.single().0.unwrap();

        for v in Line::new(
            IVec2::new(prev_mpos.x as i32, prev_mpos.y as i32),
            IVec2::new(world_position.x as i32, world_position.y as i32)
                - IVec2::new(prev_mpos.x as i32, prev_mpos.y as i32),
        ) {
            if let Some(pos) = transform_to_chunk(v.as_vec2()) {
                let atom = Atom {
                    color,
                    state,
                    ..Default::default()
                };
                let chunk = &mut chunk_manager.chunks[pos.1 as usize];
                if chunk.atoms[pos.0.d1()].actor {
                    continue;
                }

                chunk.atoms[pos.0.d1()] = atom;

                // Update simultation rect
                if let Some(dirty_rect) = dirty_rects.current[pos.1 as usize].as_mut() {
                    extend_rect_if_needed(dirty_rect, &pos.0)
                } else {
                    dirty_rects.current[pos.1 as usize] =
                        Some(IRect::new(pos.0.x, pos.0.y, pos.0.x, pos.0.y))
                }

                // Update render rect
                if let Some(dirty_rect) = dirty_rects.render[pos.1 as usize].as_mut() {
                    extend_rect_if_needed(dirty_rect, &pos.0)
                } else {
                    dirty_rects.render[pos.1 as usize] =
                        Some(IRect::new(pos.0.x, pos.0.y, pos.0.x, pos.0.y))
                }
            }
        }
    }
}

#[derive(Component)]
pub struct PreviousMousePos(pub Option<Vec2>);

fn prev_mpos(
    window: Query<&Window>,
    camera_q: Query<(&Camera, &GlobalTransform)>,
    mut prev_mpos: Query<&mut PreviousMousePos>,
) {
    let (camera, camera_transform) = camera_q.single();
    let window = window.single();

    if let Some(mut world_position) = window
        .cursor_position()
        .and_then(|cursor| camera.viewport_to_world(camera_transform, cursor))
        .map(|ray| ray.origin.truncate())
    {
        world_position.y *= -1.;

        prev_mpos.single_mut().0 = Some(world_position);
    }
}

pub fn render_dirty_rects(
    mut commands: Commands,
    dirty_rects: Query<&DirtyRects>,
    rects: Query<Entity, With<DirtyRect>>,
) {
    let dirty_rects = &dirty_rects.single().current;

    for rect in rects.iter() {
        commands.entity(rect).despawn();
    }

    for (i, rect) in dirty_rects.iter().enumerate() {
        if let Some(rect) = rect {
            let chunk_x = i % CHUNKS_WIDTH;
            let chunk_y = i / CHUNKS_WIDTH;

            // Rectangle
            commands
                .spawn(SpriteBundle {
                    sprite: Sprite {
                        color: Color::rgba(0.25, 0.25, 0.75, 0.2),
                        custom_size: Some(
                            IVec2::new(
                                (rect.max.x - rect.min.x + 1) * ATOM_SIZE as i32,
                                (rect.max.y - rect.min.y + 1) * ATOM_SIZE as i32,
                            )
                            .as_vec2(),
                        ),
                        anchor: Anchor::TopLeft,
                        ..default()
                    },
                    transform: Transform::from_translation(
                        IVec3::new(
                            (chunk_x * CHUNK_LENGHT * ATOM_SIZE) as i32
                                + (rect.min.x * ATOM_SIZE as i32),
                            -((chunk_y * CHUNK_LENGHT * ATOM_SIZE) as i32)
                                - (rect.min.y * ATOM_SIZE as i32),
                            1,
                        )
                        .as_vec3(),
                    ),
                    ..default()
                })
                .insert(DirtyRect);
        }
    }
}

#[derive(Component)]
pub struct DirtyRect;

pub struct DebugPlugin;
impl Plugin for DebugPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                render_dirty_rects.after(chunk_manager_update),
                brush.after(chunk_manager_update),
                prev_mpos.after(brush),
            ),
        );
    }
}
