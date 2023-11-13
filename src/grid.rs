use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;
use std::vec;

use async_channel::Sender;
use atomicell::AtomicCell;
use bevy::math::{ivec2, vec3};
use bevy::prelude::*;
use bevy::sprite::{self, Anchor};

use bevy::tasks::ComputeTaskPool;

use crate::actors::*;
use crate::atom::State;
use crate::atom::*;
use crate::chunk::*;
use crate::consts::*;
use crate::grid_api::*;

/// The grid is the chunk manager, it updates and do the chunks logic
#[derive(Component)]
pub struct Grid {
    pub chunks: Vec<AtomicCell<Chunk>>,
    pub width: usize,
    pub height: usize,
    pub textures_hmap: HashMap<AssetId<Image>, usize>,
    pub dt: f32,
}

#[derive(Component)]
pub struct UpdateTextures(Option<TexturesHash>);

fn grid_setup(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let side_length = (CHUNK_SIZE * ATOM_SIZE) as f32;
    let (width, height) = GRID_WIDTH_HEIGHT;

    let mut images_vec = vec![];
    let mut chunks = vec![];
    let mut textures_hmap = HashMap::new();
    for y in 0..height {
        for x in 0..width {
            let pos = Vec2::new(x as f32 * side_length, -(y as f32) * side_length);
            let index = y * GRID_WIDTH_HEIGHT.0 + x;

            //Get and spawn texture/chunk image
            let texture = images.add(Chunk::new_image());
            images_vec.push(
                commands
                    .spawn(SpriteBundle {
                        texture: texture.clone(),
                        sprite: Sprite {
                            anchor: sprite::Anchor::TopLeft,
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

            //Add texture to grid HashMap
            textures_hmap.insert(texture.id(), index);

            //Create chunk
            let chunk = Chunk::new(texture, index);

            //Update chunk image
            let image = images.get_mut(&chunk.texture).unwrap();
            chunk.update_all(image);

            chunks.push(AtomicCell::new(chunk));
        }
    }

    commands
        .spawn((
            Name::new("Chunks textures"),
            VisibilityBundle::default(),
            TransformBundle::default(),
        ))
        .push_children(&images_vec);

    let grid = Grid {
        chunks,
        width,
        height,
        dt: 0.,
        textures_hmap,
    };

    commands.spawn(grid);
    commands.spawn(UpdateTextures(None));
}

pub fn grid_update(
    mut commands: Commands,
    mut grid: Query<&mut Grid>,
    time: Res<Time>,
    actors: Query<(&Actor, &Transform)>,
    rects: Query<Entity, With<DirtyRect>>,
    mut uptextures_query: Query<&mut UpdateTextures>,
) {
    let mut grid = grid.single_mut();

    grid.dt += time.delta_seconds();
    let dt = grid.dt;

    let textures_update: ParTexturesHash = Arc::new(Mutex::new(HashMap::new()));

    // Get actors
    let mut actors_vec = vec![];
    for (actor, transform) in actors.iter() {
        actors_vec.push((*actor, *transform))
    }

    // Take dirty rects
    let dirty_rects: &Vec<Option<Rect>> = &grid
        .chunks
        .iter_mut()
        .map(|chunk| chunk.borrow_mut().dirty_rect.take())
        .collect();

    let row_range = 0..grid.width as i32;
    let column_range = 0..grid.height as i32;

    let pool = ComputeTaskPool::get();

    // Run the 4 update steps in checker like pattern
    let (deferred_updates_send, deferred_updates_recv) = async_channel::unbounded();
    let deferred_updates_send = &deferred_updates_send;
    for y_thread_off in rand_range(0..2) {
        for x_thread_off in rand_range(0..2) {
            pool.scope(|scope| {
                //Acess chunks
                let y_iter = (y_thread_off..grid.height).step_by(2);
                y_iter.for_each(|y| {
                    let x_iter = (x_thread_off..grid.width).step_by(2);
                    x_iter.for_each(|x| {
                        if let Some(rect) = dirty_rects[y * grid.width + x] {
                            let mut chunk_group =
                                ChunkGroup::new(grid.chunks[y * grid.width + x].borrow_mut());

                            // Get all 3x3 chunks for each chunk updating
                            let mut i = 0;
                            for y_off in -1..=1 {
                                for x_off in -1..=1 {
                                    if y_off == 0 && x_off == 0 {
                                        continue;
                                    }
                                    let surrounding_idx = i;
                                    i += 1;
                                    // Checks if chunk pos is within range
                                    if !column_range.contains(&(y as i32 + y_off))
                                        || !row_range.contains(&(x as i32 + x_off))
                                    {
                                        continue;
                                    }

                                    let index =
                                        ((y as i32 + y_off) * grid.width as i32 + x as i32 + x_off)
                                            as usize;

                                    chunk_group.surrounding[surrounding_idx] =
                                        Some(grid.chunks[index].borrow());
                                }
                            }

                            let textures_update = Arc::clone(&textures_update);

                            let actors = &actors_vec;
                            scope.spawn(async move {
                                update_chunks(
                                    &mut (chunk_group, &textures_update),
                                    deferred_updates_send,
                                    dt,
                                    actors,
                                    rect,
                                )
                            });
                        }
                    });
                });
            });

            // Make any updates to the sleeping chunks
            while let Ok(update) = deferred_updates_recv.try_recv() {
                match update {
                    DeferredChunkUpdate::SetAtom {
                        chunk_idx,
                        atom_idx,
                        atom: new_atom,
                    } => {
                        let mut chunk = grid.chunks[chunk_idx].borrow_mut();
                        let atom = &mut chunk.atoms[atom_idx];
                        *atom = new_atom;
                    }
                    DeferredChunkUpdate::UpdateDirtyRect { chunk_idx, pos } => {
                        let mut chunk = grid.chunks[chunk_idx].borrow_mut();
                        let dirty_rect = &mut chunk.dirty_rect;

                        if let Some(dirty_rect) = dirty_rect.as_mut() {
                            extend_rect_if_needed(dirty_rect, &pos)
                        } else {
                            *dirty_rect = Some(Rect::new(pos.x, pos.y, pos.x, pos.y))
                        }
                    }
                }
            }
        }
    }

    let mut uptextures_comp = uptextures_query.single_mut();
    uptextures_comp.0.replace(
        Arc::try_unwrap(textures_update)
            .unwrap()
            .into_inner()
            .unwrap(),
    );

    // Dirty Rect rendering
    for rect in rects.iter() {
        commands.entity(rect).despawn();
    }

    let width = grid.width;
    for (i, chunk) in grid.chunks.iter_mut().enumerate() {
        let rect = chunk.borrow().dirty_rect;

        if let Some(rect) = rect {
            let chunk_x = i % width;
            let chunk_y = i / width;

            // Rectangle
            commands
                .spawn(SpriteBundle {
                    sprite: Sprite {
                        color: Color::rgba(0.25, 0.25, 0.75, 0.50),
                        custom_size: Some(Vec2::new(
                            (rect.max.x - rect.min.x + 1.) * ATOM_SIZE as f32,
                            (rect.max.y - rect.min.y + 1.) * ATOM_SIZE as f32,
                        )),
                        anchor: Anchor::TopLeft,
                        ..default()
                    },
                    transform: Transform::from_translation(Vec3::new(
                        (chunk_x * CHUNK_SIZE * ATOM_SIZE) as f32 + (rect.min.x * ATOM_SIZE as f32),
                        -((chunk_y * CHUNK_SIZE * ATOM_SIZE) as f32)
                            - (rect.min.y * ATOM_SIZE as f32),
                        1.,
                    )),
                    ..default()
                })
                .insert(DirtyRect);
        }
    }
}

pub fn textures_update(
    grid: Query<&Grid>,
    mut images: ResMut<Assets<Image>>,
    mut uptextures_query: Query<&mut UpdateTextures>,
) {
    let mut uptextures_hash = uptextures_query.single_mut();
    let grid = grid.single();

    // TODO: Parallelize texture update on GPU.
    images.iter_mut().for_each(|(id, image)| {
        if let Some(chunk_index) = grid.textures_hmap.get(&id) {
            if let Some(pos_set) = uptextures_hash
                .as_ref()
                .0
                .as_ref()
                .unwrap()
                .get(chunk_index)
            {
                let chunk = grid.chunks[*chunk_index].borrow();
                chunk.update_image_positions(image, pos_set);
            }
        }
    });

    uptextures_hash.0 = None;
}

pub fn update_chunks(
    chunks: &mut UpdateChunksType,
    deferred_updates: &Sender<DeferredChunkUpdate>,
    dt: f32,
    actors: &[(Actor, Transform)],
    dirty_rect: Rect,
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
                let chunk = &mut chunks.0[local_pos.1 as usize];

                let atom = &mut chunk.atoms[local_pos.0.d1()];
                state = atom.state;
                vel = atom.velocity.is_some();

                if atom.f_idle < FRAMES_SLEEP && state != State::Void && state != State::Solid {
                    atom.f_idle += 1;
                    awake_self = true;
                }
            }

            let mut awakened = if vel {
                update_particle(chunks, deferred_updates, pos, dt, actors)
            } else {
                match state {
                    State::Powder => update_powder(chunks, deferred_updates, pos, dt, actors),
                    State::Liquid => update_liquid(chunks, deferred_updates, pos, dt, actors),
                    _ => vec![],
                }
            };

            if awakened.contains(&pos) {
                let chunk = &mut chunks.0[local_pos.1 as usize];

                let atom = &mut chunk.atoms[local_pos.0.d1()];
                atom.f_idle = 0;
            } else if awake_self {
                awakened.push(pos)
            }

            //TODO more efficient solution
            for awoke in awakened {
                for x_off in -1..=1 {
                    for y_off in -1..=1 {
                        let awoke = awoke + ivec2(x_off, y_off);

                        let local_pos = global_to_local(awoke);
                        if let Some(chunk) = chunks.0.get(local_pos.1 as usize) {
                            // TODO: This borrow fails when scribbling fast and wide.
                            deferred_updates
                                .try_send(DeferredChunkUpdate::UpdateDirtyRect {
                                    chunk_idx: chunk.index,
                                    pos: Vec2::new(local_pos.0.x as f32, local_pos.0.y as f32),
                                })
                                .unwrap();
                        }
                    }
                }
            }
        }
    }
}

pub struct GridPlugin;
impl Plugin for GridPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, grid_setup)
            .add_systems(Update, (grid_update, textures_update.after(grid_update)));
    }
}

#[derive(Component)]
pub struct DirtyRect;
