use bevy::prelude::IVec2;

// Make smaller
#[derive(Clone, Copy, Default, PartialEq)]
pub struct Atom {
    pub color: [u8; 4],
    pub state: State,
    pub updated_at: f32,
    pub fall_speed: u8,
    // Used when thrown up, etc
    pub velocity: Option<IVec2>,
    // Frames idle
    pub f_idle: u8,
}

impl Atom {
    pub fn new() -> Self {
        Atom {
            color: [55, 55, 55, 255],
            state: State::Void,
            updated_at: 0.,
            fall_speed: 0,
            velocity: None,
            f_idle: 0,
        }
    }
}

#[derive(Default, Clone, Copy, PartialEq)]
pub enum State {
    Solid,
    Powder,
    Liquid,
    Gas,
    #[default]
    Void,
}
