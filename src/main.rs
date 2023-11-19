use bevy::diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin};
use bevy::prelude::*;
use bevy_inspector_egui::quick::WorldInspectorPlugin;

mod actors;
mod atom;
mod chunk;
mod chunk_group;
mod chunk_manager;
mod consts;
mod debug;
mod geom_tools;
mod input;
mod manager_api;
mod prelude {
    pub use crate::{
        actors::*, atom::State, atom::*, chunk::*, chunk_group::*, chunk_manager::*, consts::*,
        debug::*, geom_tools::*, input::*, manager_api::*,
    };
    pub use bevy::prelude::*;
}

use prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()))
        .add_plugins(WorldInspectorPlugin::new())
        //local plugins
        .add_plugins((ChunkManagerPlugin, InputPlugin, DebugPlugin))
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
