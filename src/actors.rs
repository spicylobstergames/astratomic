use crate::prelude::*;

#[derive(Component, Clone, Copy)]
pub struct Actor(Rect);

#[derive(Component)]
pub struct Player;

pub fn update_actors(
    mut commands: Commands,
    mut chunk_manager: Query<&mut ChunkManager>,
    mut player: Query<(&Actor, &mut Transform), With<Player>>,
    mut camera_q: Query<&mut Transform, With<Camera>>,
) {
    let chunk_manager = chunk_manager.single();
    let dt = chunk_manager.dt;
    let chunks = &chunk_manager.chunks;
    let mut camera = camera_q.single_mut();
    let mut player = player.single_mut();

    //Update player
    update_player(player.0, &mut player.1, &mut camera);
}

/// Updates player and returns atoms awakened
pub fn update_player(
    actor: &Actor,
    transform: &mut Transform,
    camera_transform: &mut Transform,
) -> Vec<IVec2> {
    vec![]
}

pub struct ActorsPlugin;
impl Plugin for ActorsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, update_actors.after(chunk_manager_update));
    }
}
