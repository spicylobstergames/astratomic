use crate::prelude::*;
use bevy::render::{render_asset::RenderAssetUsages, render_resource::*};

#[derive(Clone, Copy)]
pub enum Item {
    Atom(Atom),
    SmartTool,
}

#[derive(Clone, Copy)]
pub struct Slot {
    item: Item,
    // If none it means item is not stackable
    pub number: Option<u16>,
}

impl Slot {
    pub fn atom_full(id: u8, materials: &Materials) -> Self {
        Self {
            item: Item::Atom(Atom::new(id, materials)),
            number: Some(u16::MAX),
        }
    }

    pub fn smart_tool() -> Self {
        Self {
            item: Item::SmartTool,
            number: None,
        }
    }
}

#[derive(Default)]
pub struct Hotbar {
    pub slots: [Option<Slot>; 8],
    selected: usize,
}

impl Hotbar {
    pub fn new(slots: [Option<Slot>; 8]) -> Self {
        Self { slots, selected: 0 }
    }
}
#[derive(Component, Default)]
pub struct Inventory {
    pub hotbar: Hotbar,
    slots: [Option<Slot>; 24],
}

impl Inventory {
    pub fn from_hotbar(hotbar: Hotbar) -> Self {
        Self {
            hotbar,
            ..Default::default()
        }
    }
}

#[derive(Component)]
pub struct SlotUi(usize);

pub fn inv_setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    materials: (Res<Assets<Materials>>, Res<MaterialsHandle>),
    mut images: ResMut<Assets<Image>>,
) {
    let materials = materials.0.get(&materials.1 .0).unwrap();

    let mut slots = [None; 8];
    slots[0] = Some(Slot::smart_tool());
    slots[1] = Some(Slot::atom_full(2, materials));
    slots[2] = Some(Slot::atom_full(3, materials));
    slots[3] = Some(Slot::atom_full(4, materials));
    slots[4] = Some(Slot::atom_full(5, materials));
    slots[5] = Some(Slot::atom_full(9, materials));

    commands
        .spawn((
            Inventory::from_hotbar(Hotbar::new(slots)),
            Node {
                justify_self: JustifySelf::Center,
                ..Default::default()
            },
        ))
        .with_children(|parent| {
            for i in 0..8 {
                let mut parent = parent.spawn((
                    Node {
                        width: Val::Px(42.),
                        height: Val::Px(42.),
                        margin: UiRect::all(Val::Px(16.)),
                        align_items: AlignItems::Center,
                        //justify_content: JustifyContent::Center,
                        justify_self: JustifySelf::Center,
                        border: UiRect::all(Val::Percent(1.)),
                        ..default()
                    },
                    BackgroundColor(Color::NONE),
                    BorderColor(Color::NONE),
                    Outline {
                        width: Val::Px(6.),
                        offset: Val::Px(6.),
                        color: Color::WHITE,
                    },
                    SlotUi(i),
                ));
                if let Some(slot) = slots[i] {
                    let image = match slot.item {
                        Item::Atom(atom) => images.add(Image::new(
                            Extent3d {
                                height: 1,
                                width: 1,
                                ..Default::default()
                            },
                            TextureDimension::D2,
                            atom.color.into(),
                            TextureFormat::Rgba8UnormSrgb,
                            RenderAssetUsages::RENDER_WORLD,
                        )),
                        Item::SmartTool => asset_server.load("player/player_tool.png"),
                    };

                    parent.with_child((
                        ImageNode {
                            image,
                            ..Default::default()
                        },
                        Node {
                            width: Val::Percent(100.),

                            ..Default::default()
                        },
                    ));
                }
            }
        });
}

#[derive(Component)]
pub struct NumberUi(pub Entity);

pub fn spawn_numbers(
    mut commands: Commands,
    slots: Query<(&SlotUi, Entity)>,
    inventory: Query<&Inventory>,
) {
    let inventory = inventory.single();

    let text_style = TextFont {
        font_size: 11.,
        ..Default::default()
    };
    let text_color = TextColor(Color::srgb(0.9, 0.9, 0.9));

    for (slot, ent) in slots.iter() {
        if let Some(slot) = inventory.hotbar.slots[slot.0] {
            if let Some(number) = slot.number {
                commands.spawn((
                    Text::new(format!("{number}")),
                    text_style.clone(),
                    text_color,
                    Node::DEFAULT,
                    Transform::from_xyz(0., 0., 10.),
                    NumberUi(ent),
                ));
            }
        }
    }
}

pub fn update_numbers(
    slots: Query<&GlobalTransform, (With<SlotUi>, Without<NumberUi>)>,
    mut numbers: Query<(&mut GlobalTransform, &NumberUi), Without<SlotUi>>,
) {
    for (mut gtransform, number_ui) in numbers.iter_mut() {
        let v = slots.get(number_ui.0).unwrap().clone().translation() + Vec3::new(0., 22., 10.);
        *gtransform = GlobalTransform::from_xyz(v.x, v.y, v.z);
    }
}

pub fn inv() {}

pub struct PlayerInvPlugin;
impl Plugin for PlayerInvPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            OnEnter(GameState::Game),
            (
                inv_setup.after(manager_setup),
                spawn_numbers.after(inv_setup),
            ),
        ).add_systems(Update, update_numbers.run_if(in_state(GameState::Game)));
    }
}
