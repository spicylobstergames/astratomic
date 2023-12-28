use crate::prelude::*;

#[derive(Component)]
pub struct Player {
    fuel: f32,
    jetpack: bool,
}

impl Default for Player {
    fn default() -> Self {
        Self {
            fuel: FUEL_MAX,
            jetpack: false,
        }
    }
}

// Player consts
const FUEL_MAX: f32 = 50.;
const FUEL_REGEN: f32 = 1.;
const FUEL_COMSUMPTON: f32 = 0.48;
const JUMP_MAG: f32 = 13.;
const JETPACK_FORCE: f32 = 1.5;
const JETPACK_MAX: f32 = 3.;
const RUN_SPEED: f32 = 5.;

/// Updates player
pub fn update_player(
    keys: ResMut<Input<KeyCode>>,
    mut query: Query<(
        &mut Actor,
        &mut Player,
        &mut TextureAtlasSprite,
        &mut AnimationIndices,
    )>,
    chunk_manager: Query<&ChunkManager>,
) {
    let (mut actor, mut player, mut textatlas_sprite, mut anim_idxs) = query.single_mut();
    let chunk_manager = chunk_manager.single();

    let on_ground = on_ground(chunk_manager, &actor);
    let mut just_jumped = false;

    // Gravity
    if actor.vel.y < TERM_VEL as f32 {
        actor.vel.y += 1.;
    }

    // Movement
    let x = -(keys.pressed(KeyCode::A) as u8 as f32) + keys.pressed(KeyCode::D) as u8 as f32;
    actor.vel.x = x * RUN_SPEED;

    // Refuel
    if on_ground {
        player.fuel = (player.fuel + FUEL_REGEN).clamp(0., Player::default().fuel);
    }

    // Jump and Jetpack
    if keys.just_pressed(KeyCode::Space) {
        if on_ground {
            actor.vel.y -= JUMP_MAG;
            just_jumped = true;
        } else {
            player.jetpack = true;
            actor.vel.y = 0.;
        }
    }

    if player.fuel > 0. && keys.pressed(KeyCode::Space) && player.jetpack {
        actor.vel.y = (actor.vel.y - JETPACK_FORCE).clamp(-JETPACK_MAX, f32::MAX);
        player.fuel -= FUEL_COMSUMPTON;
    } else {
        player.jetpack = false;
    }

    //Animation
    if player.jetpack {
        anim_idxs.first = 24;
        anim_idxs.last = 26;
    } else if !on_ground && player.fuel < FUEL_MAX {
        anim_idxs.first = 32;
        anim_idxs.last = 32;
    } else if just_jumped {
        anim_idxs.first = 16;
        anim_idxs.last = 23;
    } else if on_ground {
        if x.abs() > 0. {
            anim_idxs.first = 8;
            anim_idxs.last = 11;
        } else {
            anim_idxs.first = 0;
            anim_idxs.last = 1;
        }
    }
    if x.abs() > 0. {
        textatlas_sprite.flip_x = x < 0.;
    }

    // Weapon
    let top_corner_vec = vec3(
        actor.pos.x as f32 * ATOM_SIZE as f32,
        -actor.pos.y as f32 * ATOM_SIZE as f32,
        2.,
    );
    //let center_vec = top_corner_vec + vec3(actor.width as f32 / 2., actor.height as f32 / 2., 0.);
}

pub fn update_player_sprite(
    mut query: Query<(&mut Transform, &Actor), With<Player>>,
    mut camera_q: Query<&mut Transform, (With<Camera>, Without<Player>)>,
) {
    let (mut transform, actor) = query.single_mut();
    let mut camera_transform = camera_q.single_mut();

    let top_corner_vec = vec3(
        actor.pos.x as f32 * ATOM_SIZE as f32,
        -actor.pos.y as f32 * ATOM_SIZE as f32,
        2.,
    );
    let center_vec = top_corner_vec
        + vec3(
            actor.width as f32 / 2. * ATOM_SIZE as f32,
            -(8. * ATOM_SIZE as f32),
            0.,
        );
    transform.translation = center_vec;
    camera_transform.translation = center_vec;
}

pub struct PlayerPlugin;
impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                update_player.before(update_actors),
                update_player_sprite.after(update_actors),
            ),
        );
    }
}
