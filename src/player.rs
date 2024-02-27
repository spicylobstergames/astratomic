use bevy::sprite::Anchor;

use crate::prelude::*;

#[derive(Component)]
pub struct Player {
    fuel: f32,
    state: PlayerState,
    atom_id: u8,
}

impl Default for Player {
    fn default() -> Self {
        Self {
            fuel: FUEL_MAX,
            state: PlayerState::default(),
            atom_id: 2,
        }
    }
}

impl Drop for Actor {
    fn drop(&mut self) {
        let file = File::create("assets/world/player").unwrap();
        let mut buffered = BufWriter::new(file);
        bincode::serialize_into(&mut buffered, &self.pos).unwrap();
    }
}

#[derive(Default)]
pub enum PlayerState {
    #[default]
    Idle,
    Walking,
    Jumping(f64),
    Jetpack(bool),
}

#[derive(Component, Default)]
pub struct Tool;

#[derive(Component)]
pub struct ToolFront;

pub fn player_setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut texture_atlases: ResMut<Assets<TextureAtlasLayout>>,
) {
    let pos: IVec2;
    if let Ok(file) = File::open("assets/world/player") {
        let mut buffered = BufReader::new(file);
        pos = bincode::deserialize_from(&mut buffered).unwrap();
    } else {
        pos = IVec2::default();
        let file = File::create("assets/world/player").unwrap();
        let mut buffered = BufWriter::new(file);
        bincode::serialize_into(&mut buffered, &pos).unwrap();
    }

    let player_actor = Actor {
        height: 17,
        width: 10,
        pos,
        vel: vec2(0., 0.),
    };

    let player_handle = asset_server.load("player/player_sheet.png");
    let animation_indices = AnimationIndices { first: 0, last: 1 };
    let player_atlas_layout =
        TextureAtlasLayout::from_grid(Vec2::new(24.0, 24.0), 8, 5, None, None);
    let atlas = TextureAtlas {
        index: animation_indices.first,
        layout: texture_atlases.add(player_atlas_layout),
    };
    let player_transform = GlobalTransform::from_xyz(5. * 3., -8. * 3., PLAYER_LAYER);

    let tool_handle = asset_server.load("player/player_tool.png");
    let tool_bundle = SpriteBundle {
        texture: tool_handle,
        sprite: Sprite {
            anchor: Anchor::Custom(vec2(-0.1, 0.)),
            ..Default::default()
        },
        transform: Transform::from_translation(Vec3::new(-1., -3.5, 0.1)),
        ..Default::default()
    };
    let tool_front_ent = commands
        .spawn((
            TransformBundle::from_transform(Transform::from_translation(vec3(5., 0., 0.))),
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
            player_actor.clone(),
            Player::default(),
            SpriteSheetBundle {
                atlas,
                global_transform: player_transform,
                texture: player_handle,
                ..default()
            },
            animation_indices,
            AnimationTimer(Timer::from_seconds(0.1, TimerMode::Repeating)),
            bevy_rapier2d::prelude::RigidBody::Fixed,
            bevy_rapier2d::prelude::LockedAxes::ROTATION_LOCKED,
            bevy_rapier2d::prelude::Collider::cuboid(
                player_actor.width as f32 / 2.,
                player_actor.height as f32 / 2.,
            ),
        ))
        .add_child(tool_ent);
}

/// Updates player
pub fn update_player(
    input: (Res<Inputs>, EventReader<MouseWheel>),
    mut player: Query<(&mut Actor, &mut Player, &mut AnimationIndices)>,
    chunk_manager: ResMut<ChunkManager>,
    materials: (Res<Assets<Materials>>, Res<MaterialsHandle>),
    time: Res<Time>,
    mut zoom: ResMut<Zoom>,
) {
    let (mut actor, mut player, mut anim_idxs) = player.single_mut();
    let (inputs, mut scroll_evr) = input;
    let materials = materials.0.get(materials.1 .0.clone()).unwrap();

    // Gravity
    if actor.vel.y < TERM_VEL as f32 {
        actor.vel.y += 1.
            * if matches!(player.state, PlayerState::Jetpack { .. }) {
                0.4
            } else {
                1.
            };
    }

    // Movement
    let x = inputs.right - inputs.left;
    actor.vel.x = x * RUN_SPEED;

    let on_ground = on_ground(&chunk_manager, &actor, materials);

    // Refuel
    if on_ground {
        player.fuel = (player.fuel + FUEL_REGEN).clamp(0., Player::default().fuel);
    }

    if on_ground {
        if x.abs() > 0. {
            player.state = PlayerState::Walking
        } else {
            player.state = PlayerState::Idle
        }
    }

    // Jump
    if inputs.jump_just_pressed {
        if on_ground {
            actor.vel.y -= JUMP_MAG;
            player.state = PlayerState::Jumping(time.elapsed_seconds_wrapped_f64());
        } else {
            player.state = PlayerState::Jetpack(true);
            actor.vel.y = 0.;
        }
    }

    //Jump higher when holding space
    if let PlayerState::Jumping(jump_start) = player.state {
        if inputs.jump_pressed
            && time.elapsed_seconds_wrapped_f64() - jump_start < TIME_JUMP_PRESSED
        {
            actor.vel.y -= PRESSED_JUMP_MAG
        }
    }

    // Jetpack
    let mut new_up = false;
    if let PlayerState::Jetpack(_) = player.state {
        if player.fuel > 0. && inputs.jump_pressed {
            actor.vel.y = (actor.vel.y - JETPACK_FORCE).clamp(-JETPACK_MAX, f32::MAX);
            player.fuel -= FUEL_COMSUMPTON;
            new_up = true;
        } else {
            new_up = false;
        }
    }

    if let PlayerState::Jetpack(up) = &mut player.state {
        *up = new_up
    };

    //Animation
    (anim_idxs.first, anim_idxs.last) = match player.state {
        PlayerState::Idle => (0, 1),
        PlayerState::Walking => (8, 11),
        PlayerState::Jumping { .. } => (16, 23),
        PlayerState::Jetpack(up) => {
            if up {
                (24, 26)
            } else {
                (32, 32)
            }
        }
    };

    //Zoom
    for ev in scroll_evr.read() {
        if ev.unit == MouseScrollUnit::Line {
            zoom.0 *= 0.9_f32.powi(ev.y as i32);
            zoom.0 = zoom.0.clamp(ZOOM_LOWER_BOUND, ZOOM_UPPER_BOUND);
        }
    }

    //Change shooting atoms
    if inputs.numbers[0] {
        player.atom_id = 2;
    } else if inputs.numbers[1] {
        player.atom_id = 3;
    } else if inputs.numbers[2] {
        player.atom_id = 4;
    } else if inputs.numbers[3] {
        player.atom_id = 5;
    }
}

pub fn tool_system(
    mut commands: Commands,
    mut tool: Query<(&mut Transform, &GlobalTransform, &mut Sprite), With<Tool>>,
    mut camera: Query<(&Camera, &GlobalTransform), Without<Tool>>,
    tool_front_ent: Query<Entity, With<ToolFront>>,
    querys: (Query<&Window>, Query<(&mut Sprite, &Player), Without<Tool>>),
    resources: (ResMut<ChunkManager>, ResMut<DirtyRects>, Res<Inputs>),
    materials: (Res<Assets<Materials>>, Res<MaterialsHandle>),
) {
    let (mut tool_transform, tool_gtransform, mut tool_sprite) = tool.single_mut();
    let (camera, camera_gtransform) = camera.single_mut();
    let (window, mut player) = querys;
    let (mut textatlas_sprite, player) = player.single_mut();
    let (mut chunk_manager, mut dirty_rects, inputs) = resources;
    let Ok(window) = window.get_single() else {
        return;
    };
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

        //Tool pulling and pushing atoms
        let mut center_vec_y_flipped = center_vec;
        center_vec_y_flipped.y *= -1.;

        let tool_slope = Vec2::new(angle.cos(), -angle.sin());
        let bound_slope = Vec2::new((angle + std::f32::consts::FRAC_PI_2).cos(), -(angle).cos());
        let tool_front = center_vec_y_flipped + tool_slope * 5.;

        let mut pos_to_update = vec![];
        if inputs.push {
            let new_tool_front = tool_front + tool_slope * 3.5;
            let n = 6;

            for i in 0..=n {
                let rand_angle = fastrand::f32() * std::f32::consts::TAU;

                let mut vec = new_tool_front - bound_slope * 2.
                    + bound_slope * 2.5 * i as f32 / n as f32
                    + vec2(rand_angle.cos(), rand_angle.sin());

                vec += tool_slope * 7. * angle.sin().max(0.);

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
        } else if inputs.pull {
            let center_bound = tool_front + tool_slope * TOOL_DISTANCE;

            let bound1 = (center_bound + bound_slope * TOOL_RANGE).as_ivec2();
            let bound2 = (center_bound + -bound_slope * TOOL_RANGE).as_ivec2();

            for bound_vec in Line::new(bound1, bound2 - bound1) {
                for vec in Line::new(
                    (tool_front - 4. * tool_slope).as_ivec2(),
                    bound_vec - (tool_front - 4. * tool_slope).as_ivec2(),
                ) {
                    let chunk_pos = global_to_chunk(vec);
                    if (vec.distance_squared((tool_front - 6. * tool_slope).as_ivec2()) as f32)
                        .sqrt()
                        < 6.
                    {
                        continue;
                    }

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

        let mut chunks = HashSet::new();
        for pos in pos_to_update {
            update_dirty_rects_3x3(&mut dirty_rects.current, pos);
            update_dirty_rects(&mut dirty_rects.render, pos);
            chunks.insert(pos.chunk);
        }

        for chunk in chunks {
            let chunk = chunk_manager.chunks.get(&chunk).unwrap();
            commands.entity(chunk.entity.unwrap()).remove::<Collider>();
        }
    }
}

pub fn update_player_sprite(mut query: Query<(&mut Transform, &Actor), With<Player>>) {
    let (mut transform, actor) = query.single_mut();
    let top_corner_vec = vec3(actor.pos.x as f32, -actor.pos.y as f32, 2.);
    let center_vec = top_corner_vec + vec3(actor.width as f32 / 2., -8., 0.);
    transform.translation = center_vec;
}

#[derive(Resource, Default)]
pub struct SavingTask(pub Option<Task<()>>);

pub fn get_input(
    keys: Res<ButtonInput<KeyCode>>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mut inputs: ResMut<Inputs>,
) {
    //Jump
    if keys.just_pressed(KeyCode::Space) {
        inputs.jump_just_pressed = true;
        inputs.jump_pressed = true;
    } else if keys.pressed(KeyCode::Space) {
        inputs.jump_pressed = true;
    }

    //Movement
    if keys.pressed(KeyCode::KeyA) {
        inputs.left = 1.;
    }
    if keys.pressed(KeyCode::KeyD) {
        inputs.right = 1.;
    }

    //Tool
    if mouse_buttons.pressed(MouseButton::Left) {
        inputs.pull = true;
    }
    if mouse_buttons.pressed(MouseButton::Right) {
        inputs.push = true;
    }

    //Numbers, to select atoms
    if keys.just_pressed(KeyCode::Digit1) {
        inputs.numbers[0] = true;
    } else if keys.just_pressed(KeyCode::Digit2) {
        inputs.numbers[1] = true;
    } else if keys.just_pressed(KeyCode::Digit3) {
        inputs.numbers[2] = true;
    } else if keys.just_pressed(KeyCode::Digit4) {
        inputs.numbers[3] = true;
    }
}

pub fn clear_input(mut inputs: ResMut<Inputs>) {
    *inputs = Inputs::default();
}

#[derive(Resource, Default)]
pub struct Inputs {
    left: f32,
    right: f32,

    pull: bool,
    push: bool,

    jump_pressed: bool,
    jump_just_pressed: bool,

    numbers: [bool; 4],
}

pub struct PlayerPlugin;
impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            FixedUpdate,
            (
                update_player.before(update_actors),
                update_player_sprite.after(update_actors),
                tool_system
                    .before(chunk_manager_update)
                    .before(update_particles),
                clear_input.after(update_player).after(tool_system),
            )
                .run_if(in_state(GameState::Game)),
        )
        .add_systems(PreUpdate, get_input.run_if(in_state(GameState::Game)))
        .init_resource::<SavingTask>()
        .init_resource::<Inputs>()
        .add_systems(OnEnter(GameState::Game), player_setup.after(manager_setup));
    }
}
