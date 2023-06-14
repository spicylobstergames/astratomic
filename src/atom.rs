#[derive(Clone, Copy, Default, PartialEq)]
pub struct Atom {
    pub color: [u8; 4],
    pub state: State,
    pub updated_at: f32,
    pub sim_velocity: u8,
    pub density: f32,
}

impl Atom {
    pub fn new() -> Self {
        Atom {
            color: [55, 55, 55, 255],
            state: State::Void,
            updated_at: 0.,
            sim_velocity: 0,
            density: 0.,
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
