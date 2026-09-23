use serde::Deserialize;
use std::{fs, path::PathBuf};

const MANIFEST_URL: &str = "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";

#[derive(Clone, Debug, Deserialize)]
pub struct GameVersion {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(rename = "releaseTime")]
    pub release_time: String,
}

#[derive(Deserialize)]
struct MojangManifest { versions: Vec<GameVersion> }

#[derive(Default)]
pub struct VersionCatalog { pub versions: Vec<GameVersion> }

impl VersionCatalog {
    fn cache_path() -> Option<PathBuf> {
        dirs::data_local_dir().map(|dir| dir.join("Atlas").join("cache").join("versions.json"))
    }

    pub fn load() -> Self {
        let Some(path) = Self::cache_path() else { return Self::default(); };
        let Ok(bytes) = fs::read(path) else { return Self::default(); };
        serde_json::from_slice::<MojangManifest>(&bytes).map(|m| Self { versions: m.versions }).unwrap_or_default()
    }

    pub fn fetch() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let response = reqwest::blocking::Client::builder().user_agent(concat!("Atlas/", env!("CARGO_PKG_VERSION"))).build()?.get(MANIFEST_URL).send()?.error_for_status()?;
        let bytes = response.bytes()?;
        let manifest = serde_json::from_slice::<MojangManifest>(&bytes)?;
        if let Some(path) = Self::cache_path() {
            if let Some(parent) = path.parent() { fs::create_dir_all(parent)?; }
            let _ = fs::write(path, &bytes);
        }
        Ok(Self { versions: manifest.versions })
    }

    pub fn replace(&mut self, versions: Vec<GameVersion>) { self.versions = versions; }
    pub fn all_versions(&self) -> impl Iterator<Item = &GameVersion> { self.versions.iter() }
}
