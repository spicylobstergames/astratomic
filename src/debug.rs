use crate::prelude::*;
use bevy::sprite::Anchor;

pub fn render_dirty_rects(
    mut commands: Commands,
    mut chunk_manager: Query<&mut ChunkManager>,
    rects: Query<Entity, With<DirtyRect>>,
) {
    let mut chunk_manager = chunk_manager.single_mut();

    for rect in rects.iter() {
        commands.entity(rect).despawn();
    }

    for (i, chunk) in chunk_manager.chunks.iter_mut().enumerate() {
        let rect = chunk.dirty_rect;

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
        app.add_systems(Update, render_dirty_rects.after(dirty_rects_update));
    }
}
