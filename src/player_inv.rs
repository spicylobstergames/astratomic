use crate::prelude::*;
use bevy::render::{render_asset::RenderAssetUsages, render_resource::*};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum Item {
    Atom(Atom),
    SmartTool,
}

impl Item {
    pub fn same(&self, item: Item) -> bool {
        if let Item::Atom(atom) = self {
            if let Item::Atom(atom2) = item {
                return atom.id == atom2.id;
            }
        }
        *self == item
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Slot {
    pub item: Item,
    // If none it means item is not stackable
    pub number: Option<u16>,
}

impl Slot {
    pub fn atom_full(id: u8, materials: &Materials) -> Self {
        Self {
            item: Item::Atom(Atom::new(id, materials)),
            number: Some(100),
        }
    }

    pub fn smart_tool() -> Self {
        Self {
            item: Item::SmartTool,
            number: None,
        }
    }
}

#[derive(Resource, Default, Debug)]
pub struct Inventory {
    pub slots: [Option<Slot>; 32],
    pub selected: usize,
    slot_ents: [Option<Entity>; 32],
    showing: bool,
}

impl Inventory {
    //Returns the index of the new item if it wasn't on the inventory
    fn add(&mut self, item: Item) -> Option<usize> {
        //Search if we have the item on inventory
        for slot in self.slots.iter_mut().flatten() {
            if slot.item.same(item) {
                if let Some(number) = &mut slot.number {
                    if *number < u16::MAX {
                        *number += 1;
                        return None;
                    }
                }
            }
        }
        //If we don't, search the first empty slot
        for (i, slot) in self.slots.iter_mut().enumerate() {
            if slot.is_none() {
                *slot = Some(Slot {
                    item,
                    number: Some(1),
                });
                return Some(i);
            }
        }
        None
    }

    pub fn can_add(&self, item: Item) -> bool {
        for slot in self.slots {
            if let Some(slot) = slot {
                if slot.item.same(item) {
                    if let Some(number) = slot.number {
                        if number < u16::MAX {
                            return true;
                        }
                    }
                }
            } else {
                return true;
            }
        }
        false
    }

    pub fn new(
        slots: [Option<Slot>; 32],
        slot_ents: [Option<Entity>; 32],
        selected: usize,
    ) -> Self {
        Self {
            slots,
            selected,
            slot_ents,
            showing: false,
        }
    }

    fn remove_one(&mut self, index: usize) -> Option<usize> {
        if let Some(slot) = self.slots[index] {
            if slot.number.is_none() || slot.number.unwrap() <= 1 {
                self.slots[index] = None;
                return Some(index);
            } else if let Some(slot) = &mut self.slots[index] {
                slot.number = Some(slot.number.unwrap() - 1);
            }
        }
        None
    }

    fn remove_one_selected(&mut self) -> Option<usize> {
        self.remove_one(self.selected)
    }

    fn _full(&self) -> bool {
        for slot in self.slots {
            if slot.is_none() {
                return false;
            }
        }
        true
    }
}

impl Drop for Inventory {
    fn drop(&mut self) {
        let file = File::create("assets/world/inventory").unwrap();
        let mut buffered = BufWriter::new(file);
        bincode::serialize_into(&mut buffered, &(&self.slots, &self.selected)).unwrap();
    }
}

#[derive(Component)]
pub struct SlotUi(usize);

#[derive(Component)]
pub struct InvUi;

pub fn inv_setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    materials: (Res<Assets<Materials>>, Res<MaterialsHandle>),
    mut images: ResMut<Assets<Image>>,
    mut inv: ResMut<Inventory>,
) {
    let materials = materials.0.get(&materials.1 .0).unwrap();

    let (mut slots, selected): ([Option<Slot>; 32], usize);
    if let Ok(file) = File::open("assets/world/inventory") {
        let mut buffered = BufReader::new(file);
        (slots, selected) = bincode::deserialize_from(&mut buffered).unwrap();
    } else {
        (slots, selected) = ([None; 32], 0);
        slots[0] = Some(Slot::smart_tool());
        slots[1] = Some(Slot::atom_full(2, materials));
        slots[2] = Some(Slot::atom_full(3, materials));
        slots[3] = Some(Slot::atom_full(4, materials));
        slots[4] = Some(Slot::atom_full(5, materials));
        slots[5] = Some(Slot::atom_full(9, materials));

        let file = File::create("assets/world/inventory").unwrap();
        let mut buffered = BufWriter::new(file);
        bincode::serialize_into(&mut buffered, &(slots, selected)).unwrap();
    }

    let mut slot_ents = [None; 32];

    commands
        .spawn((Node {
            justify_self: JustifySelf::Center,
            display: Display::Grid,
            grid_template_columns: RepeatedGridTrack::flex(8, 1.0),
            grid_template_rows: RepeatedGridTrack::flex(4, 1.0),
            row_gap: Val::Px(35.),
            column_gap: Val::Px(35.),
            ..Default::default()
        },))
        .with_children(|parent| {
            for i in 0..32 {
                let mut parent = parent.spawn((
                    Node {
                        width: Val::Px(54.),
                        height: Val::Px(54.),
                        margin: UiRect::all(Val::Px(20.)),
                        align_items: AlignItems::Center,
                        //justify_content: JustifyContent::Center,
                        justify_self: JustifySelf::Center,
                        border: UiRect::all(Val::Percent(12.)),
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
                if i < 8 {
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
                } else {
                    parent.insert((InvUi, Visibility::Hidden));
                }
                slot_ents[i] = Some(parent.id());
            }
        });

    *inv = Inventory::new(slots, slot_ents, selected);
}

#[derive(Component)]
pub struct NumberUi(pub Entity);

pub fn spawn_numbers(
    mut commands: Commands,
    slots: Query<(&SlotUi, Entity)>,
    inventory: Res<Inventory>,
) {
    let text_style = TextFont {
        font_size: 11.,
        ..Default::default()
    };
    let text_color = TextColor(Color::srgb(0.9, 0.9, 0.9));

    for (index, ent) in slots.iter() {
        if let Some(slot) = inventory.slots[index.0] {
            if let Some(number) = slot.number {
                let mut ent = commands.spawn((
                    Text::new(format!("{number}")),
                    text_style.clone(),
                    text_color,
                    Node::DEFAULT,
                    Transform::from_xyz(0., 0., 100000.),
                    NumberUi(ent),
                ));
                if index.0 >= 8 {
                    ent.insert((InvUi, Visibility::Hidden));
                }
            }
        }
    }
}

pub fn update_numbers(
    mut commands: Commands,
    slots: Query<(&SlotUi, &GlobalTransform), Without<NumberUi>>,
    mut numbers: Query<(&mut GlobalTransform, &NumberUi, &mut Text, Entity), Without<SlotUi>>,
    inventory: Res<Inventory>,
) {
    for (mut gtransform, number_ui, mut text, ent) in numbers.iter_mut() {
        let (slot, slot_gtransform) = slots.get(number_ui.0).unwrap();
        if let Some(slot) = inventory.slots[slot.0] {
            text.0 = slot.number.unwrap().to_string();
            let v = slot_gtransform.clone().translation() + Vec3::new(0., 28., 500.);
            *gtransform = GlobalTransform::from_xyz(v.x, v.y, v.z);
        } else {
            commands.entity(ent).despawn_recursive();
        }
    }
}

#[derive(Event)]
pub enum ItemEvent {
    Add(Item),
    RemoveSelected,
}

pub fn item_events(
    mut commands: Commands,
    mut ev_items: EventReader<ItemEvent>,
    mut inv: ResMut<Inventory>,
    mut images: ResMut<Assets<Image>>,
    asset_server: Res<AssetServer>,
) {
    for ev in ev_items.read() {
        match ev {
            ItemEvent::Add(item) => {
                if let Some(index) = inv.add(*item) {
                    if let Some(ent) = inv.slot_ents[index] {
                        let image = match item {
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

                        commands.entity(ent).with_child((
                            ImageNode {
                                image,
                                ..Default::default()
                            },
                            Node {
                                width: Val::Percent(100.),

                                ..Default::default()
                            },
                        ));

                        let text_style = TextFont {
                            font_size: 11.,
                            ..Default::default()
                        };
                        let text_color = TextColor(Color::srgb(0.9, 0.9, 0.9));

                        if let Some(slot) = inv.slots[index] {
                            if let Some(number) = slot.number {
                                let mut ent = commands.spawn((
                                    Text::new(format!("{number}")),
                                    text_style.clone(),
                                    text_color,
                                    Node::DEFAULT,
                                    Transform::from_xyz(0., 0., 10.),
                                    NumberUi(ent),
                                ));
                                if !inv.showing && index >= 8 {
                                    ent.insert((Visibility::Hidden, InvUi));
                                } else if inv.showing && index >= 8 {
                                    ent.insert((Visibility::Visible, InvUi));
                                }
                            }
                        }
                    }
                }
            }
            ItemEvent::RemoveSelected => {
                if let Some(index) = inv.remove_one_selected() {
                    if let Some(ent) = inv.slot_ents[index] {
                        commands.entity(ent).despawn_descendants();
                    }
                }
            }
        }
    }
}

pub fn show_inventory(
    inputs: Res<Inputs>,
    mut inventory: ResMut<Inventory>,
    mut inv_ui: Query<&mut Visibility, With<InvUi>>,
    mut slots: Query<&mut Outline, With<SlotUi>>,
) {
    //Change Selected Outline
    let mut selected_outline = slots
        .get_mut(inventory.slot_ents[inventory.selected].unwrap())
        .unwrap();

    *selected_outline = Outline {
        color: Color::WHITE,
        width: Val::Px(9.),
        offset: Val::Px(6.),
    };

    //Toggle inventory open
    if !inventory.showing && inputs.inventory_toggle {
        inventory.showing = true;
        for mut vis in inv_ui.iter_mut() {
            *vis = Visibility::Visible;
        }
    } else if inputs.inventory_toggle && inventory.showing {
        inventory.showing = false;
        for mut vis in inv_ui.iter_mut() {
            *vis = Visibility::Hidden;
        }
    }
}

fn clear_selected(mut slots: Query<&mut Outline, With<SlotUi>>) {
    for mut outline in slots.iter_mut() {
        *outline = Outline {
            color: Color::linear_rgb(0.4, 0.4, 0.4),
            width: Val::Px(6.),
            offset: Val::Px(6.),
        };
    }
}

pub struct PlayerInvPlugin;
impl Plugin for PlayerInvPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            OnEnter(GameState::Game),
            (
                inv_setup.after(manager_setup),
                spawn_numbers.after(inv_setup),
            ),
        )
        .add_event::<ItemEvent>()
        .init_resource::<Inventory>()
        .add_systems(
            Update,
            (update_numbers, item_events, show_inventory).run_if(in_state(GameState::Game)),
        )
        .add_systems(PreUpdate, clear_selected.run_if(in_state(GameState::Game)));
    }
}
