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
    number: Option<u16>,
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
    slots: [Option<Slot>; 8],
    selected: usize,
}

impl Hotbar {
    pub fn new(slots: [Option<Slot>; 8]) -> Self {
        Self { slots, selected: 0 }
    }
}
#[derive(Component, Default)]
pub struct Inventory {
    hotbar: Hotbar,
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
                        width: Val::Px(25.),
                        height: Val::Px(25.),
                        margin: UiRect::all(Val::Px(20.)),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
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
                        Transform::from_scale(Vec3::new(3.2, 3.2, 0.)),
                    ));

                    if let Some(number) = slot.number {
                        let text_style = TextFont {
                            ..Default::default()
                        };
                        let text_color = TextColor(Color::srgb(0.9, 0.9, 0.9));

                        parent.with_child((Text::new(format!("{number}")), text_style, text_color));
                    }
                }
            }
        });
}

fn inv() {

}

pub struct PlayerInvPlugin;
impl Plugin for PlayerInvPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Game), inv_setup.after(manager_setup));
    }
}
