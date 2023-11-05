use std::sync::Mutex;
use std::sync::{Arc, RwLock};
use std::{thread, vec};

use bevy::math::{ivec2, vec3};
use bevy::prelude::*;
use bevy::sprite::{self, Anchor};

use crate::atom::State;
use crate::atom::*;
use crate::chunk::*;
use crate::consts::*;
use crate::grid_api::*;
use crate::player::Actor;

/// The grid is the chunk manager, it updates and do the chunks logic
#[derive(Component)]
pub struct Grid {
    pub chunks: Vec<Arc<RwLock<Chunk>>>,
    pub width: usize,
    pub height: usize,
    pub dt: f32,
}

#[derive(Component)]
pub struct ChunkSprite(usize);

fn grid_setup(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let side_length = (CHUNK_SIZE * ATOM_SIZE) as f32;
    let (width, height) = (8, 4);

    let mut images_vec = vec![];
    let mut chunks = vec![];
    for y in 0..height {
        for x in 0..width {
            let pos = Vec2::new(x as f32 * side_length, -(y as f32) * side_length);

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
                    .insert(ChunkSprite(chunks.len()))
                    .id(),
            );

            //Create chunk
            let chunk = Chunk::new(texture);

            //Update chunk image
            let image = images.get_mut(&chunk.texture).unwrap();
            chunk.update_all(image);

            chunks.push(Arc::new(RwLock::new(chunk)));
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
    };

    commands.spawn(grid);
}

pub fn grid_update(
    mut commands: Commands,
    mut grid: Query<&mut Grid>,
    mut images: ResMut<Assets<Image>>,
    time: Res<Time>,
    actors: Query<(&Actor, &Transform)>,
    rects: Query<Entity, With<DirtyRect>>,
    mut chunk_sprites: Query<(&ChunkSprite, &mut Handle<Image>)>,
) {
    let mut grid = grid.single_mut();

    grid.dt += time.delta_seconds();
    let dt = grid.dt;

    // Get actors
    let mut actors_vec = vec![];
    for (actor, transform) in actors.iter() {
        actors_vec.push((*actor, *transform))
    }

    // Get images
    let images_removed: Vec<(usize, Arc<Mutex<Image>>)> = grid
        .chunks
        .iter()
        .enumerate()
        .map(|(chunk_index, chunk)| {
            (
                chunk_index,
                Arc::new(Mutex::new(
                    images
                        .remove(chunk.read().unwrap().texture.clone())
                        .unwrap(),
                )),
            )
        })
        .collect();

    // Take dirty rects
    let dirty_rects: Vec<Option<Rect>> = grid
        .chunks
        .iter_mut()
        .map(|chunk| chunk.write().unwrap().dirty_rect.take())
        .collect();

    let row_range = 0..grid.width as i32;
    let column_range = 0..grid.height as i32;

    // Run the 4 update steps in checker like pattern
    for y_thread_off in rand_range(0..2) {
        for x_thread_off in rand_range(0..2) {
            let mut handles = vec![];

            //Acess chunks
            for y in (y_thread_off..grid.height).step_by(2) {
                for x in (x_thread_off..grid.width).step_by(2) {
                    if let Some(rect) = dirty_rects[y * grid.width + x] {
                        let mut chunks = vec![];
                        // Get all 3x3 chunks for each chunk updating
                        for y_off in -1..=1 {
                            for x_off in -1..=1 {
                                // Checks if chunk pos is within range
                                if !column_range.contains(&(y as i32 + y_off))
                                    || !row_range.contains(&(x as i32 + x_off))
                                {
                                    chunks.push(None);
                                    continue;
                                }

                                let index = ((y as i32 + y_off) * grid.width as i32
                                    + x as i32
                                    + x_off) as usize;

                                chunks.push(Some((
                                    Arc::clone(&grid.chunks[index]),
                                    Arc::clone(&images_removed[index].1),
                                )));
                            }
                        }

                        let actors = actors_vec.clone();
                        let handle = thread::spawn(move || update_chunks(chunks, dt, actors, rect));
                        handles.push(handle);
                    }
                }
            }

            // Wait for update step to finish
            for handle in handles {
                handle.join().unwrap()
            }
        }
    }

    // Add images back after update
    for (i, image) in images_removed {
        let handle = images.add(Arc::try_unwrap(image).unwrap().into_inner().unwrap());
        grid.chunks[i].write().unwrap().texture = handle;
    }
    for (index, mut texture) in chunk_sprites.iter_mut() {
        let i = index.0;
        *texture = grid.chunks[i].read().unwrap().texture.clone();
    }

    // Dirty Rect rendering
    for rect in rects.iter() {
        commands.entity(rect).despawn();
    }

    let width = grid.width;
    for (i, chunk) in grid.chunks.iter_mut().enumerate() {
        let rect = chunk.read().unwrap().dirty_rect;

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

pub fn update_chunks(
    chunks: UpdateChunksType,
    dt: f32,
    actors: Vec<(Actor, Transform)>,
    dirty_rect: Rect,
) {
    for y in rand_range(dirty_rect.min.y as usize..dirty_rect.max.y as usize + 1) {
        for x in rand_range(dirty_rect.min.x as usize..dirty_rect.max.x as usize + 1) {
            let local_pos = (ivec2(x as i32, y as i32), 4);
            let pos = local_to_global(local_pos);

            if !dt_updatable(&chunks, pos, dt) {
                continue;
            }

            let mut awake_self = false;
            let state;
            let vel;
            {
                let chunk = chunks[local_pos.1 as usize].clone().unwrap().0;
                let mut chunk = chunk.write().unwrap();

                let atom = &mut chunk.atoms[local_pos.0.d1()];
                state = atom.state;
                vel = atom.velocity.is_some();

                if atom.f_idle < FRAMES_SLEEP && state != State::Void && state != State::Solid {
                    atom.f_idle += 1;
                    awake_self = true;
                }
            }

            let mut awakened = if vel {
                update_particle(&chunks, pos, dt, &actors)
            } else {
                match state {
                    State::Powder => update_powder(&chunks, pos, dt, &actors),
                    State::Liquid => update_liquid(&chunks, pos, dt, &actors),
                    _ => vec![],
                }
            };

            if awakened.contains(&pos) {
                let chunk = chunks[local_pos.1 as usize].clone().unwrap().0;
                let mut chunk = chunk.write().unwrap();

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
                        if let Some(chunk) = &mut chunks[local_pos.1 as usize].clone() {
                            let mut chunk = chunk.0.write().unwrap();

                            let dirty_rect = &mut chunk.dirty_rect;

                            if let Some(dirty_rect) = dirty_rect.as_mut() {
                                extend_rect_if_needed(
                                    dirty_rect,
                                    &Vec2::new(local_pos.0.x as f32, local_pos.0.y as f32),
                                )
                            } else {
                                *dirty_rect = Some(Rect::new(
                                    local_pos.0.x as f32,
                                    local_pos.0.y as f32,
                                    local_pos.0.x as f32,
                                    local_pos.0.y as f32,
                                ))
                            }
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
            .add_systems(Update, grid_update);
    }
}

#[derive(Component)]
pub struct DirtyRect;
