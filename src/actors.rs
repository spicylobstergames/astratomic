use bevy::prelude::*;

use crate::grid::*;

#[derive(Component, Clone, Copy)]
pub struct Actor(Rect);

#[derive(Component)]
pub struct Player;

pub fn update_actors(
    mut commands: Commands,
    mut grid: Query<&mut Grid>,
    mut player: Query<(&Actor, &mut Transform), With<Player>>,
    mut camera_q: Query<&mut Transform, With<Camera>>,
) {
    let grid = grid.single();
    let dt = grid.dt;
    let chunks = &grid.chunks;
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
        app.add_systems(Update, update_actors.after(grid_update));
    }
}
