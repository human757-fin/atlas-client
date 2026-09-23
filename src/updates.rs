use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{fs, process::Command};
#[cfg(target_os = "windows")]
use std::path::PathBuf;

const API: &str = "https://api.github.com/repos/human757-fin/atlas-client/releases?per_page=20";

#[derive(Clone, Debug)]
pub struct Release {
    pub version: semver::Version,
    pub tag: String,
    pub notes: String,
    pub assets: Vec<Asset>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Asset { pub name: String, pub browser_download_url: String }

#[derive(Deserialize)]
struct GithubRelease {
    tag_name: String,
    draft: bool,
    body: Option<String>,
    assets: Vec<Asset>,
}

pub fn check() -> Result<Option<Release>, String> {
    let client = reqwest::blocking::Client::builder().user_agent("AtlasClient/".to_owned() + env!("CARGO_PKG_VERSION")).build().map_err(|e| e.to_string())?;
    let releases: Vec<GithubRelease> = client.get(API).send().map_err(|e| format!("Could not reach GitHub: {e}"))?.error_for_status().map_err(|e| e.to_string())?.json().map_err(|e| format!("Could not read release list: {e}"))?;
    let current = semver::Version::parse(env!("CARGO_PKG_VERSION")).map_err(|e| e.to_string())?;
    Ok(releases.into_iter().filter(|r| !r.draft).find_map(|release| {
        let version = semver::Version::parse(release.tag_name.trim_start_matches('v')).ok()?;
        (version > current).then_some(Release { version, tag: release.tag_name, notes: release.body.unwrap_or_default(), assets: release.assets })
    }))
}

fn get(client: &reqwest::blocking::Client, url: &str) -> Result<Vec<u8>, String> {
    client.get(url).send().map_err(|e| e.to_string())?.error_for_status().map_err(|e| e.to_string())?.bytes().map(|b| b.to_vec()).map_err(|e| e.to_string())
}

fn get_with_progress(client: &reqwest::blocking::Client, url: &str, mut progress: impl FnMut(u64, u64)) -> Result<Vec<u8>, String> {
    use std::io::Read;
    let mut response = client.get(url).send().map_err(|e| e.to_string())?.error_for_status().map_err(|e| e.to_string())?;
    let total = response.content_length().unwrap_or(0);
    let mut bytes = Vec::with_capacity(total.min(32 * 1024 * 1024) as usize);
    let mut chunk = [0_u8; 64 * 1024];
    let mut downloaded = 0_u64;
    loop {
        let count = response.read(&mut chunk).map_err(|e| e.to_string())?;
        if count == 0 { break; }
        bytes.extend_from_slice(&chunk[..count]);
        downloaded += count as u64;
        progress(downloaded, total);
    }
    Ok(bytes)
}

fn asset<'a>(release: &'a Release, name: &str) -> Result<&'a Asset, String> {
    release.assets.iter().find(|a| a.name == name).ok_or_else(|| format!("Release {} is missing {name}", release.tag))
}

pub fn install(release: &Release, mut progress: impl FnMut(u64, u64)) -> Result<String, String> {
    let client = reqwest::blocking::Client::builder().user_agent("AtlasClient-Updater").build().map_err(|e| e.to_string())?;
    let sums_asset = asset(release, "SHA256SUMS")?;
    let sums = String::from_utf8(get(&client, &sums_asset.browser_download_url)?).map_err(|e| e.to_string())?;
    #[cfg(target_os = "windows")]
    let filename = if installed_windows() { "atlas-client-windows-x86_64-setup.exe" } else { "atlas-client-windows-x86_64.zip" };
    #[cfg(target_os = "linux")]
    let filename = "atlas-client-linux-x86_64.tar.gz";
    let expected = sums.lines().find_map(|line| { let mut p = line.split_whitespace(); let hash = p.next()?; let file = p.next()?.trim_start_matches('*'); (file == filename).then(|| hash.to_ascii_lowercase()) }).ok_or_else(|| format!("No checksum found for {filename}"))?;
    let bytes = get_with_progress(&client, &asset(release, filename)?.browser_download_url, &mut progress)?;
    let actual = Sha256::digest(&bytes).iter().map(|byte| format!("{byte:02x}")).collect::<String>();
    if actual != expected { return Err("The downloaded update failed its SHA-256 check.".into()); }
    let temp = std::env::temp_dir().join("atlas-update");
    fs::create_dir_all(&temp).map_err(|e| e.to_string())?;
    #[cfg(target_os = "windows")]
    {
        let file = temp.join(filename);
        fs::write(&file, &bytes).map_err(|e| e.to_string())?;
        if filename.ends_with(".exe") {
            Command::new(file).args(["/SILENT", "/SUPPRESSMSGBOXES", "/NORESTART"]).spawn().map_err(|e| e.to_string())?;
            return Ok("Setup is ready. Atlas will close while the update is installed, then reopen.".into());
        }
        let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes)).map_err(|e| e.to_string())?;
        let mut exe = archive.by_name("atlas-client.exe").map_err(|e| e.to_string())?;
        let current = std::env::current_exe().map_err(|e| e.to_string())?;
        let staged = current.with_extension("exe.atlas-new");
        std::io::copy(&mut exe, &mut fs::File::create(&staged).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
        let helper = temp.join("update-portable.ps1");
        let quote = |p: &std::path::Path| p.to_string_lossy().replace("'", "''");
        fs::write(&helper, format!(r#"$ErrorActionPreference='Stop'; Wait-Process -Id {}; Move-Item -Force '{}' '{}'; Start-Process '{}'"#, std::process::id(), quote(&staged), quote(&current), quote(&current))).map_err(|e| e.to_string())?;
        Command::new("powershell.exe").args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-File"]).arg(helper).spawn().map_err(|e| e.to_string())?;
        Ok("The verified update is ready. Atlas will restart to finish installing it.".into())
    }
    #[cfg(target_os = "linux")]
    {
        use std::io::copy;
        let archive = temp.join(filename);
        fs::write(&archive, bytes).map_err(|e| e.to_string())?;
        let current = std::env::current_exe().map_err(|e| e.to_string())?;
        let mut tar = Command::new("tar").args(["-xOzf"]).arg(&archive).arg("atlas-client").stdout(std::process::Stdio::piped()).spawn().map_err(|e| format!("Could not open update archive: {e}"))?;
        let staged = current.with_extension("atlas-new");
        let mut out = fs::File::create(&staged).map_err(|e| e.to_string())?;
        copy(&mut tar.stdout.take().ok_or("Could not read update archive")?, &mut out).map_err(|e| e.to_string())?;
        if !tar.wait().map_err(|e| e.to_string())?.success() { let _ = fs::remove_file(staged); return Err("The update archive did not contain Atlas.".into()); }
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&staged, fs::Permissions::from_mode(0o755)).map_err(|e| e.to_string())?;
        fs::rename(&staged, &current).map_err(|e| format!("Could not replace Atlas. Check that it is installed in a writable folder: {e}"))?;
        Command::new(current).spawn().map_err(|e| e.to_string())?;
        Ok("Atlas updated and restarted.".into())
    }
}

#[cfg(target_os = "windows")]
fn installed_windows() -> bool {
    let Ok(current) = std::env::current_exe() else { return false; };
    let Some(local) = std::env::var_os("LOCALAPPDATA") else { return false; };
    current == PathBuf::from(local).join("Programs/Atlas/atlas-client.exe")
}
