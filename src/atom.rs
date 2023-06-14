#[derive(Clone, Copy, Default, PartialEq)]
pub struct Atom {
    pub color: [u8; 4],
    pub state: State,
    pub updated_at: f32,
    pub fall_velocity: u8,
}

impl Atom {
    pub fn new() -> Self {
        Atom {
            color: [55, 55, 55, 255],
            state: State::Void,
            updated_at: 0.,
            fall_velocity: 0,
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
