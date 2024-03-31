//! emulation of sfxr using fundsp
use crate::fundsp_kira::Machine;
use bevy::{
    asset::{io::Reader, AssetLoader, AsyncReadExt, LoadContext},
    prelude::*,
    utils::BoxedFuture,
};
use fundsp::hacker32::*;

pub struct SfxrPlugin;

impl Plugin for SfxrPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset_loader::<SfxrLoader>();
    }
}

fn default_amp() -> f32 {
    1.0
}

#[derive(serde::Deserialize, serde::Serialize, Debug, Clone, Copy)]
pub enum Sfxr {
    PinkExp {
        #[serde(default = "default_amp")]
        amp: f32,
        f: f32,
    },
    PinkExpStereo {
        #[serde(default = "default_amp")]
        amp: f32,
        f: f32,
    },
}

impl From<Sfxr> for Machine {
    fn from(sfxr: Sfxr) -> Self {
        match sfxr {
            Sfxr::PinkExp { amp, f } => {
                Machine::new(pink() * amp.min(1.0) * envelope(move |t| exp(-f * t)))
            }
            Sfxr::PinkExpStereo { amp, f } => {
                let envelope = envelope(move |t| exp(-f * t)) * amp.min(1.0);
                Machine::new((envelope.clone() * pink()) | (envelope * pink()))
            }
        }
        .with_userdata(sfxr)
    }
}

#[derive(Default)]
struct SfxrLoader;

#[non_exhaustive]
#[derive(thiserror::Error, Debug)]
enum SfxrLoaderError {
    /// An [IO](std::io) Error
    #[error("Could not load asset: {0}")]
    Io(#[from] std::io::Error),
    /// A [RON](ron) Error
    #[error("Could not parse RON: {0}")]
    RonSpannedError(#[from] ron::error::SpannedError),
}

impl AssetLoader for SfxrLoader {
    type Asset = Machine;
    type Settings = ();
    type Error = SfxrLoaderError;
    fn load<'a>(
        &'a self,
        reader: &'a mut Reader,
        _settings: &'a Self::Settings,
        _load_context: &'a mut LoadContext,
    ) -> BoxedFuture<'a, Result<Self::Asset, Self::Error>> {
        Box::pin(async move {
            let mut bytes = Vec::new();
            reader.read_to_end(&mut bytes).await?;
            let sfxr = ron::de::from_bytes::<Sfxr>(&bytes)?;
            Ok(sfxr.into())
        })
    }

    fn extensions(&self) -> &[&str] {
        &["sfxr.ron"]
    }
}
