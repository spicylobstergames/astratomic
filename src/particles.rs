use crate::prelude::*;

//To spawn a particle just add a entity with this component
#[derive(Component, Debug, Default)]
pub struct Particle {
    pub atom: Atom,
    pub velocity: Vec2,
    //Used when initiating particle
    pub pos: Vec2,
    pub state: PartState,
}

/// Particle State
#[derive(Default, Debug, PartialEq)]
pub enum PartState {
    //Used when the particle is looking for a place to put itself
    Looking,
    #[default]
    Normal,
    //Used when following a entity transform
    Follow(Entity),
}

#[derive(Component)]
pub struct Hydrated;

pub fn hydrate_particles(
    mut commands: Commands,
    particles: Query<(&Particle, Entity), Without<Hydrated>>,
) {
    //Spawn particle sprite
    for (particle, ent) in particles.iter() {
        let mult = if particle.state == PartState::Normal {
            1.2
        } else {
            0.8
        };
        commands
            .entity(ent)
            .insert(SpriteBundle {
                sprite: Sprite {
                    color: Color::rgba(
                        particle.atom.color[0] as f32 / 255. * mult,
                        particle.atom.color[1] as f32 / 255. * mult,
                        particle.atom.color[2] as f32 / 255. * mult,
                        particle.atom.color[3] as f32 / 255.,
                    ),
                    custom_size: Some(Vec2::new(1.0, 1.0)),
                    ..default()
                },
                transform: Transform::from_translation(Vec3::new(
                    particle.pos.x,
                    -particle.pos.y,
                    PARTICLE_LAYER,
                )),
                ..default()
            })
            .insert(Hydrated);
    }
}

pub fn update_particles(
    mut commands: Commands,
    mut particles: Query<(&mut Particle, &mut Transform, Entity), With<Hydrated>>,
    entities: Query<&GlobalTransform, Without<Particle>>,
    mut chunk_manager: ResMut<ChunkManager>,
    mut dirty_rects: ResMut<DirtyRects>,
    materials: (Res<Assets<Materials>>, Res<MaterialsHandle>),
) {
    let materials = materials.0.get(materials.1 .0.clone()).unwrap();

    let compute_pool = ComputeTaskPool::get();

    compute_pool.scope(|deferred_scope| {
        let manager_pos = &chunk_manager.pos.clone();
        let chunk_manager = Arc::new(RwLock::new(&mut chunk_manager));

        let (particles_send, particles_recv) = async_channel::unbounded::<DeferredParticleUpdate>();
        let particle_send = &particles_send;

        let chunk_manager_deffered = Arc::clone(&chunk_manager);
        deferred_scope.spawn(async move {
            while let Ok(update) = particles_recv.recv().await {
                if let Some((chunk_pos, update_atom)) = update.remove {
                    let mut change_atom = false;
                    if let Some(atom) = chunk_manager_deffered.read().unwrap().get_atom(&chunk_pos)
                    {
                        if materials[atom.id].is_void() {
                            change_atom = true;
                        }
                    }

                    if change_atom {
                        let atom = &mut chunk_manager_deffered.write().unwrap()[chunk_pos];
                        *atom = update_atom;
                        commands.entity(update.ent).despawn();

                        update_dirty_rects(&mut dirty_rects.render, chunk_pos);
                        update_dirty_rects_3x3(&mut dirty_rects.current, chunk_pos);
                    }
                } else {
                    commands.entity(update.ent).despawn();
                    continue;
                }
            }
        });

        particles
            .par_iter_mut()
            .for_each(|(mut particle, mut transform, ent)| {
                let mut dest_pos = transform.translation.xy();
                dest_pos.y *= -1.;
                dest_pos += particle.velocity;

                let mut cur_pos = transform.translation.xy();
                cur_pos.y *= -1.;

                let x_bound = (manager_pos.x * CHUNK_LENGHT as i32)
                    ..((manager_pos.x + LOAD_WIDTH) * CHUNK_LENGHT as i32);
                let y_bound = (manager_pos.y * CHUNK_LENGHT as i32)
                    ..((manager_pos.y + LOAD_HEIGHT) * CHUNK_LENGHT as i32);
                //If not on bounds, remove particle
                if !x_bound.contains(&(dest_pos.x as i32))
                    || !y_bound.contains(&(dest_pos.y as i32))
                    || !x_bound.contains(&(cur_pos.x as i32))
                    || !y_bound.contains(&(cur_pos.y as i32))
                {
                    particle_send
                        .try_send(DeferredParticleUpdate { remove: None, ent })
                        .unwrap();
                } else {
                    match particle.state {
                        PartState::Looking | PartState::Normal => {
                            if particle.state == PartState::Normal {
                                particle.velocity += GRAVITY as f32 * Vec2::Y;
                            }

                            let mut prev_pos = cur_pos.as_ivec2();
                            for pos in
                                Line::new(cur_pos.as_ivec2(), (dest_pos - cur_pos).as_ivec2())
                            {
                                let chunk_pos = global_to_chunk(pos);
                                let prev_chunk_pos = global_to_chunk(prev_pos);

                                let atom =
                                    *chunk_manager.read().unwrap().get_atom(&chunk_pos).unwrap();
                                let prev_atom = *chunk_manager
                                    .read()
                                    .unwrap()
                                    .get_atom(&prev_chunk_pos)
                                    .unwrap();

                                if particle.state == PartState::Normal
                                    && !materials[atom.id].is_void()
                                {
                                    //Hit something!
                                    //If our previous pos is free
                                    if materials[prev_atom.id].is_void() {
                                        particle_send
                                            .try_send(DeferredParticleUpdate {
                                                remove: Some((
                                                    global_to_chunk(prev_pos),
                                                    particle.atom,
                                                )),
                                                ent,
                                            })
                                            .unwrap();
                                    } else if particle.state != PartState::Looking {
                                        //Upward warp if can't find a place to put
                                        particle.velocity.y = -2.;
                                        particle.velocity.x = 0.;
                                        particle.state = PartState::Looking;
                                    }

                                    break;
                                } else if particle.state == PartState::Looking
                                    && materials[prev_atom.id].is_void()
                                {
                                    particle_send
                                        .try_send(DeferredParticleUpdate {
                                            remove: Some((global_to_chunk(pos), particle.atom)),
                                            ent,
                                        })
                                        .unwrap();
                                    break;
                                }

                                prev_pos = pos;
                            }
                            transform.translation.x = dest_pos.x;
                            transform.translation.y = -dest_pos.y;
                        }
                        PartState::Follow(follow_ent) => {
                            let follow_transform = entities.get(follow_ent).unwrap();
                            let mut follow_pos =
                                follow_transform.compute_transform().translation.xy();
                            follow_pos.y *= -1.;

                            let mag = (particle.velocity.length()).clamp(0., 6.);
                            let angle = follow_pos - cur_pos;
                            let angle = angle.y.atan2(angle.x);

                            particle.velocity = vec2(angle.cos(), angle.sin()) * (mag + 0.5);

                            let mut part_vel = particle.velocity;
                            part_vel.y *= -1.;
                            transform.translation += part_vel.extend(0.);

                            if transform
                                .translation
                                .xy()
                                .distance(follow_transform.compute_transform().translation.xy())
                                < 3.
                            {
                                particle_send
                                    .try_send(DeferredParticleUpdate { remove: None, ent })
                                    .unwrap();
                            }
                        }
                    }
                }
            });
    });
}

pub struct DeferredParticleUpdate {
    pub ent: Entity,
    pub remove: Option<(ChunkPos, Atom)>,
}

pub struct ParticlesPlugin;
impl Plugin for ParticlesPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            FixedUpdate,
            (
                hydrate_particles.after(update_player),
                update_particles.before(chunk_manager_update),
            )
                .run_if(in_state(GameState::Game)),
        );
    }
}
