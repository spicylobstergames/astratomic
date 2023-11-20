use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

use bevy::math::{ivec2, vec3};
use bevy::sprite::Anchor;
use bevy::tasks::ComputeTaskPool;

use crate::prelude::*;

/// Updates and do the chunks logic
#[derive(Component)]
pub struct ChunkManager {
    pub chunks: Vec<Chunk>,
    pub textures_hmap: HashMap<AssetId<Image>, usize>,
    pub dt: f32,
}

#[derive(Component)]
pub struct UpdateTextures(Option<TexturesHash>);

#[derive(Component)]
pub struct DirtyRects {
    pub current: Vec<Option<Rect>>,
    pub new: Vec<Option<Rect>>,
}

impl DirtyRects {
    pub fn swap(&mut self) {
        std::mem::swap(&mut self.current, &mut self.new)
    }
}

fn manager_setup(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let side_length = (CHUNK_LENGHT * ATOM_SIZE) as f32;
    let (width, height) = (CHUNKS_WIDTH, CHUNKS_HEIGHT);

    let mut images_vec = vec![];
    let mut chunks = vec![];
    let mut textures_hmap = HashMap::new();
    for y in 0..height {
        for x in 0..width {
            let pos = Vec2::new(x as f32 * side_length, -(y as f32) * side_length);
            let index = y * CHUNKS_WIDTH + x;

            //Get and spawn texture/chunk image
            let texture = images.add(Chunk::new_image());
            images_vec.push(
                commands
                    .spawn(SpriteBundle {
                        texture: texture.clone(),
                        sprite: Sprite {
                            anchor: Anchor::TopLeft,
                            ..Default::default()
                        },
                        transform: Transform::from_xyz(pos.x, pos.y, 0.).with_scale(vec3(
                            ATOM_SIZE as f32,
                            ATOM_SIZE as f32,
                            1.,
                        )),
                        ..Default::default()
                    })
                    .id(),
            );

            //Add texture to chunks manager HashMap
            textures_hmap.insert(texture.id(), index);

            //Create chunk
            let chunk = Chunk::new(texture, index);

            //Update chunk image
            let image = images.get_mut(&chunk.texture).unwrap();
            chunk.update_all(image);

            chunks.push(chunk);
        }
    }

    commands
        .spawn((
            Name::new("Chunks textures"),
            VisibilityBundle::default(),
            TransformBundle::default(),
        ))
        .push_children(&images_vec);

    let chunk_manager = ChunkManager {
        chunks,
        dt: 0.,
        textures_hmap,
    };

    commands.spawn(DirtyRects {
        current: vec![None; CHUNKS_WIDTH * CHUNKS_HEIGHT],
        new: vec![None; CHUNKS_WIDTH * CHUNKS_HEIGHT],
    });
    commands.spawn(chunk_manager);
    commands.spawn(UpdateTextures(None));
}

pub fn chunk_manager_update(
    mut chunk_manager: Query<&mut ChunkManager>,
    mut dirty_rects: Query<&mut DirtyRects>,
    time: Res<Time>,
    actors: Query<(&Actor, &Transform)>,
    mut uptextures_query: Query<&mut UpdateTextures>,
) {
    let mut chunk_manager = chunk_manager.single_mut();

    chunk_manager.dt += time.delta_seconds();
    let dt = chunk_manager.dt;

    let textures_update: ParTexturesHash = Arc::new(Mutex::new(HashMap::new()));

    // Get dirty rects
    let mut dirty_rects_resource = dirty_rects.single_mut();
    let DirtyRects {
        current: dirty_rects,
        new: new_dirty_rects,
    } = &mut *dirty_rects_resource;

    // Get actors
    let mut actors_vec = vec![];
    for (actor, transform) in actors.iter() {
        actors_vec.push((*actor, *transform))
    }

    let row_range = 0..CHUNKS_WIDTH as i32;
    let column_range = 0..CHUNKS_HEIGHT as i32;

    let compute_pool = ComputeTaskPool::get();

    let (deferred_updates_send, deferred_updates_recv) = async_channel::unbounded();
    let deferred_updates_send = &deferred_updates_send;

    // Create a scope in which we handle deferred updates and update chunks.
    compute_pool.scope(|deferred_scope| {
        // Spawn a task on the deferred scope for handling the deferred updates.
        deferred_scope.spawn(async move {
            // Clear the new dirty rects so we can update a fresh list
            new_dirty_rects.iter_mut().for_each(|x| *x = None);

            // Loop through deferred tasks
            while let Ok(update) = deferred_updates_recv.recv().await {
                match update {
                    DeferredUpdate::UpdateDirtyRect { chunk_idx, pos } => {
                        let rect = &mut new_dirty_rects[chunk_idx];
                        if let Some(rect) = rect.as_mut() {
                            extend_rect_if_needed(rect, &pos);
                        } else {
                            *rect = Some(Rect::new(
                                (pos.x - 1.).clamp(0., 63.),
                                (pos.y - 1.).clamp(0., 63.),
                                (pos.x + 1.).clamp(0., 63.),
                                (pos.y + 1.).clamp(0., 63.),
                            ));
                        }
                    } // TODO: Parallelize texture update on GPU.
                      //DeferredUpdate::UpdateImage { .. } => todo!(),
                }
            }
        });

        // Run the 4 update steps in checker like pattern
        for y_thread_off in rand_range(0..2) {
            for x_thread_off in rand_range(0..2) {
                compute_pool.scope(|scope| {
                    let mut mutable_references = MutableReferences::default();
                    get_mutable_references(
                        &mut chunk_manager.chunks,
                        &mut mutable_references,
                        (x_thread_off, y_thread_off),
                    );

                    //Acess chunks
                    let y_iter = (y_thread_off..CHUNKS_HEIGHT).step_by(2);
                    y_iter.for_each(|y| {
                        let x_iter = (x_thread_off..CHUNKS_WIDTH).step_by(2);
                        x_iter.for_each(|x| {
                            if let Some(rect) = dirty_rects[y * CHUNKS_WIDTH + x] {
                                let center_index = y * CHUNKS_WIDTH + x;
                                let mut chunk_group = ChunkGroup::new(
                                    mutable_references.centers[y / 2 * CHUNKS_WIDTH / 2 + x / 2]
                                        .take()
                                        .unwrap(),
                                    center_index,
                                );

                                // Get all 3x3 chunks for each chunk updating
                                for y_off in -1..=1 {
                                    for x_off in -1..=1 {
                                        if (x_off == 0 && y_off == 0)
                                            || !column_range.contains(&(y as i32 + y_off))
                                            || !row_range.contains(&(x as i32 + x_off))
                                        {
                                            //If it's the center chunk, or out of bounds continue
                                            continue;
                                        }

                                        let x = (x as i32 + x_off) / 2;
                                        let y = (y as i32 + y_off) / 2;
                                        let index_off = y * CHUNKS_WIDTH as i32 / 2 + x;

                                        let (group_idx, mut_idx) = match (x_off, y_off) {
                                            // Right Left
                                            (1, 0) => (2, 1),
                                            (-1, 0) => (1, 2),
                                            // Top Down
                                            (0, 1) => (3, 0),
                                            (0, -1) => (0, 3),
                                            // Corners
                                            (1, 1) => (3, 0),
                                            (-1, 1) => (2, 1),
                                            (1, -1) => (1, 2),
                                            (-1, -1) => (0, 3),
                                            _ => unreachable!(),
                                        };

                                        if x_off.abs() != y_off.abs() {
                                            // Side
                                            chunk_group.sides[group_idx] = mutable_references.sides
                                                [mut_idx]
                                                [index_off as usize]
                                                .take();
                                        } else {
                                            // Corner
                                            chunk_group.corners[group_idx] = mutable_references
                                                .corners[mut_idx]
                                                [index_off as usize]
                                                .take()
                                        }
                                    }
                                }

                                let textures_update = Arc::clone(&textures_update);

                                let actors = &actors_vec;
                                scope.spawn(async move {
                                    update_chunks(
                                        &mut (chunk_group, &textures_update, deferred_updates_send),
                                        dt,
                                        actors,
                                        &rect,
                                    )
                                });
                            }
                        });
                    });
                });
            }
        }

        // Close the deferred updates channel so that our deferred update task will complete.
        deferred_updates_send.close();
    });

    // Once we are done with our updates, swap the new dirty rects to the current one.
    dirty_rects_resource.swap();

    let mut uptextures_comp = uptextures_query.single_mut();
    uptextures_comp.0.replace(
        Arc::try_unwrap(textures_update)
            .unwrap()
            .into_inner()
            .unwrap(),
    );
}

pub fn update_chunks(
    chunks: &mut UpdateChunksType,
    dt: f32,
    actors: &[(Actor, Transform)],
    dirty_rect: &Rect,
) {
    for y in rand_range(dirty_rect.min.y as usize..dirty_rect.max.y as usize + 1) {
        for x in rand_range(dirty_rect.min.x as usize..dirty_rect.max.x as usize + 1) {
            let local_pos = (ivec2(x as i32, y as i32), 4);
            let pos = local_to_global(local_pos);

            if !dt_updatable(chunks, pos, dt) {
                continue;
            }

            let mut awake_self = false;
            let state;
            let vel;
            {
                let atom = &mut chunks.0[local_pos];
                state = atom.state;
                vel = atom.velocity.is_some();

                if atom.f_idle < FRAMES_SLEEP && state != State::Void && state != State::Solid {
                    atom.f_idle += 1;
                    awake_self = true;
                }
            }

            let mut awakened = if vel {
                update_particle(chunks, pos, dt, actors)
            } else {
                match state {
                    State::Powder => update_powder(chunks, pos, dt, actors),
                    State::Liquid => update_liquid(chunks, pos, dt, actors),
                    _ => HashSet::new(),
                }
            };

            if awakened.contains(&pos) {
                let atom = &mut chunks.0[local_pos];
                atom.f_idle = 0;
            } else if awake_self {
                awakened.insert(pos);
            }

            for awoke in awakened {
                for x in -1..=1 {
                    for y in -1..=1 {
                        let local = global_to_local(awoke + ivec2(x, y));
                        let chunk_manager_idx =
                            ChunkGroup::group_to_manager_idx(chunks.0.center_index, local.1);
                        if (0..CHUNKS_WIDTH * CHUNKS_HEIGHT).contains(&chunk_manager_idx) {
                            chunks
                                .2
                                .try_send(DeferredUpdate::UpdateDirtyRect {
                                    chunk_idx: chunk_manager_idx,
                                    pos: local.0.as_vec2(),
                                })
                                .unwrap();
                        }
                    }
                }
            }
        }
    }
}

pub fn textures_update(
    chunk_manager: Query<&ChunkManager>,
    mut images: ResMut<Assets<Image>>,
    mut uptextures_query: Query<&mut UpdateTextures>,
) {
    let mut uptextures_hash = uptextures_query.single_mut();
    let chunk_manager = chunk_manager.single();

    // TODO: Parallelize texture update on GPU.
    images.iter_mut().for_each(|(id, image)| {
        if let Some(chunk_index) = chunk_manager.textures_hmap.get(&id) {
            if let Some(pos_set) = uptextures_hash
                .as_ref()
                .0
                .as_ref()
                .unwrap()
                .get(chunk_index)
            {
                let chunk = &chunk_manager.chunks[*chunk_index];
                chunk.update_image_positions(image, pos_set);
            }
        }
    });

    uptextures_hash.0 = None;
}

pub struct ChunkManagerPlugin;
impl Plugin for ChunkManagerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, manager_setup).add_systems(
            Update,
            (
                chunk_manager_update,
                textures_update.after(chunk_manager_update),
            ),
        );
    }
}
