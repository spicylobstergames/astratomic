use std::f32::consts::PI;
use std::ops::Range;
use std::sync::Mutex;
use std::sync::{Arc, RwLock};
use std::{thread, vec};

use rand::seq::SliceRandom;
use rand::Rng;

use bevy::math::ivec2;
use bevy::math::vec2;
use bevy::prelude::*;
use bevy::sprite::{self, Anchor};

use crate::atom::State;
use crate::chunk::*;
use crate::consts::*;
use crate::geom_tools::{circle_points, Line};
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

fn grid_setup(mut commands: Commands, windows: Query<&Window>, mut images: ResMut<Assets<Image>>) {
    let window = windows.single();
    let side_length = (CHUNK_SIZE * ATOM_SIZE) as f32;

    let (mut width, mut height) = (
        (window.width() / side_length).ceil() as usize,
        (window.height() / side_length).ceil() as usize,
    );

    //If chunks aren't even, make them
    if width % 2 != 0 {
        width += 1
    }
    if height % 2 != 0 {
        height += 1
    }

    println!("{} {}", width, height);

    let mut images_vec = vec![];
    let mut chunks = vec![];
    for y in 0..height {
        for x in 0..width {
            // Get image position
            let mut pos = Vec2::new(x as f32 * side_length, -(y as f32) * side_length);
            pos.x -= width as f32 / 2. * side_length;
            pos.y += height as f32 / 2. * side_length;

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
                        transform: Transform::from_xyz(pos.x, pos.y, 0.),
                        ..Default::default()
                    })
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

    println!("{}", std::mem::size_of::<Grid>());

    commands.spawn(grid);
}

pub fn grid_update(
    mut commands: Commands,
    mut grid: Query<&mut Grid>,
    mut images: ResMut<Assets<Image>>,
    time: Res<Time>,
    actors: Query<(&Actor, &Transform)>,
    rects: Query<Entity, With<DirtyRect>>,
) {
    let mut grid = grid.single_mut();

    grid.dt += time.delta_seconds();
    let dt = grid.dt;
    if dt < UPDATE_TIME {
        return;
    }

    // Get actors
    let mut actors_vec = vec![];
    for (actor, transform) in actors.iter() {
        actors_vec.push((*actor, *transform))
    }

    // Get images
    let images_removed: Vec<(Handle<Image>, Arc<Mutex<Image>>)> = grid
        .chunks
        .iter()
        .map(|chunk| {
            (
                chunk.read().unwrap().texture.clone(),
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
    for image in images_removed {
        images.set_untracked(
            image.0,
            Arc::try_unwrap(image.1).unwrap().into_inner().unwrap(),
        )
    }

    // Dirty Rect rendering
    for rect in rects.iter() {
        commands.entity(rect).despawn();
    }

    let (width, height) = (grid.width, grid.height);
    for (i, chunk) in grid.chunks.iter_mut().enumerate() {
        let mut rect = chunk.read().unwrap().dirty_rect;
        rect = None;

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
                        (chunk_x * CHUNK_SIZE * ATOM_SIZE) as f32 + (rect.min.x * ATOM_SIZE as f32)
                            - width as f32 / 2. * (CHUNK_SIZE * ATOM_SIZE) as f32,
                        -((chunk_y * CHUNK_SIZE * ATOM_SIZE) as f32)
                            - (rect.min.y * ATOM_SIZE as f32)
                            + height as f32 / 2. * (CHUNK_SIZE * ATOM_SIZE) as f32,
                        1.,
                    )),
                    ..default()
                })
                .insert(DirtyRect);
        }
    }

    grid.dt -= UPDATE_TIME;
}

fn rand_range(vec: Range<usize>) -> Vec<usize> {
    let mut vec: Vec<usize> = vec.collect();
    vec.shuffle(&mut rand::thread_rng());
    vec
}

pub fn update_chunks(
    chunks: UpdateChunksType,
    dt: f32,
    actors: Vec<(Actor, Transform)>,
    dirty_rect: Rect,
) {
    for y in rand_range(dirty_rect.min.y as usize..dirty_rect.max.y as usize + 1) {
        for x in rand_range(dirty_rect.min.x as usize..dirty_rect.max.x as usize + 1) {
            let pos = ivec2(x as i32, y as i32);
            let pos = local_to_global((pos, 4));

            if !dt_upable(&chunks, pos, dt) {
                continue;
            }

            let state;
            let mut awake_self = false;
            let vel;
            {
                let local_pos = global_to_local(pos);
                state = chunks[local_pos.1 as usize]
                    .clone()
                    .unwrap()
                    .0
                    .read()
                    .unwrap()
                    .atoms[local_pos.0.d1()]
                .state;

                vel = chunks[local_pos.1 as usize]
                    .clone()
                    .unwrap()
                    .0
                    .read()
                    .unwrap()
                    .atoms[local_pos.0.d1()]
                .velocity
                .is_some();
            }
            {
                let local_pos = global_to_local(pos);
                let chunk = chunks[local_pos.1 as usize].clone().unwrap().0;
                let mut chunk = chunk.write().unwrap();

                let atom = &mut chunk.atoms[local_pos.0.d1()];
                let state = atom.state;

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

            if awakened.contains(&pos.as_vec2()) {
                let local_pos = global_to_local(pos);
                let chunk = chunks[local_pos.1 as usize].clone().unwrap().0;
                let mut chunk = chunk.write().unwrap();

                let atom = &mut chunk.atoms[local_pos.0.d1()];
                atom.f_idle = 0;
            }

            if awake_self {
                awakened.push(pos.as_vec2())
            }

            //TODO more efficient solution
            for awoke in awakened {
                for x_off in -1..=1 {
                    for y_off in -1..=1 {
                        let awoke = awoke + vec2(x_off as f32, y_off as f32);

                        let local_pos = global_to_local(awoke.as_ivec2());
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

/// Updates powder and returns if it's moving or not
fn update_powder(
    chunks: &UpdateChunksType,
    pos: IVec2,
    dt: f32,
    _actors: &[(Actor, Transform)],
) -> Vec<Vec2> {
    let mut awakened = vec![];

    let mut cur_pos = pos;

    // Get fall speed
    let mut fall_speed = get_fspeed(chunks, cur_pos);
    if fall_speed < TERM_VEL {
        fall_speed += GRAVITY;
        set_fspeed(chunks, cur_pos, fall_speed);
    }

    for _ in 0..fall_speed {
        let neigh = down_neigh(chunks, cur_pos, &[(State::Liquid, 0.2)], dt);
        let mut swapped = false;
        for neigh in neigh {
            if neigh.0 {
                swap(chunks, cur_pos, cur_pos + neigh.1, dt);
                awakened.push(cur_pos.as_vec2());
                cur_pos += neigh.1;
                awakened.push(cur_pos.as_vec2());
                swapped = true;

                break;
            }
        }

        if !swapped {
            let new_vel = Vec2::new(0.0, -(fall_speed as f32));

            set_vel(
                chunks,
                cur_pos,
                Vec2::from_angle(rand::thread_rng().gen_range(-PI / 2.0..PI / 2.))
                    .rotate(new_vel * 0.3)
                    .as_ivec2(),
            );

            set_fspeed(chunks, cur_pos, 0);

            break;
        }
    }

    awakened
}

/// Updates liquid and returns if it's moving or not
fn update_liquid(
    chunks: &UpdateChunksType,
    pos: IVec2,
    dt: f32,
    _actors: &[(Actor, Transform)],
) -> Vec<Vec2> {
    let mut awakened = vec![];

    let mut cur_pos = pos;

    // Get fall speed
    let mut fall_speed = get_fspeed(chunks, pos);
    if fall_speed < TERM_VEL {
        fall_speed += GRAVITY;
        set_fspeed(chunks, pos, fall_speed);
    }

    let mut swapped = false;
    for _ in 0..fall_speed {
        let neigh = down_neigh(chunks, cur_pos, &[], dt);
        for neigh in neigh {
            if neigh.0 {
                swap(chunks, cur_pos, cur_pos + neigh.1, dt);
                awakened.push(cur_pos.as_vec2());
                cur_pos += neigh.1;
                awakened.push(cur_pos.as_vec2());
                swapped = true;

                break;
            }
        }
    }

    if !swapped {
        set_fspeed(chunks, cur_pos, 0);

        let neigh = side_neigh(chunks, cur_pos, &[], dt);
        let side = if neigh[0].0 {
            Some(neigh[0].1.x)
        } else if neigh[1].0 {
            Some(neigh[1].1.x)
        } else {
            None
        };

        if let Some(side) = side {
            for _ in 0..5 {
                if !swapable(chunks, cur_pos + IVec2::new(side, 0), &[], dt) {
                    break;
                }

                swap(chunks, cur_pos, cur_pos + IVec2::new(side, 0), dt);
                awakened.push(cur_pos.as_vec2());
                cur_pos += IVec2::new(side, 0);
                awakened.push(cur_pos.as_vec2());
            }
        }
    }

    //Fluid force
    const RADIUS: i32 = 10;

    let mut force = Vec2::ZERO;
    for point in circle_points(cur_pos, RADIUS) {
        if point == cur_pos {
            continue;
        }

        let state = get_state(chunks, point).unwrap_or(State::Solid);
        let sign = if state == State::Liquid {
            -1.
        } else if state == State::Void {
            1.
        } else {
            continue;
        };

        let rel_vec = (point - cur_pos).as_vec2();
        force += (rel_vec / (rel_vec.length())) * 3. * sign;
    }

    set_vel(chunks, cur_pos, force.as_ivec2());

    awakened
}

/// Updates particle and returns if it's moving or not
fn update_particle(
    chunks: &UpdateChunksType,
    pos: IVec2,
    dt: f32,
    _actors: &[(Actor, Transform)],
) -> Vec<Vec2> {
    let mut awakened = vec![];
    let mut cur_pos = pos;

    // Add gravity
    let mut vel = get_vel(chunks, cur_pos).unwrap_or(IVec2::ZERO);
    if vel.y < TERM_VEL as i32 {
        vel += GRAVITY as i32 * IVec2::Y;
        set_vel(chunks, cur_pos, vel);
    }

    let liquid = get_state(chunks, cur_pos).unwrap_or(State::Solid) == State::Liquid;
    let neigh = get_neigh(chunks, cur_pos, State::Liquid);
    let npos = Line::new(cur_pos, vel).next();
    if let Some(npos) = npos {
        if get_state(chunks, npos) == Some(State::Void) {
            let mut n = vec![];
            for neigh in neigh {
                if neigh.0 {
                    n.push(neigh.1);
                }
            }
            let len = n.len() as f32;

            for pos in n {
                add_vel(chunks, cur_pos + pos, (vel.as_vec2() / 3. / len).as_ivec2());
            }

            vel = (vel.as_vec2() * 0.66).as_ivec2();
        }
    }

    // Move
    for pos in Line::new(cur_pos, vel) {
        awakened.push(cur_pos.as_vec2());
        if swapable(chunks, pos, &[], dt) {
            swap(chunks, cur_pos, pos, dt);
            cur_pos = pos;
            awakened.push(cur_pos.as_vec2());
        } else if liquid && swapable(chunks, pos, &[(State::Liquid, 1.0)], dt) {
            add_vel(chunks, pos, get_vel(chunks, cur_pos).unwrap_or(IVec2::ZERO));
            awakened.push(pos.as_vec2());
            set_vel(chunks, cur_pos, IVec2::ZERO);
            break;
        } else {
            if vel.abs().x > 5 && vel.abs().y > 5 {
                set_vel(
                    chunks,
                    cur_pos,
                    (Vec2::from_angle(PI).rotate(vel.as_vec2()) * 0.5).as_ivec2(),
                );
            } else {
                set_vel(chunks, cur_pos, IVec2::ZERO);
            }
            break;
        }
    }
    awakened
}

pub struct GridPlugin;
impl Plugin for GridPlugin {
    fn build(&self, app: &mut App) {
        app.add_startup_system(grid_setup).add_system(grid_update);
    }
}

#[derive(Component)]
pub struct DirtyRect;
