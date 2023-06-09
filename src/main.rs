use bevy::diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin};
use bevy::prelude::*;
use bevy_inspector_egui::quick::WorldInspectorPlugin;

use grid::*;
use input::*;

mod atom;
mod chunk;
mod consts;
mod grid;
mod grid_api;
mod input;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()))
        .add_plugin(WorldInspectorPlugin::new())
        //local plugins
        .add_plugin(GridPlugin)
        .add_plugin(InputPlugin)
        .add_startup_system(setup)
        //Frame on console
        .add_plugin(LogDiagnosticsPlugin::default())
        .add_plugin(FrameTimeDiagnosticsPlugin::default())
        .run();
}

fn setup(mut commands: Commands) {
    let mut camera = Camera2dBundle::default();
    camera.camera.hdr = true;

    commands.spawn(camera);
    commands.spawn(PreviousMousePos(None));
}
