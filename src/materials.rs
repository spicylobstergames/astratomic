use crate::prelude::*;

use bevy::utils::thiserror;
use bevy::{
    asset::{io::Reader, AssetLoader, AsyncReadExt, LoadContext},
    prelude::*,
    reflect::TypePath,
    utils::BoxedFuture,
};
use serde::Deserialize;
use thiserror::Error;

#[derive(Default, Debug, Deserialize, PartialEq, Clone, Copy)]
pub enum Material {
    Solid,
    Powder {
        inertial_resistance: f32,
    },
    Liquid {
        flow: u8,
    },
    Gas,
    Object,
    #[default]
    Void,
}

impl Material {
    pub fn is_liquid(&self) -> bool {
        matches!(self, Material::Liquid { .. })
    }

    pub fn is_void(&self) -> bool {
        matches!(self, Material::Void)
    }

    pub fn is_object(&self) -> bool {
        matches!(self, Material::Object)
    }

    pub fn is_powder(&self) -> bool {
        matches!(self, Material::Powder { .. })
    }

    pub fn is_solid(&self) -> bool {
        matches!(self, Material::Solid)
    }
}

#[derive(Asset, TypePath, Debug, Deserialize)]
pub struct Materials(pub Vec<Material>);

impl Materials {
    pub fn get_from_atom(&self, atom: &Atom) -> &Material {
        &self.0[atom.id as usize]
    }

    pub fn get_from_id(&self, id: u8) -> &Material {
        &self.0[id as usize]
    }
}

impl std::ops::Index<&Atom> for Materials {
    type Output = Material;
    #[track_caller]
    fn index(&self, atom: &Atom) -> &Self::Output {
        self.get_from_atom(atom)
    }
}

impl std::ops::Index<u8> for Materials {
    type Output = Material;
    #[track_caller]
    fn index(&self, id: u8) -> &Self::Output {
        self.get_from_id(id)
    }
}

//Asset stuff

#[derive(Resource, Default)]
pub struct MaterialsHandle(pub Handle<Materials>);

#[derive(Default)]
pub struct MaterialsLoader;

#[non_exhaustive]
#[derive(Debug, Error)]
pub enum MaterialsLoaderError {
    /// An [IO](std::io) Error
    #[error("Could not load asset: {0}")]
    Io(#[from] std::io::Error),
    /// A [RON](ron) Error
    #[error("Could not parse RON: {0}")]
    RonSpannedError(#[from] ron::error::SpannedError),
}

impl AssetLoader for MaterialsLoader {
    type Asset = Materials;
    type Settings = ();
    type Error = MaterialsLoaderError;
    fn load<'a>(
        &'a self,
        reader: &'a mut Reader,
        _settings: &'a (),
        _load_context: &'a mut LoadContext,
    ) -> BoxedFuture<'a, Result<Self::Asset, Self::Error>> {
        Box::pin(async move {
            let mut bytes = Vec::new();
            reader.read_to_end(&mut bytes).await?;
            let custom_asset = ron::de::from_bytes::<Materials>(&bytes)?;
            Ok(custom_asset)
        })
    }

    fn extensions(&self) -> &[&str] {
        &["ron"]
    }
}

pub fn setup(mut materials_handle: ResMut<MaterialsHandle>, asset_server: Res<AssetServer>) {
    materials_handle.0 = asset_server.load("atoms.ron");
}

pub struct MaterialsPlugin;
impl Plugin for MaterialsPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<Materials>()
            .init_resource::<MaterialsHandle>()
            .init_asset_loader::<MaterialsLoader>()
            .add_systems(Startup, setup);
    }
}
