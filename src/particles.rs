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
                //TODO figure out layers
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
) {
    let compute_pool = ComputeTaskPool::get();

    compute_pool.scope(|deferred_scope| {
        let chunks = &chunk_manager.chunks.clone();
        let manager_pos = &chunk_manager.pos.clone();

        let (particles_send, particles_recv) = async_channel::unbounded::<DeferredParticleUpdate>();
        let particle_send = &particles_send;

        deferred_scope.spawn(async move {
            while let Ok(update) = particles_recv.recv().await {
                if update.remove {
                    commands.entity(update.ent).despawn();
                    continue;
                }

                if let Some(atom) = chunk_manager.get_mut_atom(update.chunk_pos) {
                    if atom.state == State::Void {
                        *atom = update.atom;
                        commands.entity(update.ent).despawn();

                        update_dirty_rects(&mut dirty_rects.render, update.chunk_pos);
                        update_dirty_rects_3x3(&mut dirty_rects.current, update.chunk_pos);
                    }
                }
            }
        });

        particles
            .iter_mut()
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
                        .try_send(DeferredParticleUpdate {
                            remove: true,
                            atom: Atom::default(),
                            chunk_pos: ChunkPos::default(),
                            ent,
                        })
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

                                let atom = chunks.get(&chunk_pos.chunk).unwrap().atoms
                                    [chunk_pos.atom.d1()];
                                let prev_atom = chunks.get(&prev_chunk_pos.chunk).unwrap().atoms
                                    [prev_chunk_pos.atom.d1()];

                                if particle.state == PartState::Normal
                                    && atom.state != State::Void
                                    && atom.state != State::Object
                                {
                                    //Hit something!
                                    //If our previous pos is free
                                    if prev_atom.state == State::Void
                                        || prev_atom.state == State::Object
                                    {
                                        particle_send
                                            .try_send(DeferredParticleUpdate {
                                                chunk_pos: global_to_chunk(prev_pos),
                                                atom: particle.atom,
                                                ent,
                                                remove: false,
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
                                    && atom.state == State::Void
                                    || atom.state == State::Object
                                {
                                    particle_send
                                        .try_send(DeferredParticleUpdate {
                                            chunk_pos: global_to_chunk(pos),
                                            atom: particle.atom,
                                            ent,
                                            remove: false,
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
                                    .try_send(DeferredParticleUpdate {
                                        remove: true,
                                        atom: Atom::default(),
                                        chunk_pos: ChunkPos::default(),
                                        ent,
                                    })
                                    .unwrap();
                            }
                        }
                    }
                }
            });
    });
}

pub struct DeferredParticleUpdate {
    pub chunk_pos: ChunkPos,
    pub atom: Atom,
    pub ent: Entity,
    pub remove: bool,
}

pub struct ParticlesPlugin;
impl Plugin for ParticlesPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                hydrate_particles.before(update_particles),
                update_particles.after(chunk_manager_update),
            ),
        );
    }
}
