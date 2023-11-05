use bevy::diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin};
use bevy::prelude::*;
//use bevy_inspector_egui::quick::WorldInspectorPlugin;

use grid::*;
use input::*;

mod atom;
mod chunk;
mod consts;
mod geom_tools;
mod grid;
mod grid_api;
mod input;
mod player;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()))
        //.add_plugins(WorldInspectorPlugin::new())
        //local plugins
        .add_plugins((GridPlugin, InputPlugin))
        .add_systems(Startup, setup)
        //Frame on console
        .add_plugins((LogDiagnosticsPlugin::default(), FrameTimeDiagnosticsPlugin))
        .run();
}

fn setup(mut commands: Commands) {
    let mut camera = Camera2dBundle::default();
    camera.camera.hdr = true;

    commands.spawn(camera);
    commands.spawn(PreviousMousePos(None));
}
