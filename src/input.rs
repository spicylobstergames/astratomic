use bevy::prelude::*;
use line_drawing::Bresenham;
use rand::Rng;

use crate::atom::State;
use crate::atom::*;
use crate::consts::*;
use crate::grid::Grid;

fn camera(keys: Res<Input<KeyCode>>, mut camera_q: Query<&mut Transform, With<Camera>>) {
    let x = -(keys.pressed(KeyCode::A) as u8 as f32) + keys.pressed(KeyCode::D) as u8 as f32;
    let y = -(keys.pressed(KeyCode::S) as u8 as f32) + keys.pressed(KeyCode::W) as u8 as f32;

    let v = Vec2::new(x, y).normalize_or_zero().extend(0.);
    camera_q.single_mut().translation += v * CAMERA_SPEED;
}

fn brush(
    window: Query<&Window>,
    camera_q: Query<(&Camera, &GlobalTransform)>,
    mut grid: Query<&mut Grid>,
    mut images: ResMut<Assets<Image>>,
    prev_mpos: Query<&PreviousMousePos>,
    buttons: Res<Input<MouseButton>>,
    keys: Res<Input<KeyCode>>,
) {
    let (state, color);

    if keys.pressed(KeyCode::F) {
        state = State::Gas;
        color = [255, 255, 255, 255];
    } else if buttons.pressed(MouseButton::Left) {
        state = State::Powder;
        color = [
            (230 + rand::thread_rng().gen_range(-20..20)) as u8,
            (197 + rand::thread_rng().gen_range(-20..20)) as u8,
            (92 + rand::thread_rng().gen_range(-20..20)) as u8,
            255,
        ];
    } else if keys.pressed(KeyCode::LControl) {
        state = State::Liquid;
        color = [
            (20 + rand::thread_rng().gen_range(-20..20)) as u8,
            (125 + rand::thread_rng().gen_range(-20..20)) as u8,
            (204 + rand::thread_rng().gen_range(-20..20)) as u8,
            255,
        ];
    } else if buttons.pressed(MouseButton::Middle) {
        state = State::Solid;
        color = [127, 131, 134, 255];
    } else {
        return;
    }

    let (camera, camera_transform) = camera_q.single();
    let window = window.single();

    if let Some(mut world_position) = window
        .cursor_position()
        .and_then(|cursor| camera.viewport_to_world(camera_transform, cursor))
        .map(|ray| ray.origin.truncate())
    {
        let grid = grid.single_mut();

        world_position.x += (grid.grid_width * CHUNK_SIZE * ATOM_SIZE) as f32 * 0.5;
        world_position.y -= (grid.grid_height * CHUNK_SIZE * ATOM_SIZE) as f32 * 0.5;
        world_position.y *= -1.;
        let prev_mpos = prev_mpos.single().0.unwrap();

        for (x, y) in Bresenham::new(
            (prev_mpos.x as i32, prev_mpos.y as i32),
            (world_position.x as i32, world_position.y as i32),
        ) {
            let x = x as f32;
            let y = y as f32;

            if x < 0. || y < 0. {
                continue;
            }

            let (chunk_x, chunk_y) = (
                (x / (CHUNK_SIZE * ATOM_SIZE) as f32) as usize,
                (y / (CHUNK_SIZE * ATOM_SIZE) as f32) as usize,
            );

            let (atom_x, atom_y) = (
                ((x / ATOM_SIZE as f32) % CHUNK_SIZE as f32) as usize,
                ((y / ATOM_SIZE as f32) % CHUNK_SIZE as f32) as usize,
            );

            if chunk_x >= grid.grid_width || chunk_y >= grid.grid_height {
                continue;
            }

            let atom = Atom {
                color,
                state,
                ..Default::default()
            };
            let mut chunk = grid.chunks[chunk_y * grid.grid_width + chunk_x]
                .write()
                .unwrap();
            chunk.atoms[atom_y * CHUNK_SIZE + atom_x] = atom;
            chunk.update_image_positions(
                images.get_mut(&chunk.texture).unwrap(),
                &vec![IVec2::new(atom_x as i32, atom_y as i32)],
            );
            chunk.active = true;
        }
    }
}

#[derive(Component)]
pub struct PreviousMousePos(pub Option<Vec2>);

fn prev_mpos(
    window: Query<&Window>,
    camera_q: Query<(&Camera, &GlobalTransform)>,
    mut grid: Query<&mut Grid>,
    mut prev_mpos: Query<&mut PreviousMousePos>,
) {
    let (camera, camera_transform) = camera_q.single();
    let window = window.single();

    if let Some(mut world_position) = window
        .cursor_position()
        .and_then(|cursor| camera.viewport_to_world(camera_transform, cursor))
        .map(|ray| ray.origin.truncate())
    {
        let grid = grid.single_mut();

        world_position.x += (grid.grid_width * CHUNK_SIZE * ATOM_SIZE) as f32 * 0.5;
        world_position.y -= (grid.grid_height * CHUNK_SIZE * ATOM_SIZE) as f32 * 0.5;
        world_position.y *= -1.;

        prev_mpos.single_mut().0 = Some(world_position);
    }
}

pub struct InputPlugin;
impl Plugin for InputPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems((camera, brush, prev_mpos.after(brush)));
    }
}
