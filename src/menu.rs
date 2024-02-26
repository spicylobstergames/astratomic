use crate::prelude::*;
use bevy::app::AppExit;

#[derive(Component)]
pub struct MenuUI;

#[derive(Component)]
enum ButtonType {
    Start,
    Quit,
}

#[derive(Component)]
pub struct Background(pub Handle<Image>);

fn spawn_menu_buttons(mut commands: Commands, asset_server: Res<AssetServer>) {
    let background = asset_server.load("images/menu_background.png");

    commands.spawn((
        SpriteBundle {
            texture: background.clone(),
            ..Default::default()
        },
        Background(background),
        MenuUI,
    ));

    let button_style: Style = Style {
        width: Val::Px(150.0),
        height: Val::Px(65.0),
        border: UiRect::all(Val::Px(5.0)),
        justify_content: JustifyContent::Center,
        align_items: AlignItems::Center,
        ..default()
    };

    let ui_style = Style {
        width: Val::Percent(100.0),
        height: Val::Percent(100.0),
        align_items: AlignItems::Center,
        justify_content: JustifyContent::Center,
        flex_direction: FlexDirection::Column,
        row_gap: Val::Px(15.),
        ..default()
    };

    let text_style = TextStyle {
        font_size: 40.0,
        color: Color::rgb(0.9, 0.9, 0.9),
        ..Default::default()
    };

    commands
        .spawn(NodeBundle {
            style: ui_style,
            ..default()
        })
        .insert(MenuUI)
        .with_children(|parent| {
            //Start
            parent
                .spawn(ButtonBundle {
                    style: button_style.clone(),
                    border_color: BorderColor(Color::BLACK),
                    background_color: NORMAL_BUTTON.into(),
                    ..default()
                })
                .insert(ButtonType::Start)
                .with_children(|parent| {
                    parent.spawn(TextBundle::from_section("Start", text_style.clone()));
                });

            //Quit
            parent
                .spawn(ButtonBundle {
                    style: button_style,
                    border_color: BorderColor(Color::BLACK),
                    background_color: NORMAL_BUTTON.into(),
                    ..default()
                })
                .insert(ButtonType::Quit)
                .with_children(|parent| {
                    parent.spawn(TextBundle::from_section("Quit", text_style));
                });
        });
}

fn background_system(
    mut background: Query<(&mut Transform, &Background)>,
    images: Res<Assets<Image>>,
    window: Query<&Window>,
) {
    let (mut transform, handle) = background.single_mut();
    let Some(image) = images.get(handle.0.clone()) else {
        return;
    };
    let window = window.single();

    let scale =
        (window.width() / image.width() as f32).max(window.height() / image.height() as f32);

    transform.scale.x = scale * 0.23;
    transform.scale.y = scale * 0.23;
}

fn button_system(
    mut interaction_query: Query<
        (
            &Interaction,
            &mut BackgroundColor,
            &mut BorderColor,
            &ButtonType,
        ),
        (Changed<Interaction>, With<Button>),
    >,
    mut next_state: ResMut<NextState<GameState>>,
    mut exit: EventWriter<AppExit>,
) {
    for (interaction, mut color, mut border_color, button_type) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => {
                *color = PRESSED_BUTTON.into();
                border_color.0 = Color::RED;

                match *button_type {
                    ButtonType::Start => next_state.set(GameState::Game),
                    ButtonType::Quit => {
                        exit.send(AppExit);
                    }
                }
            }
            Interaction::Hovered => {
                *color = HOVERED_BUTTON.into();
                border_color.0 = Color::WHITE;
            }
            Interaction::None => {
                *color = NORMAL_BUTTON.into();
                border_color.0 = Color::BLACK;
            }
        }
    }
}

pub fn cleanup_menu(mut commands: Commands, menu_ui: Query<Entity, With<MenuUI>>) {
    for ent in menu_ui.iter() {
        commands.entity(ent).despawn_recursive()
    }
}

pub struct MenuPlugin;
impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (button_system, background_system).run_if(in_state(GameState::Menu)),
        )
        .add_systems(OnEnter(GameState::Menu), spawn_menu_buttons)
        .add_systems(OnExit(GameState::Menu), cleanup_menu);
    }
}
