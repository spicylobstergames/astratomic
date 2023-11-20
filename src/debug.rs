use crate::prelude::*;
use bevy::sprite::Anchor;

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
                        color: Color::rgba(0.25, 0.25, 0.75, 0.50),
                        custom_size: Some(Vec2::new(
                            (rect.max.x - rect.min.x + 1.) * ATOM_SIZE as f32,
                            (rect.max.y - rect.min.y + 1.) * ATOM_SIZE as f32,
                        )),
                        anchor: Anchor::TopLeft,
                        ..default()
                    },
                    transform: Transform::from_translation(Vec3::new(
                        (chunk_x * CHUNK_LENGHT * ATOM_SIZE) as f32
                            + (rect.min.x * ATOM_SIZE as f32),
                        -((chunk_y * CHUNK_LENGHT * ATOM_SIZE) as f32)
                            - (rect.min.y * ATOM_SIZE as f32),
                        1.,
                    )),
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
        app.add_systems(Update, render_dirty_rects.after(chunk_manager_update));
    }
}
