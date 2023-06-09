#[derive(Clone, Copy, Default, PartialEq)]
pub struct Atom {
    pub color: [u8; 4],
    pub state: State,
    pub updated_at: f32,
    pub free_falling: bool,
}

impl Atom {
    pub fn new() -> Self {
        Atom {
            color: [55, 55, 55, 255],
            state: State::Void,
            updated_at: 0.,
            free_falling: false,
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
