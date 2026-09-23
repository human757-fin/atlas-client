use serde::Deserialize;
use sha1::{Digest, Sha1};
use std::{collections::HashSet, fs, path::Path};

const API: &str = "https://api.modrinth.com/v2";

#[derive(Clone, Copy)]
pub struct ModOption { pub slug: &'static str, pub name: &'static str, pub description: &'static str, pub group: &'static str }

pub const MODS: &[ModOption] = &[
    ModOption { slug: "sodium", name: "Sodium", description: "A faster renderer with smoother frame rates.", group: "PERFORMANCE" },
    ModOption { slug: "lithium", name: "Lithium", description: "Optimizes game logic without changing how the game plays.", group: "PERFORMANCE" },
    ModOption { slug: "ferrite-core", name: "FerriteCore", description: "Reduces memory use, especially in modded worlds.", group: "PERFORMANCE" },
    ModOption { slug: "immediatelyfast", name: "ImmediatelyFast", description: "Speeds up common rendering tasks.", group: "PERFORMANCE" },
    ModOption { slug: "entityculling", name: "Entity Culling", description: "Skips rendering entities hidden behind blocks.", group: "PERFORMANCE" },
    ModOption { slug: "modmenu", name: "Mod Menu", description: "Browse installed mods and open their settings.", group: "QUALITY OF LIFE" },
    ModOption { slug: "appleskin", name: "AppleSkin", description: "See hunger and saturation values while playing.", group: "QUALITY OF LIFE" },
    ModOption { slug: "betterf3", name: "BetterF3", description: "A clearer, configurable debug screen.", group: "QUALITY OF LIFE" },
    ModOption { slug: "shulkerboxtooltip", name: "Shulker Box Tooltip", description: "Preview container contents from your inventory.", group: "QUALITY OF LIFE" },
    ModOption { slug: "zoomify", name: "Zoomify", description: "Add a smooth, configurable zoom key.", group: "QUALITY OF LIFE" },
];

#[derive(Clone, Debug, Deserialize)]
struct ModVersion {
    id: String,
    #[serde(default)] name: String,
    #[serde(default)] dependencies: Vec<Dependency>,
    #[serde(default)] files: Vec<ModFile>,
}

#[derive(Clone, Debug, Deserialize)]
struct Dependency { project_id: Option<String>, version_id: Option<String>, dependency_type: String }

#[derive(Clone, Debug, Deserialize)]
struct ModFile { filename: String, url: String, primary: bool, hashes: Hashes }
#[derive(Clone, Debug, Deserialize)]
struct Hashes { sha1: Option<String> }

fn api_client() -> Result<reqwest::blocking::Client, Box<dyn std::error::Error + Send + Sync>> {
    Ok(reqwest::blocking::Client::builder().user_agent(concat!("Atlas/", env!("CARGO_PKG_VERSION"), " (Minecraft launcher)" )).build()?)
}

fn project_version(client: &reqwest::blocking::Client, project: &str, game_version: &str) -> Result<Option<ModVersion>, Box<dyn std::error::Error + Send + Sync>> {
    let versions: Vec<ModVersion> = client.get(format!("{API}/project/{project}/version"))
        .query(&[("loaders", r#"["fabric"]"#), ("game_versions", &format!(r#"["{game_version}"]"#)), ("include_changelog", "false")])
        .send()?.error_for_status()?.json()?;
    Ok(versions.into_iter().find(|v| !v.files.is_empty()))
}

fn version_by_id(client: &reqwest::blocking::Client, id: &str) -> Result<ModVersion, Box<dyn std::error::Error + Send + Sync>> {
    Ok(client.get(format!("{API}/version/{id}")).send()?.error_for_status()?.json()?)
}

fn safe_file_name(filename: &str) -> Option<&str> {
    let path = Path::new(filename);
    if path.components().count() == 1 && path.file_name()?.to_str()? == filename && filename.ends_with(".jar") { Some(filename) } else { None }
}

fn install_version(client: &reqwest::blocking::Client, version: &ModVersion, game_version: &str, mods_dir: &Path, done: &mut HashSet<String>, progress: &mut impl FnMut(String)) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if !done.insert(version.id.clone()) { return Ok(()); }
    for dependency in &version.dependencies {
        if dependency.dependency_type != "required" { continue; }
        let required = if let Some(version_id) = &dependency.version_id {
            version_by_id(client, version_id)?
        } else if let Some(project_id) = &dependency.project_id {
            project_version(client, project_id, game_version)?.ok_or_else(|| format!("Could not find required dependency {project_id}"))?
        } else { continue; };
        install_version(client, &required, game_version, mods_dir, done, progress)?;
    }
    for file in &version.files {
        if !file.primary && version.files.iter().any(|candidate| candidate.primary) { continue; }
        let Some(filename) = safe_file_name(&file.filename) else { return Err("Mod provider returned an unsafe filename".into()); };
        let target = mods_dir.join(filename);
        if target.exists() { break; }
        progress(format!("Downloading {}", version.name));
        let response = client.get(&file.url).send()?.error_for_status()?;
        let bytes = response.bytes()?;
        if let Some(expected) = &file.hashes.sha1 {
            let actual = Sha1::digest(&bytes).iter().map(|byte| format!("{byte:02x}")).collect::<String>();
            if !actual.eq_ignore_ascii_case(expected) { return Err(format!("Checksum verification failed for {filename}").into()); }
        }
        let temporary = target.with_extension("jar.part");
        fs::write(&temporary, &bytes)?;
        fs::rename(temporary, target)?;
        break;
    }
    Ok(())
}

pub fn install_selected(game_version: &str, instance_dir: &Path, selected: &[&'static str], mut progress: impl FnMut(String)) -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>> {
    let client = api_client()?;
    let mods_dir = instance_dir.join("mods");
    fs::create_dir_all(&mods_dir)?;
    let mut done = HashSet::new();
    let mut installed = Vec::new();
    for slug in selected {
        progress(format!("Finding {slug} for Minecraft {game_version}"));
        let version = project_version(&client, slug, game_version)?.ok_or_else(|| format!("{slug} has no Fabric release for Minecraft {game_version}"))?;
        install_version(&client, &version, game_version, &mods_dir, &mut done, &mut progress)?;
        installed.push((*slug).to_string());
    }
    Ok(installed)
}

pub fn installed_mod_names(instance_dir: &Path) -> HashSet<String> {
    fs::read_dir(instance_dir.join("mods")).ok().into_iter().flatten().flatten()
        .filter_map(|entry| entry.file_name().into_string().ok()).collect()
}
