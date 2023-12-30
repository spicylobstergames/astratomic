// Chunk lenght MUST be divisible by 4
pub const CHUNK_LENGHT: usize = 64;
pub const HALF_CHUNK_LENGHT: usize = CHUNK_LENGHT / 2;

pub const CHUNK_LEN: usize = CHUNK_LENGHT * CHUNK_LENGHT;
pub const HALF_CHUNK_LEN: usize = CHUNK_LEN / 2;
pub const QUARTER_CHUNK_LEN: usize = CHUNK_LEN / 4;

pub const UP_WALK_HEIGHT: usize = 3;
pub const DOWN_WALK_HEIGHT: usize = 6;
pub const ATOM_SIZE: usize = 3;
pub const _CAMERA_SPEED: f32 = 10.;
pub const GRAVITY: u8 = 1;
pub const TERM_VEL: u8 = 10;
pub const FRAMES_SLEEP: u8 = 4;
pub const CHUNKS_WIDTH: usize = 8;
pub const CHUNKS_HEIGHT: usize = 6;
