use bevy::sprite::Anchor;

use crate::prelude::*;

#[derive(Component)]
pub struct Player {
    fuel: f32,
    jetpack: bool,
    atom_id: u8,
}

impl Default for Player {
    fn default() -> Self {
        Self {
            fuel: FUEL_MAX,
            jetpack: false,
            atom_id: 2,
        }
    }
}

#[derive(Component, Default)]
pub struct Tool;

#[derive(Component)]
pub struct ToolFront;

pub fn player_setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
) {
    let player_actor = Actor {
        height: 17,
        width: 10,
        pos: ivec2(0, 0),
        vel: vec2(0., 0.),
    };

    let player_handle = asset_server.load("player/player_sheet.png");
    let player_atlas =
        TextureAtlas::from_grid(player_handle, Vec2::new(24.0, 24.0), 8, 5, None, None);
    let player_atlas_handle = texture_atlases.add(player_atlas);
    let animation_indices = AnimationIndices { first: 0, last: 1 };
    let player_transform = GlobalTransform::from_xyz(5. * 3., -8. * 3., PLAYER_LAYER);

    let tool_handle = asset_server.load("player/player_tool.png");
    let tool_bundle = SpriteBundle {
        texture: tool_handle,
        sprite: Sprite {
            anchor: Anchor::CenterLeft,
            ..Default::default()
        },
        transform: Transform::from_translation(Vec3::new(-3., -3.5, 0.1)),
        ..Default::default()
    };
    let tool_front_ent = commands
        .spawn((
            TransformBundle::from_transform(Transform::from_translation(vec3(8., 0., 0.))),
            ToolFront,
        ))
        .id();
    let tool_ent = commands
        .spawn(tool_bundle)
        .insert(Tool)
        .insert_children(0, &[tool_front_ent])
        .id();

    commands
        .spawn((
            player_actor,
            Player::default(),
            SpriteSheetBundle {
                texture_atlas: player_atlas_handle,
                sprite: TextureAtlasSprite::new(animation_indices.first),
                global_transform: player_transform,
                ..default()
            },
            animation_indices,
            AnimationTimer(Timer::from_seconds(0.1, TimerMode::Repeating)),
        ))
        .add_child(tool_ent);
}

/// Updates player
pub fn update_player(
    input: (ResMut<Input<KeyCode>>, EventReader<MouseWheel>),
    mut player: Query<(&mut Actor, &mut Player, &mut AnimationIndices)>,
    chunk_manager: ResMut<ChunkManager>,
    mut camera: Query<&mut Transform, (Without<Tool>, With<Camera>)>,
    materials: (Res<Assets<Materials>>, Res<MaterialsHandle>),
) {
    let (mut actor, mut player, mut anim_idxs) = player.single_mut();
    let (keys, mut scroll_evr) = input;
    let mut camera_transform = camera.single_mut();
    let materials = materials.0.get(materials.1 .0.clone()).unwrap();

    // Gravity
    if actor.vel.y < TERM_VEL as f32 {
        actor.vel.y += 1.;
    }

    // Movement
    let x = -(keys.pressed(KeyCode::A) as u8 as f32) + keys.pressed(KeyCode::D) as u8 as f32;
    actor.vel.x = x * RUN_SPEED;

    // Refuel
    let on_ground = on_ground(&chunk_manager, &actor, materials);
    if on_ground {
        player.fuel = (player.fuel + FUEL_REGEN).clamp(0., Player::default().fuel);
    }

    // Jump and Jetpack
    let mut just_jumped = false;
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

    //Zoom
    for ev in scroll_evr.read() {
        if ev.unit == MouseScrollUnit::Line {
            camera_transform.scale *= 0.9_f32.powi(ev.y as i32);
        }
    }

    //Change shooting atoms
    if keys.just_pressed(KeyCode::Key1) {
        player.atom_id = 2;
    } else if keys.just_pressed(KeyCode::Key2) {
        player.atom_id = 3;
    } else if keys.just_pressed(KeyCode::Key3) {
        player.atom_id = 4;
    } else if keys.just_pressed(KeyCode::Key4) {
        player.atom_id = 5;
    }
}

pub fn tool_system(
    mut commands: Commands,
    mut tool: Query<(&mut Transform, &GlobalTransform, &mut Sprite), With<Tool>>,
    mut camera: Query<(&Camera, &GlobalTransform), Without<Tool>>,
    tool_front_ent: Query<Entity, With<ToolFront>>,
    querys: (Query<&Window>, Query<(&mut TextureAtlasSprite, &Player)>),
    resources: (
        ResMut<ChunkManager>,
        ResMut<DirtyRects>,
        Res<Input<MouseButton>>,
    ),
    materials: (Res<Assets<Materials>>, Res<MaterialsHandle>),
) {
    let (mut tool_transform, tool_gtransform, mut tool_sprite) = tool.single_mut();
    let (camera, camera_gtransform) = camera.single_mut();
    let (window, mut player) = querys;
    let (mut textatlas_sprite, player) = player.single_mut();
    let (mut chunk_manager, mut dirty_rects, mouse) = resources;
    let window = window.single();
    let materials = materials.0.get(materials.1 .0.clone()).unwrap();

    if let Some(world_position) = window
        .cursor_position()
        .and_then(|cursor| camera.viewport_to_world(camera_gtransform, cursor))
        .map(|ray| ray.origin.truncate())
    {
        //Rotate and move sprite
        let center_vec = tool_gtransform.compute_transform().translation.xy();
        let tool_vec = world_position - center_vec;
        let angle = tool_vec.y.atan2(tool_vec.x);
        tool_transform.rotation = Quat::from_rotation_z(angle);

        let flip_bool = angle.abs() > std::f32::consts::FRAC_PI_2;
        textatlas_sprite.flip_x = flip_bool;
        tool_sprite.flip_y = flip_bool;
        tool_transform.translation.x =
            tool_transform.translation.x.abs() * (flip_bool as i8 * 2 - 1) as f32;

        //Tool shooting and sucking atoms
        let mut center_vec_y_flipped = center_vec;
        center_vec_y_flipped.y *= -1.;

        let tool_slope = Vec2::new(angle.cos(), -angle.sin());
        let bound_slope = Vec2::new((angle + std::f32::consts::FRAC_PI_2).cos(), -(angle).cos());
        let tool_front = center_vec_y_flipped + tool_slope * 8.;

        let mut pos_to_update = vec![];
        if mouse.pressed(MouseButton::Right) {
            let new_tool_front = tool_front + tool_slope * 2.;
            let n = 6;

            for i in 0..=n {
                let angle = fastrand::f32() * std::f32::consts::TAU;

                let vec = new_tool_front - bound_slope * 2.
                    + bound_slope * 2.5 * i as f32 / n as f32
                    + vec2(angle.cos(), angle.sin());
                let chunk_pos = global_to_chunk(vec.as_ivec2());
                if let (Some(atom), tool_atom) = (
                    chunk_manager.get_mut_atom(chunk_pos),
                    Atom::new(player.atom_id),
                ) {
                    if materials[atom.id].is_void() || materials[atom.id].is_object() {
                        let angle = fastrand::f32() * 0.5 - 0.25;
                        let vel = (tool_slope * 10. * (fastrand::f32() * 0.2 + 0.8))
                            .rotate(vec2(angle.cos(), angle.sin()));
                        commands.spawn(Particle {
                            atom: tool_atom,
                            velocity: vel,
                            pos: vec,
                            ..Default::default()
                        });
                    }
                }
            }
        } else if mouse.pressed(MouseButton::Left) {
            let center_bound = tool_front + tool_slope * TOOL_DISTANCE;

            let bound1 = (center_bound + bound_slope * TOOL_RANGE).as_ivec2();
            let bound2 = (center_bound + -bound_slope * TOOL_RANGE).as_ivec2();

            for bound_vec in Line::new(bound1, bound2 - bound1) {
                for vec in Line::new(tool_front.as_ivec2(), bound_vec - tool_front.as_ivec2()) {
                    let chunk_pos = global_to_chunk(vec);
                    if let Some(atom) = chunk_manager.get_mut_atom(chunk_pos) {
                        if !materials[atom.id].is_void() && !materials[atom.id].is_object() {
                            commands.spawn(Particle {
                                atom: *atom,
                                pos: chunk_pos.to_global().as_vec2(),
                                state: PartState::Follow(tool_front_ent.single()),
                                ..Default::default()
                            });

                            pos_to_update.push(chunk_pos);
                            *atom = Atom::default();
                            break;
                        }
                    }
                }
            }
        }

        for pos in pos_to_update {
            update_dirty_rects_3x3(&mut dirty_rects.current, pos);
            update_dirty_rects(&mut dirty_rects.render, pos);
        }
    }
}

pub fn update_player_sprite(
    mut query: Query<(&mut Transform, &Actor), With<Player>>,
    mut camera_q: Query<&mut Transform, (With<Camera>, Without<Player>)>,
) {
    let (mut transform, actor) = query.single_mut();
    let mut camera_transform = camera_q.single_mut();

    let top_corner_vec = vec3(actor.pos.x as f32, -actor.pos.y as f32, 2.);
    let center_vec = top_corner_vec + vec3(actor.width as f32 / 2., -8., 0.);
    transform.translation = center_vec;
    camera_transform.translation = center_vec;
}

#[derive(Resource, Default)]
pub struct SavingTask(pub Option<Task<()>>);

pub struct PlayerPlugin;
impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                update_player.after(chunk_manager_update),
                tool_system.after(update_player),
                update_player_sprite.after(update_actors),
            ),
        )
        .insert_resource(SavingTask::default())
        .add_systems(PostStartup, player_setup.after(manager_setup));
    }
}
