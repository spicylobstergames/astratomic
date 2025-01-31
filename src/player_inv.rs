use crate::prelude::*;
use bevy::color::palettes::css::*;
use bevy::color::palettes::tailwind::*;

#[derive(Component)]
pub struct HotBar;

pub fn inv_setup(mut commands: Commands) {
    commands
        .spawn((
            HotBar,
            Node {
                justify_self: JustifySelf::Center,
                ..Default::default()
            },
        ))
        .with_children(|parent| {
            for _ in 0..10 {
                parent.spawn((
                    Node {
                        width: Val::Px(25.),
                        height: Val::Px(25.),
                        margin: UiRect::all(Val::Px(20.)),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        justify_self: JustifySelf::Center,
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
            }
        });
}

pub struct PlayerInvPlugin;
impl Plugin for PlayerInvPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Game), inv_setup);
    }
}
