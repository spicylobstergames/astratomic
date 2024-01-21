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
    pub pos: IVec2,
    pub dt: u8,
}

//If true the direction is 1, if false the direction is -1
#[derive(Debug)]
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

impl Drop for ChunkManager {
    fn drop(&mut self) {
        let file = File::open("assets/world/world").unwrap();
        let mut buffered = BufReader::new(file);
        let mut file_chunks: HashMap<IVec2, Chunk> =
            bincode::deserialize_from(&mut buffered).unwrap();

        for (pos, chunk) in &self.chunks {
            if let Some(file_chunk) = file_chunks.get_mut(pos) {
                *file_chunk = chunk.clone();
            } else {
                file_chunks.insert(*pos, chunk.clone());
            }
        }

        let file = File::create("assets/world/world").unwrap();
        let mut buffered = BufWriter::new(file);
        bincode::serialize_into(&mut buffered, &file_chunks).unwrap();
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

#[derive(Resource, Default)]
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

    let file_chunks: HashMap<IVec2, Chunk>;
    if let Ok(file) = File::open("assets/world/world") {
        let mut buffered = BufReader::new(file);
        file_chunks = bincode::deserialize_from(&mut buffered).unwrap();
    } else {
        file_chunks = HashMap::new();
        let file = File::create("assets/world/world").unwrap();
        let mut buffered = BufWriter::new(file);
        bincode::serialize_into(&mut buffered, &file_chunks).unwrap();
    }

    for (x, y) in (chunk_manager.pos.x..chunk_manager.pos.x + width)
        .cartesian_product(chunk_manager.pos.y..chunk_manager.pos.y + height)
    {
        let index = ivec2(x, y);
        let chunk;
        if let Some(file_chunk) = file_chunks.get(&index) {
            chunk = file_chunk.clone();
        } else {
            chunk = Chunk::new(Handle::default(), index);
        }

        let ent = add_chunk(&mut commands, &mut images, &mut chunk_manager, chunk, index);
        images_vec.push(ent);
    }

    commands
        .spawn((
            Name::new("Chunks textures"),
            VisibilityBundle::default(),
            TransformBundle::from_transform(Transform::from_translation(vec3(
                0.,
                0.,
                AUTOMATA_LAYER,
            ))),
            ChunkTextures,
        ))
        .push_children(&images_vec);
}

pub fn chunk_manager_update(
    mut chunk_manager: ResMut<ChunkManager>,
    mut dirty_rects_resource: ResMut<DirtyRects>,
    materials: (Res<Assets<Materials>>, Res<MaterialsHandle>),
) {
    puffin::profile_function!();

    chunk_manager.dt = chunk_manager.dt.wrapping_add(1);
    let dt = chunk_manager.dt;

    // Get dirty rects
    let DirtyRects {
        current: dirty_rects,
        new: new_dirty_rects,
        render: render_dirty_rects,
    } = &mut *dirty_rects_resource;

    //Get materials
    let materials = &materials.0.get(materials.1 .0.clone()).unwrap();

    let manager_pos = ivec2(chunk_manager.pos.x, chunk_manager.pos.y);

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
                if update.awake_surrouding {
                    update_dirty_rects_3x3(new_dirty_rects, update.chunk_pos);
                } else {
                    update_dirty_rects(new_dirty_rects, update.chunk_pos)
                }
            }
        });

        // Spawn a task on the deferred scope for handling deferred dirty render rects.
        deferred_scope.spawn(async move {
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
            puffin::profile_scope!("Update step scope.");

            compute_pool.scope(|scope| {
                update_chunk_groups(
                    &mut chunk_manager.chunks,
                    (x_toff, y_toff),
                    dirty_rects,
                    manager_pos,
                    (dirty_update_rect_send, dirty_render_rect_send),
                    (dt, materials),
                    scope,
                );
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
    puffin::profile_function!();

    let materials = chunks.materials;

    let x_iter = rand_range(dirty_rect.min.x as i32..dirty_rect.max.x as i32 + 1).into_iter();
    let y_iter = rand_range(dirty_rect.min.y as i32..dirty_rect.max.y as i32 + 1).into_iter();
    for (y, x) in y_iter.cartesian_product(x_iter) {
        let local_pos = (ivec2(x, y), 4);
        let pos = local_to_global(local_pos);

        if !dt_updatable(chunks, pos, dt) {
            continue;
        }

        let mut awake_self = false;
        let id;
        let speed;
        {
            let atom = &mut chunks.group[local_pos];
            id = atom.id;
            speed = atom.speed;

            if atom.f_idle < FRAMES_SLEEP
                && materials[id] != Material::Void
                && materials[id] != Material::Solid
            {
                atom.f_idle += 1;
                awake_self = true;
            }
        }

        let (vector, mut awakened) = if speed.0 == 0 && speed.1 >= 0 {
            (
                false,
                match materials[id] {
                    Material::Powder {
                        inertial_resistance,
                    } => update_powder(chunks, pos, dt, inertial_resistance),
                    Material::Liquid { flow } => update_liquid(chunks, pos, flow, dt),
                    _ => HashSet::new(),
                },
            )
        } else {
            (true, update_atom(chunks, pos, dt))
        };

        let atom = &mut chunks.group[local_pos];
        let mut self_awakened = HashSet::new();
        if awakened.contains(&pos) {
            atom.f_idle = 0;
            atom.moving = true;
        } else if vector {
            atom.f_idle = 0;
            awakened.insert(pos);
            atom.moving = false;
        } else if awake_self {
            awakened.insert(pos);
            self_awakened.insert(pos);
            atom.moving = false;
        }

        for awoke in awakened {
            let local = global_to_local(awoke);
            let chunk = ChunkGroup::group_to_chunk(chunks.group.center_pos, local.1);

            chunks
                .dirty_update_rect_send
                .try_send(DeferredDirtyRectUpdate {
                    chunk_pos: ChunkPos::new(local.0.try_into().unwrap(), chunk),
                    awake_surrouding: !self_awakened.contains(&awoke),
                })
                .unwrap();
        }
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
        index.x as f32 * CHUNK_LENGHT as f32,
        (-index.y as f32) * CHUNK_LENGHT as f32,
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
            transform: Transform::from_xyz(pos.x, pos.y, 0.),
            ..Default::default()
        })
        .id()
}

pub fn update_manager_pos(
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
                    let file = File::open("assets/world/world").unwrap();
                    let mut buffered = BufReader::new(file);

                    let chunks: HashMap<IVec2, Chunk> =
                        bincode::deserialize_from(&mut buffered).unwrap();
                    (chunks, new_diff)
                });
            }
        }
        AsyncTaskStatus::Finished((mut file_chunks, diff)) => {
            let chunk_textures = chunk_textures.single();
            for _ in 0..diff.x.abs() {
                chunk_manager.move_manager(
                    &mut commands,
                    &mut images,
                    &chunk_textures,
                    &image_entities,
                    &mut file_chunks,
                    MoveDir::X(diff.x.signum()),
                );
            }

            for _ in 0..diff.y.abs() {
                chunk_manager.move_manager(
                    &mut commands,
                    &mut images,
                    &chunk_textures,
                    &image_entities,
                    &mut file_chunks,
                    MoveDir::Y(diff.y.signum()),
                );
            }

            let pool = AsyncComputeTaskPool::get();
            saving_task.0 = Some(pool.spawn(async move {
                let file = File::create("assets/world/world").unwrap();
                let mut buffered = BufWriter::new(file);
                bincode::serialize_into(&mut buffered, &file_chunks).unwrap();
            }));
        }
        AsyncTaskStatus::Pending => {}
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
    dirty_rects: Extract<Res<DirtyRects>>,
    mut extracted_updates: ResMut<ExtractedTextureUpdates>,
) {
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

fn clear_render_rect(mut dirty_rects: ResMut<DirtyRects>) {
    dirty_rects.render = HashMap::new();
}

pub struct ChunkManagerPlugin;
impl Plugin for ChunkManagerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, manager_setup)
            .add_systems(FixedUpdate, chunk_manager_update)
            .add_systems(Update, update_manager_pos)
            .add_systems(PreUpdate, clear_render_rect)
            .init_resource::<ChunkManager>()
            .init_resource::<DirtyRects>();

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
