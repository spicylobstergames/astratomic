use bevy::prelude::Color;
use bevy_rapier2d::geometry::Group;

// Chunk Length consts
// Chunk length MUST be divisible by 4
pub const CHUNK_LENGHT: usize = 64;
pub const HALF_CHUNK_LENGHT: usize = CHUNK_LENGHT / 2;

pub const CHUNK_LEN: usize = CHUNK_LENGHT * CHUNK_LENGHT;
pub const HALF_CHUNK_LEN: usize = CHUNK_LEN / 2;
pub const QUARTER_CHUNK_LEN: usize = CHUNK_LEN / 4;

// Actor consts

pub const UP_WALK_HEIGHT: usize = 3;
pub const DOWN_WALK_HEIGHT: usize = 4;

// Player consts
pub const FUEL_MAX: f32 = 50.;
pub const FUEL_REGEN: f32 = 1.;
pub const FUEL_COMSUMPTON: f32 = 0.48;
pub const JETPACK_FORCE: f32 = 1.5;
pub const JETPACK_MAX: f32 = 3.;

pub const JUMP_MAG: f32 = 9.;
pub const PRESSED_JUMP_MAG: f32 = 0.6;
pub const TIME_JUMP_PRESSED: f64 = 0.8;
pub const RUN_SPEED: f32 = 2.5;

pub const TOOL_DISTANCE: f32 = 32.;
pub const TOOL_RANGE: f32 = 16.;

pub const ZOOM_LOWER_BOUND: f32 = 0.15;
pub const ZOOM_UPPER_BOUND: f32 = 0.30;

// Engine consts

//This was a "scale" const for the atoms, but we can just zoom in, so it was removed
//Made the code less verbose and simpler, we can readd if it makes sense
//pub const ATOM_SIZE: usize = 3;

pub const GRAVITY: u8 = 1;
pub const TERM_VEL: u8 = 10;
pub const FRAMES_SLEEP: u8 = 1;
//Has to be even
pub const LOAD_WIDTH: i32 = 32;
pub const LOAD_HEIGHT: i32 = 18;

pub const _CAMERA_SPEED: f32 = 10.;

//Layers
pub const PLAYER_LAYER: f32 = 1.;
pub const PARTICLE_LAYER: f32 = 10.;
pub const AUTOMATA_LAYER: f32 = 100.;
pub const RIGIDBODY_LAYER: f32 = 1000.;

//Rapier2d collision groups
pub const RIGIDBODY_GROUP: Group = Group::GROUP_1;
//pub const WORLD_GROUP: Group = Group::GROUP_2;
pub const ACTOR_GROUP: Group = Group::GROUP_3;

//Buttons
pub const NORMAL_BUTTON: Color = Color::rgb(0.15, 0.15, 0.15);
pub const HOVERED_BUTTON: Color = Color::rgb(0.25, 0.25, 0.25);
pub const PRESSED_BUTTON: Color = Color::rgb(0.35, 0.75, 0.35);

//How much the rigidbody needs to be submerged to totally cut of gravity
//pub const STOP_RATE: f32 = 0.05;
