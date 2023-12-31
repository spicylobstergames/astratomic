// Chunk Lenght consts
// Chunk lenght MUST be divisible by 4
pub const CHUNK_LENGHT: usize = 64;
pub const HALF_CHUNK_LENGHT: usize = CHUNK_LENGHT / 2;

pub const CHUNK_LEN: usize = CHUNK_LENGHT * CHUNK_LENGHT;
pub const HALF_CHUNK_LEN: usize = CHUNK_LEN / 2;
pub const QUARTER_CHUNK_LEN: usize = CHUNK_LEN / 4;

// Actor consts

pub const UP_WALK_HEIGHT: usize = 3;
pub const DOWN_WALK_HEIGHT: usize = 6;

// Player consts
pub const FUEL_MAX: f32 = 50.;
pub const FUEL_REGEN: f32 = 1.;
pub const FUEL_COMSUMPTON: f32 = 0.48;
pub const JETPACK_FORCE: f32 = 1.5;
pub const JETPACK_MAX: f32 = 3.;

pub const JUMP_MAG: f32 = 13.;
pub const RUN_SPEED: f32 = 5.;

pub const TOOL_DISTANCE: f32 = 32.;
pub const TOOL_RANGE: f32 = 12.;

// Engine consts
pub const ATOM_SIZE: usize = 3;
pub const GRAVITY: u8 = 1;
pub const TERM_VEL: u8 = 10;
pub const FRAMES_SLEEP: u8 = 4;
pub const CHUNKS_WIDTH: usize = 32;
pub const CHUNKS_HEIGHT: usize = 32;

pub const _CAMERA_SPEED: f32 = 10.;
