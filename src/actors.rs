use crate::prelude::*;

#[derive(Component, Clone, Copy)]
pub struct Actor {
    collider: IRect,
    pos: IVec2,
}

pub fn update_actors(mut commands: Commands, mut chunk_manager: Query<&mut ChunkManager>) {
    let chunk_manager = chunk_manager.single();
    let chunks = &chunk_manager.chunks;
}

pub struct ActorsPlugin;
impl Plugin for ActorsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, update_actors.after(chunk_manager_update));
    }
}
