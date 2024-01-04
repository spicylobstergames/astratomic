use bevy::render::render_asset::{RenderAssetDependency, RenderAssets};
use bevy::render::render_resource::{Extent3d, ImageCopyTexture, ImageDataLayout, Origin3d};
use bevy::render::renderer::RenderQueue;
use bevy::render::{Extract, RenderApp, RenderSet};
use bevy::sprite::Anchor;
use itertools::Itertools;
use smallvec::SmallVec;

use crate::prelude::*;

/// Updates and do the chunks logic
#[derive(Default, Resource)]
pub struct ChunkManager {
    pub chunks: HashMap<IVec2, Chunk>,
    pub colliders: ChunkColliders,
    pub pos: IVec2,
    pub dt: u8,
}

//If true the direction is 1, if false the direction is -1
pub enum MoveDir {
    X(i32),
    Y(i32),
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

    //Still needs to save file chunks to file after this function is called
    pub fn move_manager(
        &mut self,
        commands: &mut Commands,
        images: &mut ResMut<Assets<Image>>,
        chunk_textures: &Entity,
        image_entities: &Query<(&Parent, Entity, &Handle<Image>)>,
        file_chunks: &mut HashMap<IVec2, Chunk>,
        move_dir: MoveDir,
    ) {
        let (for_load, vec_load, dir, y) = match move_dir {
            MoveDir::X(dir) => {
                self.pos.x += dir;
                (LOAD_HEIGHT, LOAD_WIDTH, dir, false)
            }
            MoveDir::Y(dir) => {
                self.pos.y += dir;
                (LOAD_WIDTH, LOAD_HEIGHT, dir, true)
            }
        };

        let mut images_vec = vec![];
        let mut to_remove = vec![];
        for i in 0..for_load {
            {
                //Save far chunks
                let mut to_add = ivec2(if dir == -1 { vec_load } else { -1 }, i);
                if y {
                    to_add = to_add.yx()
                }
                let pos = self.pos + to_add;

                let changed_chunk = self.chunks.remove(&pos).unwrap();
                to_remove.push(changed_chunk.texture.clone());
                images.remove(changed_chunk.texture.clone());

                if let Some(chunk) = file_chunks.get_mut(&pos) {
                    *chunk = changed_chunk;
                } else {
                    file_chunks.insert(pos, changed_chunk);
                }
            }

            {
                //Load new chunks
                let mut to_add = ivec2(if dir == 1 { vec_load - 1 } else { 0 }, i);
                if y {
                    to_add = to_add.yx()
                }
                let pos = self.pos + to_add;

                let chunk = if let Some(file_chunk) = file_chunks.get(&pos) {
                    file_chunk.clone()
                } else {
                    Chunk::new(Handle::default(), pos)
                };

                images_vec.push(add_chunk(commands, images, self, chunk, pos));
            }
        }

        for (parent, ent, handle) in image_entities.iter() {
            if parent.get() == *chunk_textures && to_remove.contains(handle) {
                commands.get_entity(ent).unwrap().despawn();
            }
        }
        let mut chunk_textures = commands.get_entity(*chunk_textures).unwrap();
        chunk_textures.insert_children(0, &images_vec);
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

#[derive(Default, Clone)]
pub struct ChunkColliders {
    data: HashMap<IVec2, HashMap<UVec2, u8>>,
}

impl ChunkColliders {
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
        if let Some(chunk) = self.data.get_mut(&pos.chunk) {
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

#[derive(Component)]
pub struct ChunkTextures;

pub fn manager_setup(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut chunk_manager: ResMut<ChunkManager>,
) {
    let (width, height) = (LOAD_WIDTH, LOAD_HEIGHT);

    let mut images_vec = vec![];
    chunk_manager.pos = ivec2(-16, -16);
    for (x, y) in (chunk_manager.pos.x..chunk_manager.pos.x + width)
        .cartesian_product(chunk_manager.pos.y..chunk_manager.pos.y + height)
    {
        let index = ivec2(x, y);
        let chunk = Chunk::new(Handle::default(), index);

        let ent = add_chunk(&mut commands, &mut images, &mut chunk_manager, chunk, index);
        images_vec.push(ent);
    }

    commands
        .spawn((
            Name::new("Chunks textures"),
            VisibilityBundle::default(),
            TransformBundle::default(),
            ChunkTextures,
        ))
        .push_children(&images_vec);

    commands.spawn(DirtyRects {
        current: HashMap::new(),
        new: HashMap::new(),
        render: HashMap::new(),
    });
}

pub fn chunk_manager_update(
    mut chunk_manager: ResMut<ChunkManager>,
    mut dirty_rects: Query<&mut DirtyRects>,
) {
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

    let first_x = chunk_manager.pos.x;
    let first_y = chunk_manager.pos.y;
    let last_x = chunk_manager.pos.x + LOAD_WIDTH;
    let last_y = chunk_manager.pos.y + LOAD_HEIGHT;
    let manager_pos = ivec2(chunk_manager.pos.x, chunk_manager.pos.y);

    let row_range = first_x..last_x;
    let column_range = first_y..last_y;

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
                update_dirty_rects_3x3(new_dirty_rects, update.chunk_pos);
            }
        });

        // Spawn a task on the deferred scope for handling deferred dirty render rects.
        deferred_scope.spawn(async move {
            // Clear the previous render rects
            *render_dirty_rects = HashMap::new();

            //Update all rendering, used when debugging
            /*
            for x in 0..CHUNKS_WIDTH {
                for y in 0..CHUNKS_HEIGHT {
                    render_dirty_rects
                        .insert(IVec2::new(x as i32, y as i32), URect::new(0, 0, 63, 63));
                }
            }
            */

            // Loop through deferred tasks
            while let Ok(update) = dirty_render_rects_recv.recv().await {
                update_dirty_rects(render_dirty_rects, update.chunk_pos);
            }
        });

        // Run the 4 update steps in checker like pattern
        for (y_toff, x_toff) in rand_range(0..2)
            .into_iter()
            .cartesian_product(rand_range(0..2).into_iter())
        {
            compute_pool.scope(|scope| {
                //Get chopped chunks references
                let mut mutable_references = HashMap::new();
                get_mutable_references(
                    &mut chunk_manager.chunks,
                    &mut mutable_references,
                    (x_toff, y_toff),
                    dirty_rects,
                    manager_pos,
                );

                //Iterate through the center chunks
                let y_iter = ((y_toff + first_y)..last_y).step_by(2);
                let x_iter = ((x_toff + first_x)..last_x).step_by(2);
                for (x, y) in x_iter.cartesian_product(y_iter) {
                    let center_pos = ivec2(x, y);
                    let Some(rect) = dirty_rects.get(&center_pos) else {
                        continue;
                    };
                    let ChunkReference::Center(center) =
                        mutable_references.remove(&center_pos).unwrap()
                    else {
                        unreachable!()
                    };
                    let mut chunk_group = ChunkGroup::new(center, center_pos);

                    // Get the rest of the 3x3 chunks to add to the chunk group
                    for (x_off, y_off) in (-1..=1).cartesian_product(-1..=1) {
                        //If it's the center chunk, or out of bounds continue
                        if (x_off == 0 && y_off == 0)
                            || !column_range.contains(&(y + y_off))
                            || !row_range.contains(&(x + x_off))
                        {
                            continue;
                        }

                        let (group_idx, reference_idx) = match (x_off, y_off) {
                            // Left Right
                            (-1, 0) => (1, 1),
                            (1, 0) => (2, 0),
                            // Up Down
                            (0, -1) => (0, 1),
                            (0, 1) => (3, 0),
                            // Corners
                            (-1, -1) => (0, 3),
                            (1, -1) => (1, 2),
                            (-1, 1) => (2, 1),
                            (1, 1) => (3, 0),

                            _ => unreachable!(),
                        };

                        if x_off.abs() != y_off.abs() {
                            // Side
                            let side = if let Some(ChunkReference::Side(ref mut side)) =
                                mutable_references.get_mut(&(center_pos + ivec2(x_off, y_off)))
                            {
                                side[reference_idx].take()
                            } else {
                                unreachable!()
                            };

                            chunk_group.sides[group_idx] = side;
                        } else {
                            // Corner
                            let corner = if let Some(ChunkReference::Corner(ref mut corner)) =
                                mutable_references.get_mut(&(center_pos + ivec2(x_off, y_off)))
                            {
                                corner[reference_idx].take()
                            } else {
                                unreachable!()
                            };

                            chunk_group.corners[group_idx] = corner;
                        }
                    }

                    scope.spawn(async move {
                        update_chunks(
                            &mut UpdateChunksType {
                                group: chunk_group,
                                dirty_update_rect_send,
                                dirty_render_rect_send,
                                colliders,
                            },
                            dt,
                            rect,
                        )
                    });
                }
            });
        }

        // Close the deferred updates channel so that our deferred update task will complete.
        dirty_update_rect_send.close();
        dirty_render_rect_send.close();
    });

    // Once we are done with our updates, swap the new dirty rects to the current one.
    dirty_rects_resource.swap();
}

pub fn update_chunks(chunks: &mut UpdateChunksType, dt: u8, dirty_rect: &URect) {
    let x_iter = rand_range(dirty_rect.min.x as i32..dirty_rect.max.x as i32 + 1).into_iter();
    let y_iter = rand_range(dirty_rect.min.y as i32..dirty_rect.max.y as i32 + 1).into_iter();
    for (y, x) in y_iter.cartesian_product(x_iter) {
        let local_pos = (ivec2(x, y), 4);
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
    chunk_manager: Extract<Res<ChunkManager>>,
    dirty_rects: Extract<Query<&DirtyRects>>,
    mut extracted_updates: ResMut<ExtractedTextureUpdates>,
) {
    let dirty_rects = dirty_rects.single();

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

pub fn save_to_file(chunk_manager: Res<ChunkManager>, input: Res<Input<KeyCode>>) {
    if input.just_pressed(KeyCode::K) {
        let file = File::open("assets/worlds/world").unwrap_or(File::create("assets/worlds/world").unwrap());
        let mut file_chunks: HashMap<IVec2, Chunk> =
            bincode::deserialize_from(BufReader::new(file)).unwrap_or_default();

        for (pos, chunk) in &chunk_manager.chunks {
            if let Some(file_chunk) = file_chunks.get_mut(pos) {
                *file_chunk = chunk.clone();
            } else {
                file_chunks.insert(*pos, chunk.clone());
            }
        }

        let mut f = BufWriter::new(File::open("assets/worlds/world").unwrap());
        serialize_into(&mut f, &file_chunks).unwrap();
    }
}

pub fn load_from_file(
    mut commands: Commands,
    mut chunk_manager: ResMut<ChunkManager>,
    chunk_textures: Query<Entity, With<ChunkTextures>>,
    input: Res<Input<KeyCode>>,
    mut images: ResMut<Assets<Image>>,
) {
    if input.just_pressed(KeyCode::L) {
        for chunk in chunk_manager.chunks.values() {
            images.remove(chunk.texture.clone());
        }
        chunk_manager.chunks = HashMap::new();

        chunk_manager.chunks = HashMap::new();
        let file = File::open("assets/worlds/world").unwrap();
        let file_chunks: HashMap<IVec2, Chunk> =
            bincode::deserialize_from(BufReader::new(file)).unwrap();

        //Add new chunks to world
        let mut images_vec = vec![];
        for (pos, chunk) in &file_chunks {
            let ent = add_chunk(
                &mut commands,
                &mut images,
                &mut chunk_manager,
                chunk.clone(),
                *pos,
            );
            images_vec.push(ent);
        }

        //Delete old and add new textures entities
        let mut chunk_textures = commands.get_entity(chunk_textures.single()).unwrap();
        chunk_textures
            .clear_children()
            .insert_children(0, &images_vec);
    }
}

//Still needs to add the return entity to a parent
pub fn add_chunk(
    commands: &mut Commands,
    images: &mut ResMut<Assets<Image>>,
    chunk_manager: &mut ChunkManager,
    mut chunk: Chunk,
    index: IVec2,
) -> Entity {
    let pos = Vec2::new(
        index.x as f32 * SIDE_LENGHT,
        (-index.y as f32) * SIDE_LENGHT,
    );

    //Add texture
    chunk.texture = images.add(Chunk::new_image());
    let texture_copy = chunk.texture.clone();

    //Update chunk image
    let image = images.get_mut(&chunk.texture).unwrap();
    chunk.update_all(image);
    chunk_manager.chunks.insert(index, chunk);

    //Spawn Image
    commands
        .spawn(SpriteBundle {
            texture: texture_copy,
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
        .id()
}

pub struct ChunkManagerPlugin;
impl Plugin for ChunkManagerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, manager_setup)
            .add_systems(Update, chunk_manager_update)
            .add_systems(
                PreUpdate,
                (save_to_file, load_from_file.after(save_to_file)),
            )
            .init_resource::<ChunkManager>();

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
