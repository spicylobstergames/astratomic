use bevy::prelude::*;

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
mod materials;
mod particles;
mod player;
mod prelude {
    pub use crate::{
        actors::*, animation::*, atom::*, chunk::*, chunk_group::*, chunk_manager::*, consts::*,
        debug::*, geom_tools::*, manager_api::*, materials::*, particles::*, player::*,
    };
    pub use bevy::input::mouse::MouseScrollUnit;
    pub use bevy::input::mouse::MouseWheel;
    pub use bevy::math::{ivec2, uvec2, vec2, vec3};
    pub use bevy::prelude::*;
    pub use bevy::tasks::*;
    pub use bevy_async_task::*;

    pub use serde::{Deserialize, Serialize};
    pub use serde_big_array::BigArray;

    pub use std::collections::{HashMap, HashSet};
    pub use std::env;
    pub use std::fs::File;
    pub use std::io::Write;
    pub use std::io::{BufReader, BufWriter};

    pub use crate::materials::Material;
}

use prelude::*;

fn main() {
    let args: Vec<_> = env::args().collect();

    let mut app = App::new();

    app.add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()))
        //local plugins
        .add_plugins((
            ChunkManagerPlugin,
            ActorsPlugin,
            PlayerPlugin,
            animation::AnimationPlugin,
            ParticlesPlugin,
            MaterialsPlugin,
        ))
        .add_systems(Startup, setup_camera);

    if args.contains(&"-d".to_string()) || args.contains(&"--debug".to_string()) {
        app.add_plugins(DebugPlugin);
    }

    app.run();
}

fn setup_camera(mut commands: Commands) {
    let mut camera = Camera2dBundle::default();
    camera.camera.hdr = true;
    camera.transform.scale.x = 0.23;
    camera.transform.scale.y = 0.23;

    commands.spawn(camera);
}
