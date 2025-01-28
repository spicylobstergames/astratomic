use crate::prelude::*;

#[derive(Component)]
pub struct AnimationIndices {
    pub first: usize,
    pub last: usize,
}

#[derive(Component, Deref, DerefMut)]
pub struct AnimationTimer(pub Timer);

fn animate_sprite(
    time: Res<Time>,
    mut query: Query<(&AnimationIndices, &mut AnimationTimer, &mut Sprite)>,
) {
    for (indices, mut timer, mut sprite) in &mut query {
        if let Some(atlas) = &mut sprite.texture_atlas {
            timer.tick(time.delta());
            if timer.just_finished() {
                atlas.index = if !(indices.first..=indices.last).contains(&atlas.index)
                    || atlas.index == indices.last
                {
                    indices.first
                } else {
                    atlas.index + 1
                };
            }
        }
    }
}

pub struct AnimationPlugin;
impl Plugin for AnimationPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            FixedUpdate,
            animate_sprite
                .after(update_player)
                .run_if(in_state(GameState::Game)),
        );
    }
}
