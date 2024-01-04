use bevy::sprite::Anchor;

use crate::atom::State;
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

#[derive(Component, Default)]
pub struct Tool {
    atoms: Vec<Atom>,
}

pub fn player_setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
    mut chunk_manager: ResMut<ChunkManager>,
) {
    let player_actor = Actor {
        height: 17,
        width: 10,
        pos: ivec2(0, 0),
        vel: vec2(0., 0.),
    };
    add_actor(&mut chunk_manager, &player_actor);

    let player_handle = asset_server.load("player/player_sheet.png");
    let player_atlas =
        TextureAtlas::from_grid(player_handle, Vec2::new(24.0, 24.0), 8, 5, None, None);
    let player_atlas_handle = texture_atlases.add(player_atlas);
    let animation_indices = AnimationIndices { first: 0, last: 1 };
    let mut player_transform = Transform::from_scale(Vec3::splat(3.0));
    player_transform.translation = vec2(5. * 3., -8. * 3.).extend(2.);

    let tool_handle = asset_server.load("player/player_tool.png");
    let tool_bundle = SpriteBundle {
        texture: tool_handle,
        sprite: Sprite {
            anchor: Anchor::CenterLeft,
            ..Default::default()
        },
        transform: Transform::from_translation(Vec3::new(-3., -3.5, 1.)),
        ..Default::default()
    };
    let tool_ent = commands.spawn(tool_bundle).insert(Tool::default()).id();

    commands
        .spawn((
            player_actor,
            Player::default(),
            SpriteSheetBundle {
                texture_atlas: player_atlas_handle,
                sprite: TextureAtlasSprite::new(animation_indices.first),
                transform: player_transform,
                ..default()
            },
            animation_indices,
            AnimationTimer(Timer::from_seconds(0.1, TimerMode::Repeating)),
        ))
        .add_child(tool_ent);
}

/// Updates player
pub fn update_player(
    input: (
        Res<Input<MouseButton>>,
        ResMut<Input<KeyCode>>,
        EventReader<MouseWheel>,
    ),
    window: Query<&Window>,
    mut player: Query<(
        &mut Actor,
        &mut Player,
        &mut TextureAtlasSprite,
        &mut AnimationIndices,
    )>,
    mut tool: Query<(&mut Transform, &GlobalTransform, &mut Sprite, &mut Tool)>,
    mut camera_q: Query<(&Camera, &GlobalTransform, &mut Transform), Without<Tool>>,
    mut chunk_manager: ResMut<ChunkManager>,
    mut dirty_rects: Query<&mut DirtyRects>,
) {
    let (mut actor, mut player, mut textatlas_sprite, mut anim_idxs) = player.single_mut();
    let (mut tool_transform, tool_gtransform, mut tool_sprite, mut tool) = tool.single_mut();
    let (mouse, keys, mut scroll_evr) = input;

    // Gravity
    if actor.vel.y < TERM_VEL as f32 {
        actor.vel.y += 1.;
    }

    // Movement
    let x = -(keys.pressed(KeyCode::A) as u8 as f32) + keys.pressed(KeyCode::D) as u8 as f32;
    actor.vel.x = x * RUN_SPEED;

    // Refuel
    let on_ground = on_ground(&chunk_manager, &actor);
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

    // Tool
    let (camera, camera_gtransform, mut camera_transform) = camera_q.single_mut();
    let window = window.single();
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
        center_vec_y_flipped /= ATOM_SIZE as f32;

        let tool_slope = Vec2::new(angle.cos(), -angle.sin());
        let bound_slope = Vec2::new((angle + std::f32::consts::FRAC_PI_2).cos(), -(angle).cos());
        let tool_front = center_vec_y_flipped + tool_slope * 8.;

        let mut pos_to_update = vec![];
        if mouse.pressed(MouseButton::Right) {
            let new_tool_front = tool_front + tool_slope * 6.;
            for i in 0..3 {
                for vec in Line::new(
                    (new_tool_front - bound_slope * 3. + tool_slope * i as f32 * 2.).as_ivec2(),
                    (bound_slope * 3.).as_ivec2(),
                ) {
                    let chunk_pos = global_to_chunk(vec);
                    if let (Some(atom), Some(mut tool_atom)) =
                        (chunk_manager.get_mut_atom(chunk_pos), tool.atoms.pop())
                    {
                        if atom.state == State::Void {
                            let vel = tool_slope * 10. * (fastrand::f32() * 0.2 + 0.8);

                            tool_atom.velocity = (vel.x as i8, vel.y as i8);
                            chunk_manager[chunk_pos] = tool_atom;

                            pos_to_update.push(chunk_pos);
                        }
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
                        if atom.state != State::Void {
                            tool.atoms.push(*atom);
                            *atom = Atom::new();
                            pos_to_update.push(chunk_pos);
                            break;
                        }
                    }
                }
            }
        }

        let mut dirty_rects = dirty_rects.single_mut();
        for pos in pos_to_update {
            update_dirty_rects_3x3(&mut dirty_rects.current, pos);
            update_dirty_rects(&mut dirty_rects.render, pos);
        }
    }

    for ev in scroll_evr.read() {
        if ev.unit == MouseScrollUnit::Line {
            camera_transform.scale *= 0.9_f32.powi(ev.y as i32);
        }
    }
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

pub fn send_manager_task(
    mut commands: Commands,
    chunk_textures: Query<Entity, With<ChunkTextures>>,
    image_entities: Query<(&Parent, Entity, &Handle<Image>)>,
    player: Query<&Actor, With<Player>>,
    resources: (
        ResMut<SavingTask>,
        ResMut<ChunkManager>,
        ResMut<Assets<Image>>,
    ),
    mut task_executor: AsyncTaskRunner<(HashMap<IVec2, Chunk>, IVec2)>,
) {
    let (mut saving_task, mut chunk_manager, mut images) = resources;

    let mut player_pos = player.single().pos;
    if player_pos.x < 0 {
        player_pos.x -= CHUNK_LENGHT as i32
    }
    if player_pos.y < 0 {
        player_pos.y -= CHUNK_LENGHT as i32
    }
    player_pos /= CHUNK_LENGHT as i32;

    let diff_x = player_pos.x - chunk_manager.pos.x - LOAD_WIDTH / 2;
    let diff_y = player_pos.y - chunk_manager.pos.y - LOAD_HEIGHT / 2;
    let new_diff = ivec2(diff_x, diff_y);

    match task_executor.poll() {
        AsyncTaskStatus::Idle => {
            if let Some(task) = &saving_task.0 {
                if task.is_finished() {
                    saving_task.0 = None;
                } else {
                    return;
                }
            }

            if new_diff != IVec2::ZERO {
                task_executor.start(async move {
                    let file = fs::read("assets/worlds/world").unwrap_or_default();
                    let chunks: HashMap<IVec2, Chunk> =
                        bincode::deserialize(&file).unwrap_or_default();
                    (chunks, new_diff)
                });
            }
        }
        AsyncTaskStatus::Finished((mut file, diff)) => {
            let chunk_textures = chunk_textures.single();
            for _ in 0..diff.x.abs() {
                chunk_manager.move_manager(
                    &mut commands,
                    &mut images,
                    &chunk_textures,
                    &image_entities,
                    &mut file,
                    MoveDir::X(diff.x.signum()),
                );
            }

            for _ in 0..diff.y.abs() {
                chunk_manager.move_manager(
                    &mut commands,
                    &mut images,
                    &chunk_textures,
                    &image_entities,
                    &mut file,
                    MoveDir::Y(diff.y.signum()),
                );
            }

            let pool = AsyncComputeTaskPool::get();
            saving_task.0 = Some(pool.spawn(async move {
                let data = bincode::serialize(&file).unwrap();
                //Save file
                let _ = File::create("assets/worlds/world").unwrap().write(&data).unwrap();
            }));
        }
        AsyncTaskStatus::Pending => {}
    }
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
                update_player_sprite.after(update_actors),
                send_manager_task,
            ),
        )
        .insert_resource(SavingTask::default())
        .add_systems(PostStartup, player_setup.after(manager_setup));
    }
}
