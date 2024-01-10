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
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
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
    let player_atlas =
        TextureAtlas::from_grid(player_handle, Vec2::new(24.0, 24.0), 8, 5, None, None);
    let player_atlas_handle = texture_atlases.add(player_atlas);
    let animation_indices = AnimationIndices { first: 0, last: 1 };
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
    time: Res<Time>,
) {
    let (mut actor, mut player, mut anim_idxs) = player.single_mut();
    let (keys, mut scroll_evr) = input;
    let mut camera_transform = camera.single_mut();
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
    let x = -(keys.pressed(KeyCode::A) as u8 as f32) + keys.pressed(KeyCode::D) as u8 as f32;
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
    if keys.just_pressed(KeyCode::Space) {
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
        if keys.pressed(KeyCode::Space)
            && time.elapsed_seconds_wrapped_f64() - jump_start < TIME_JUMP_PRESSED
        {
            actor.vel.y -= PRESSED_JUMP_MAG
        }
    }

    // Jetpack
    let mut new_up = false;
    if let PlayerState::Jetpack(_) = player.state {
        if player.fuel > 0. && keys.pressed(KeyCode::Space) {
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

/// A resource for the state of the in-game smooth camera.
#[derive(Resource)]
pub struct TrackingCamera {
    /// The position in world space of the origin of this camera.
    pub position: Vec2,

    /// The current target of the camera; what it smoothly focuses on.
    pub target: Vec2,

    /// The half-size of the rectangle around the center of the screen where
    /// the player can move without the camera retargeting. When the player
    /// leaves this rectangle, the camera will retarget to include the player
    /// back into this region of the screen.
    pub tracking_size: Vec2,

    /// The half-size of the rectangle around the center of the screen where
    /// the camera will smoothly interpolate. If the player leaves this region,
    /// the camera will clamp to keep the player within it.
    pub clamp_size: Vec2,

    /// A dead distance from the edge of the tracking region to the player
    /// where the camera will not perform any tracking, even if the player is
    /// minutely outside of the tracking region. This is provided so that the
    /// camera can recenter even if the player has not moved since a track.
    pub dead_zone: Vec2,

    /// The proportion (between 0.0-1.0) that the camera reaches its target
    /// from its initial position during a second's time.
    pub speed: f64,

    /// A timeout to recenter the camera on the player even if the player has
    /// not left the tracking rectangle.
    pub recenter_timeout: f32,

    /// The duration in seconds since the player has left the tracking rectangle.
    ///
    /// When this duration reaches `recenter_timeout`, the player will be
    /// recentered.
    pub last_track: f32,
}

impl Default for TrackingCamera {
    fn default() -> Self {
        Self {
            position: Vec2::ZERO,
            target: Vec2::ZERO,
            tracking_size: vec2(32.0, 16.0),
            clamp_size: vec2(96.0, 64.0),
            dead_zone: Vec2::splat(0.1),
            speed: 0.98,
            recenter_timeout: 3.0,
            last_track: 0.0,
        }
    }
}

impl TrackingCamera {
    /// Update the camera with the current position and this frame's delta time.
    pub fn update(&mut self, player_pos: Vec2, dt: f64) {
        // update target with player position
        self.track_player(player_pos);

        // track time since last time we had to track the player
        let new_last_track = self.last_track + dt as f32;

        // test if we've triggered a recenter
        if self.last_track < self.recenter_timeout && new_last_track > self.recenter_timeout {
            // target the player
            self.target = player_pos;
        }

        // update the duration since last track
        self.last_track = new_last_track;

        // lerp the current position towards the target
        // correct lerp degree using delta time
        // perform pow() with high precision
        let lerp = 1.0 - (1.0 - self.speed).powf(dt) as f32;
        self.position = self.position.lerp(self.target, lerp);
    }

    /// Helper function to clamp a rectangle (given as a half-size at the
    /// origin) so that a point lays within it. Returns an offset to apply to
    /// the rectangle, if any was required.
    pub fn clamp_rect(half_size: Vec2, point: Vec2) -> Option<Vec2> {
        let mut ox = None;
        let mut oy = None;

        if point.x > half_size.x {
            ox = Some(point.x - half_size.x);
        } else if point.x < -half_size.x {
            ox = Some(point.x + half_size.x);
        }

        if point.y > half_size.y {
            oy = Some(point.y - half_size.y);
        } else if point.y < -half_size.y {
            oy = Some(point.y + half_size.y);
        }

        if let (None, None) = (ox, oy) {
            None
        } else {
            Some(vec2(ox.unwrap_or(0.0), oy.unwrap_or(0.0)))
        }
    }

    pub fn track_player(&mut self, player_pos: Vec2) {
        // get current relative position to player
        let rel_pos = player_pos - self.position;

        // track the player and reset last track if change was necessary
        if let Some(offset) = Self::clamp_rect(self.tracking_size, rel_pos) {
            // skip tracking if it falls within the dead zone
            if !self.dead_zone.cmpgt(offset.abs()).all() {
                self.target = self.position + offset;
                self.last_track = 0.0;
            }
        }

        // clamp the player within the screen
        if let Some(offset) = Self::clamp_rect(self.clamp_size, rel_pos) {
            self.position += offset;
        }
    }
}

pub fn update_camera(
    query: Query<&Transform, With<Player>>,
    mut camera_q: Query<&mut Transform, (With<Camera>, Without<Player>)>,
    mut tracking: ResMut<TrackingCamera>,
    time: Res<Time>,
) {
    let transform = query.single();
    let mut camera_transform = camera_q.single_mut();
    let dt = time.delta_seconds_f64();
    tracking.update(transform.translation.xy(), dt);
    camera_transform.translation = tracking.position.extend(2.0);
}

pub fn update_player_sprite(mut query: Query<(&mut Transform, &Actor), With<Player>>) {
    let (mut transform, actor) = query.single_mut();
    let top_corner_vec = vec3(actor.pos.x as f32, -actor.pos.y as f32, 2.);
    let center_vec = top_corner_vec + vec3(actor.width as f32 / 2., -8., 0.);
    transform.translation = center_vec;
}

#[derive(Resource, Default)]
pub struct SavingTask(pub Option<Task<()>>);

pub struct PlayerPlugin;
impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            FixedUpdate,
            (
                update_player.after(chunk_manager_update),
                update_player_sprite,
                update_camera,
                tool_system.after(chunk_manager_update),
            ),
        )
        .insert_resource(SavingTask::default())
        .insert_resource(TrackingCamera::default())
        .add_systems(PostStartup, player_setup.after(manager_setup));
    }
}
