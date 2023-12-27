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
    pub chunks: HashMap<IVec2, Chunk>,
    pub colliders: ChunkColliders,
    pub textures_hmap: HashMap<AssetId<Image>, IVec2>,
    pub dt: u8,
}

impl ChunkManager {
    pub fn get_atom(&self, pos: &ChunkPos) -> Option<&Atom> {
        if let Some(chunk) = self.chunks.get(&pos.chunk) {
            chunk.atoms.get(pos.atom.d1())
        } else {
            None
        }
    }

    pub fn get_mut_atom(&mut self, pos: ChunkPos) -> Option<&mut Atom> {
        if let Some(chunk) = self.chunks.get_mut(&pos.chunk) {
            chunk.atoms.get_mut(pos.atom.d1())
        } else {
            None
        }
    }
}

impl std::ops::Index<ChunkPos> for ChunkManager {
    type Output = Atom;
    #[track_caller]
    fn index(&self, pos: ChunkPos) -> &Self::Output {
        self.get_atom(&pos).expect("Invalid index position.")
    }
}
impl std::ops::IndexMut<ChunkPos> for ChunkManager {
    #[track_caller]
    fn index_mut(&mut self, pos: ChunkPos) -> &mut Self::Output {
        self.get_mut_atom(pos).expect("Invalid index position.")
    }
}

#[derive(Clone)]
pub struct ChunkColliders {
    data: HashMap<IVec2, HashMap<UVec2, u8>>,
}

impl ChunkColliders {
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
        }
    }

    pub fn get_collider(&self, pos: &ChunkPos) -> Option<&u8> {
        if let Some(chunk) = self.data.get(&pos.chunk) {
            chunk.get(&pos.atom)
        } else {
            None
        }
    }

    pub fn get_mut_collider(&mut self, pos: ChunkPos) -> Option<&mut u8> {
        if let Some(chunk) = self.data.get_mut(&pos.chunk) {
            chunk.get_mut(&pos.atom)
        } else {
            None
        }
    }

    pub fn add_one(&mut self, pos: ChunkPos) {
        if let Some (chunk) = self.data.get_mut(&pos.chunk) {
            if let Some(collider) = chunk.get_mut(&pos.atom) {
                *collider += 1
            } else {
                chunk.insert(pos.atom, 1);
            }
        } else {
            let mut chunk_hash = HashMap::new();
            chunk_hash.insert(pos.atom, 1);
            self.data.insert(pos.chunk, chunk_hash);
        }
    }

    pub fn remove_one(&mut self, pos: ChunkPos) {
        if self[pos] == 1 {
            self.data.get_mut(&pos.chunk).unwrap().remove(&pos.atom);
            if self.data.get_mut(&pos.chunk).unwrap().is_empty() {
                self.data.remove(&pos.chunk);
            }
        } else {
            self[pos] -= 1;
        }
    }
}

impl std::ops::Index<ChunkPos> for ChunkColliders {
    type Output = u8;
    #[track_caller]
    fn index(&self, pos: ChunkPos) -> &Self::Output {
        self.get_collider(&pos).expect("Invalid index position.")
    }
}
impl std::ops::IndexMut<ChunkPos> for ChunkColliders {
    #[track_caller]
    fn index_mut(&mut self, pos: ChunkPos) -> &mut Self::Output {
        self.get_mut_collider(pos).expect("Invalid index position.")
    }
}

#[derive(Component)]
pub struct DirtyRects {
    /// The current chunk update dirty rects
    pub current: HashMap<IVec2, URect>,
    /// The new chunk update dirty rects
    pub new: HashMap<IVec2, URect>,
    /// The dirty render rects
    pub render: HashMap<IVec2, URect>,
}

impl DirtyRects {
    pub fn swap(&mut self) {
        std::mem::swap(&mut self.current, &mut self.new)
    }
}

pub fn manager_setup(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    asset_server: Res<AssetServer>,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
) {
    let side_length = (CHUNK_LENGHT * ATOM_SIZE) as f32;
    let (width, height) = (CHUNKS_WIDTH, CHUNKS_HEIGHT);

    let mut images_vec = vec![];
    let mut chunks = HashMap::new();
    let mut textures_hmap = HashMap::new();
    for y in 0..height {
        for x in 0..width {
            let pos = Vec2::new(x as f32 * side_length, -(y as f32) * side_length);
            let index = ivec2(x as i32, y as i32);

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

            chunks.insert(index, chunk);
        }
    }

    commands
        .spawn((
            Name::new("Chunks textures"),
            VisibilityBundle::default(),
            TransformBundle::default(),
        ))
        .push_children(&images_vec);

    let mut chunk_manager = ChunkManager {
        chunks,
        colliders: ChunkColliders::new(),
        dt: 0,
        textures_hmap,
    };

    let player_actor = Actor {
        height: 17,
        width: 10,
        pos: ivec2(0, 0),
        vel: vec2(0., 0.),
    };
    add_actor(&mut chunk_manager, &player_actor);

    let texture_handle = asset_server.load("player/player_sheet.png");
    let texture_atlas =
        TextureAtlas::from_grid(texture_handle, Vec2::new(24.0, 24.0), 8, 4, None, None);
    let texture_atlas_handle = texture_atlases.add(texture_atlas);
    let animation_indices = AnimationIndices { first: 0, last: 1 };
    let mut transform = Transform::from_scale(Vec3::splat(3.0));
    transform.translation = vec2(5. * 3., -8. * 3.).extend(2.);

    commands.spawn((
        player_actor,
        Player::default(),
        SpriteSheetBundle {
            texture_atlas: texture_atlas_handle,
            sprite: TextureAtlasSprite::new(animation_indices.first),
            transform,
            ..default()
        },
        animation_indices,
        AnimationTimer(Timer::from_seconds(0.1, TimerMode::Repeating)),
    ));

    commands.spawn(DirtyRects {
        current: HashMap::new(),
        new: HashMap::new(),
        render: HashMap::new(),
    });
    commands.spawn(chunk_manager);
}

pub fn chunk_manager_update(
    mut chunk_manager: Query<&mut ChunkManager>,
    mut dirty_rects: Query<&mut DirtyRects>,
) {
    let mut chunk_manager = chunk_manager.single_mut();
    let colliders = &chunk_manager.colliders.clone();

    chunk_manager.dt = chunk_manager.dt.wrapping_add(1);
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
            *new_dirty_rects = HashMap::new();

            // Loop through deferred tasks
            while let Ok(update) = dirty_update_rects_recv.recv().await {
                update_dirty_rects(new_dirty_rects, update.chunk_pos);
            }
        });

        // Spawn a task on the deferred scope for handling deferred dirty render rects.
        deferred_scope.spawn(async move {
            // Clear the previous render rects
            *render_dirty_rects = HashMap::new();

            //Update all rendering
            for x in 0..CHUNKS_WIDTH {
                for y in 0..CHUNKS_HEIGHT {
                    render_dirty_rects
                        .insert(IVec2::new(x as i32, y as i32), URect::new(0, 0, 63, 63));
                }
            }

            // Loop through deferred tasks
            while let Ok(update) = dirty_render_rects_recv.recv().await {
                let pos = update.chunk_pos;
                if let Some(rect) = render_dirty_rects.get_mut(&pos.chunk) {
                    extend_rect_if_needed(rect, &pos.atom)
                } else {
                    render_dirty_rects.insert(
                        pos.chunk,
                        URect::new(pos.atom.x, pos.atom.y, pos.atom.x, pos.atom.y),
                    );
                }
            }
        });

        // Run the 4 update steps in checker like pattern
        for y_thread_off in rand_range(0..2) {
            for x_thread_off in rand_range(0..2) {
                compute_pool.scope(|scope| {
                    let mut mutable_references = HashMap::new();
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
                            let center_pos = ivec2(x as i32, y as i32);
                            if let Some(rect) = dirty_rects.get(&center_pos) {
                                let center = if let ChunkReference::Center(center) =
                                    mutable_references.remove(&center_pos).unwrap()
                                {
                                    center
                                } else {
                                    unreachable!()
                                };

                                let mut chunk_group = ChunkGroup::new(center, center_pos);

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

                                        let (group_idx, reference_idx) = match (x_off, y_off) {
                                            // Right Left
                                            (1, 0) => (2, 0),
                                            (-1, 0) => (1, 1),
                                            // Top Down
                                            (0, 1) => (3, 0),
                                            (0, -1) => (0, 1),
                                            // Corners
                                            (1, 1) => (3, 0),
                                            (-1, 1) => (2, 1),
                                            (1, -1) => (1, 2),
                                            (-1, -1) => (0, 3),
                                            _ => unreachable!(),
                                        };

                                        if x_off.abs() != y_off.abs() {
                                            // Side
                                            let side =
                                                if let Some(ChunkReference::Sides(mut sides)) =
                                                    mutable_references
                                                        .remove(&(center_pos + ivec2(x_off, y_off)))
                                                {
                                                    sides[reference_idx].take()
                                                } else {
                                                    None
                                                };

                                            chunk_group.sides[group_idx] = side;
                                        } else {
                                            // Corner
                                            let corner =
                                                if let Some(ChunkReference::Corners(mut corners)) =
                                                    mutable_references
                                                        .remove(&(center_pos + ivec2(x_off, y_off)))
                                                {
                                                    corners[reference_idx].take()
                                                } else {
                                                    None
                                                };

                                            chunk_group.corners[group_idx] = corner;
                                        }
                                    }
                                }

                                scope.spawn(async move {
                                    update_chunks(
                                        &mut UpdateChunksType {
                                            group: chunk_group,
                                            dirty_update_rect_send,
                                            dirty_render_rect_send,
                                            colliders
                                        },
                                        dt,
                                        rect,
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

pub fn update_chunks(chunks: &mut UpdateChunksType, dt: u8, dirty_rect: &URect) {
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
                vel = atom.velocity != (0, 0);

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
                let chunk = ChunkGroup::group_to_chunk(chunks.group.center_pos, local.1);

                chunks
                    .dirty_update_rect_send
                    .try_send(DeferredDirtyRectUpdate {
                        chunk_pos: ChunkPos::new(local.0.try_into().unwrap(), chunk),
                    })
                    .unwrap();
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

    for (chunk_pos, chunk) in chunk_manager.chunks.iter() {
        if let Some(rect) = dirty_rects.render.get(chunk_pos) {
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
