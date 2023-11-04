use bevy::prelude::*;

use crate::grid_api::*;

#[derive(Component, Clone, Copy)]
pub struct Actor(Rect);

#[derive(Component)]
pub struct Player;

/// Updates player and returns atoms awakened
fn update_player(chunks: &UpdateChunksType, dt: f32) -> Vec<IVec2> {
    vec![]
}
