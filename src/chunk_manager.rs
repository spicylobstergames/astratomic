use std::collections::{HashMap, HashSet};

use bevy::math::{ivec2, vec3};
use bevy::render::render_asset::{RenderAssetDependency, RenderAssets};
use bevy::render::render_resource::{Extent3d, ImageCopyTexture, ImageDataLayout, Origin3d};
use bevy::render::renderer::RenderQueue;
use bevy::render::{Extract, RenderApp, RenderSet};
use bevy::sprite::Anchor;
use bevy::tasks::ComputeTaskPool;
use smallvec::SmallVec;

use crate::prelude::*;

/// Updates and do the chunks logic
#[derive(Component)]
pub struct ChunkManager {
    pub chunks: Vec<Chunk>,
    pub textures_hmap: HashMap<AssetId<Image>, usize>,
    pub dt: f32,
}

#[derive(Component)]
pub struct DirtyRects {
    /// The current chunk update dirty rects
    pub current: Vec<Option<IRect>>,
    /// The new chunk update dirty rects
    pub new: Vec<Option<IRect>>,
    /// The dirty render rects
    pub render: Vec<Option<IRect>>,
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
        render: vec![None; CHUNKS_WIDTH * CHUNKS_HEIGHT],
    });
    commands.spawn(chunk_manager);
}

pub fn chunk_manager_update(
    mut chunk_manager: Query<&mut ChunkManager>,
    mut dirty_rects: Query<&mut DirtyRects>,
    time: Res<Time>,
) {
    let mut chunk_manager = chunk_manager.single_mut();

    chunk_manager.dt += time.delta_seconds();
    let dt = chunk_manager.dt;

    // Get dirty rects
    let mut dirty_rects_resource = dirty_rects.single_mut();
    let DirtyRects {
        current: dirty_rects,
        new: new_dirty_rects,
        render: render_dirty_rects,
    } = &mut *dirty_rects_resource;

    let row_range = 0..CHUNKS_WIDTH as i32;
    let column_range = 0..CHUNKS_HEIGHT as i32;

    let compute_pool = ComputeTaskPool::get();

    // Create channel for sending dirty update rects
    let (dirty_update_rects_send, dirty_update_rects_recv) =
        async_channel::unbounded::<DeferredDirtyRectUpdate>();
    let dirty_update_rect_send = &dirty_update_rects_send;

    // Create channel for sending dirty render rect updates
    let (dirty_render_rects_send, dirty_render_rects_recv) =
        async_channel::unbounded::<DeferredDirtyRectUpdate>();
    let dirty_render_rect_send = &dirty_render_rects_send;

    // Create a scope in which we handle deferred updates and update chunks.
    compute_pool.scope(|deferred_scope| {
        // Spawn a task on the deferred scope for handling the deferred dirty update rects.
        deferred_scope.spawn(async move {
            // Clear the new dirty rects so we can update a fresh list
            new_dirty_rects.iter_mut().for_each(|x| *x = None);

            // Loop through deferred tasks
            while let Ok(update) = dirty_update_rects_recv.recv().await {
                update_dirty_rects(
                    update.pos,
                    new_dirty_rects,
                    update.chunk_idx,
                    update.global_pos,
                    update.center_idx,
                );
            }
        });

        // Spawn a task on the deferred scope for handling deferred dirty render rects.
        deferred_scope.spawn(async move {
            // Clear the previous render rects
            render_dirty_rects.iter_mut().for_each(|x| *x = None);

            // Loop through deferred tasks
            while let Ok(update) = dirty_render_rects_recv.recv().await {
                let rect = &mut render_dirty_rects[update.chunk_idx];
                if let Some(rect) = rect {
                    extend_rect_if_needed(rect, &update.pos)
                } else {
                    *rect = Some(IRect::new(
                        update.pos.x,
                        update.pos.y,
                        update.pos.x,
                        update.pos.y,
                    ))
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

                                scope.spawn(async move {
                                    update_chunks(
                                        &mut UpdateChunksType {
                                            group: chunk_group,
                                            dirty_update_rect_send,
                                            dirty_render_rect_send,
                                        },
                                        dt,
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
        dirty_update_rect_send.close();
        dirty_render_rect_send.close();
    });

    // Once we are done with our updates, swap the new dirty rects to the current one.
    dirty_rects_resource.swap();
}

pub fn update_chunks(chunks: &mut UpdateChunksType, dt: f32, dirty_rect: &IRect) {
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
                let atom = &mut chunks.group[local_pos];
                state = atom.state;
                vel = atom.velocity.is_some();

                if atom.f_idle < FRAMES_SLEEP && state != State::Void && state != State::Solid {
                    atom.f_idle += 1;
                    awake_self = true;
                }
            }

            let mut awakened = if vel {
                update_particle(chunks, pos, dt)
            } else {
                match state {
                    State::Powder => update_powder(chunks, pos, dt),
                    State::Liquid => update_liquid(chunks, pos, dt),
                    _ => HashSet::new(),
                }
            };

            if awakened.contains(&pos) {
                let atom = &mut chunks.group[local_pos];
                atom.f_idle = 0;
            } else if awake_self {
                awakened.insert(pos);
            }

            for awoke in awakened {
                let local = global_to_local(awoke);
                let chunk_manager_idx =
                    ChunkGroup::group_to_manager_idx(chunks.group.center_index, local.1);
                if (0..CHUNKS_WIDTH * CHUNKS_HEIGHT).contains(&chunk_manager_idx) {
                    chunks
                        .dirty_update_rect_send
                        .try_send(DeferredDirtyRectUpdate {
                            chunk_idx: chunk_manager_idx,
                            pos: local.0,
                            global_pos: awoke,
                            center_idx: chunks.group.center_index,
                        })
                        .unwrap();
                }
            }
        }
    }
}

#[derive(Resource, Default, Deref, DerefMut)]
struct ExtractedTextureUpdates(Vec<ExtractedTextureUpdate>);

struct ExtractedTextureUpdate {
    id: AssetId<Image>,
    // TODO: determine a good size for the data smallvec array.
    // The size of the array determines how many bytes we can store before it overflows and has
    // to make a heap allocation. 256 is enough to store an 8x8 pixel dirty rect.
    data: SmallVec<[u8; 256]>,
    origin: Origin3d,
    size: Extent3d,
}

fn extract_chunk_texture_updates(
    chunk_manager: Extract<Query<&ChunkManager>>,
    dirty_rects: Extract<Query<&DirtyRects>>,
    mut extracted_updates: ResMut<ExtractedTextureUpdates>,
) {
    let dirty_rects = dirty_rects.single();
    let chunk_manager = chunk_manager.single();

    for (chunk, dirty_rect) in chunk_manager.chunks.iter().zip(&dirty_rects.render) {
        if let Some(rect) = dirty_rect {
            let rect = rect.as_urect();
            let id = chunk.texture.id();
            let mut data = SmallVec::new();

            for y in rect.min.y..=rect.max.y {
                for x in rect.min.x..=rect.max.x {
                    let pos = IVec2::new(x as i32, y as i32);
                    let color = chunk.atoms[pos.d1()].color;
                    data.extend_from_slice(&color)
                }
            }

            extracted_updates.push(ExtractedTextureUpdate {
                id,
                data,
                origin: Origin3d {
                    x: rect.min.x,
                    y: rect.min.y,
                    z: 0,
                },
                size: Extent3d {
                    width: rect.width() + 1,
                    height: rect.height() + 1,
                    depth_or_array_layers: 1,
                },
            });
        }
    }
}

fn prepare_chunk_gpu_textures(
    queue: Res<RenderQueue>,
    image_render_assets: Res<RenderAssets<Image>>,
    mut extracted_updates: ResMut<ExtractedTextureUpdates>,
) {
    for update in extracted_updates.drain(..) {
        let Some(gpu_image) = image_render_assets.get(update.id) else {
            continue;
        };

        queue.write_texture(
            ImageCopyTexture {
                texture: &gpu_image.texture,
                mip_level: 0,
                origin: update.origin,
                aspect: bevy::render::render_resource::TextureAspect::All,
            },
            &update.data,
            ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(update.size.width * 4),
                rows_per_image: None,
            },
            update.size,
        );
    }
}

pub struct ChunkManagerPlugin;
impl Plugin for ChunkManagerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, manager_setup)
            .add_systems(Update, chunk_manager_update);

        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .init_resource::<ExtractedTextureUpdates>()
                .add_systems(ExtractSchedule, extract_chunk_texture_updates);
            Image::register_system(
                render_app,
                prepare_chunk_gpu_textures.in_set(RenderSet::PrepareAssets),
            )
        }
    }
}
