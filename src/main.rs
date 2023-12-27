use bevy::diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin};
use bevy::prelude::*;
use bevy_inspector_egui::quick::WorldInspectorPlugin;

mod actors;
mod animation;
mod atom;
mod chunk;
mod chunk_group;
mod chunk_manager;
mod consts;
mod debug;
mod geom_tools;
mod manager_api;
mod player;
mod prelude {
    pub use crate::{
        actors::*, animation::*, atom::State, atom::*, chunk::*, chunk_group::*, chunk_manager::*,
        consts::*, debug::*, geom_tools::*, manager_api::*, player::*,
    };
    pub use bevy::math::{ivec2, ivec3, uvec2, uvec3, vec2, vec3};
    pub use bevy::prelude::*;
    pub use std::collections::{HashMap, HashSet};
}

use crate::animation::AnimationPlugin;
use prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()))
        .add_plugins(WorldInspectorPlugin::new())
        //local plugins
        .add_plugins((
            ChunkManagerPlugin,
            DebugPlugin,
            ActorsPlugin,
            PlayerPlugin,
            AnimationPlugin,
        ))
        .add_systems(Startup, setup)
        //Frame on console
        .add_plugins((LogDiagnosticsPlugin::default(), FrameTimeDiagnosticsPlugin))
        .run();
}

fn setup(mut commands: Commands) {
    let mut camera = Camera2dBundle::default();
    camera.camera.hdr = true;
    //camera.transform.scale.x = 0.67;
    //camera.transform.scale.y = 0.67;

    commands.spawn(camera);
    commands.spawn(PreviousMousePos(None));
}
