mod versions;
mod mods;
mod updates;

use eframe::egui::{self, Align, Color32, FontId, Layout, RichText, Stroke, Vec2};
use mc_launcher_core::prelude::{InstallRequest, JavaInstallPolicy, Launcher, LoaderSpec, LoaderVersion};
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, path::PathBuf, sync::mpsc::{self, Receiver, Sender}};
use versions::{GameVersion, VersionCatalog};

const BG: Color32 = Color32::from_rgb(17, 17, 17);
const PANEL: Color32 = Color32::from_rgb(24, 24, 24);
const PANEL_HI: Color32 = Color32::from_rgb(35, 35, 35);
const LINE: Color32 = Color32::from_rgb(58, 58, 58);
const TEXT: Color32 = Color32::from_rgb(229, 229, 229);
const MUTED: Color32 = Color32::from_rgb(163, 163, 163);
const LIME: Color32 = Color32::from_rgb(156, 171, 134);

#[derive(Clone, Copy, PartialEq, Eq)]
enum Page { Home, Performance, Versions, Mods }

enum WorkerMessage { Progress(String), Complete(Result<String, String>) }
enum UpdateWorker { Check(Result<Option<updates::Release>, String>), Installed(Result<String, String>) }

#[derive(Clone, Deserialize, PartialEq, Serialize)]
struct UserPreferences {
    ram_gb: u8,
    fps_limit: usize,
    vsync: bool,
    background_saver: bool,
    username: String,
}

impl Default for UserPreferences {
    fn default() -> Self { Self { ram_gb: 4, fps_limit: 0, vsync: false, background_saver: true, username: String::new() } }
}

struct AtlasApp {
    page: Page,
    catalog: VersionCatalog,
    selected: Option<String>,
    ram_gb: u8,
    fps_limit: usize,
    vsync: bool,
    background_saver: bool,
    username: String,
    loading: bool,
    receiver: Option<Receiver<Result<Vec<GameVersion>, String>>>,
    message: String,
    last_saved: UserPreferences,
    operation_rx: Option<Receiver<WorkerMessage>>,
    operation_active: bool,
    operation_message: String,
    selected_mods: HashSet<&'static str>,
    installed_mods: HashSet<String>,
    update_rx: Option<Receiver<UpdateWorker>>,
    update: Option<updates::Release>,
    update_message: String,
    update_busy: bool,
}

impl AtlasApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        set_theme(&cc.egui_ctx);
        let catalog = VersionCatalog::load();
        let saved = load_preferences();
        let mut app = Self {
            page: Page::Home,
            selected: catalog.versions.first().map(|v| v.id.clone()),
            catalog,
            ram_gb: saved.ram_gb,
            fps_limit: saved.fps_limit.min(4),
            vsync: saved.vsync,
            background_saver: saved.background_saver,
            username: saved.username.clone(),
            loading: false,
            receiver: None,
            message: String::new(),
            last_saved: saved,
            operation_rx: None,
            operation_active: false,
            operation_message: String::new(),
            selected_mods: ["sodium", "lithium", "ferrite-core", "modmenu", "appleskin", "zoomify"].into_iter().collect(),
            installed_mods: HashSet::new(),
            update_rx: None,
            update: None,
            update_message: "Checking for updates…".into(),
            update_busy: false,
        };
        app.check_updates();
        if app.catalog.versions.is_empty() { app.refresh_versions(); }
        app
    }

    fn refresh_versions(&mut self) {
        if self.loading { return; }
        self.loading = true;
        self.message = "Checking the official Minecraft version list…".into();
        let (tx, rx) = mpsc::channel();
        self.receiver = Some(rx);
        std::thread::spawn(move || {
            let result = VersionCatalog::fetch().map(|catalog| catalog.versions).map_err(|e| e.to_string());
            let _ = tx.send(result);
        });
    }

    fn check_updates(&mut self) {
        if self.update_rx.is_some() { return; }
        let (tx, rx) = mpsc::channel();
        self.update_rx = Some(rx);
        self.update_message = "Checking GitHub for the latest Atlas release…".into();
        std::thread::spawn(move || { let _ = tx.send(UpdateWorker::Check(updates::check())); });
    }

    fn poll_updates(&mut self, ctx: &egui::Context) {
        let Some(rx) = &self.update_rx else { return; };
        match rx.try_recv() {
            Ok(UpdateWorker::Check(Ok(Some(release)))) => { self.update_message = format!("Atlas {} is ready to install.", release.version); self.update = Some(release); self.update_rx = None; ctx.request_repaint(); }
            Ok(UpdateWorker::Check(Ok(None))) => { self.update_message = format!("Atlas {} is up to date.", env!("CARGO_PKG_VERSION")); self.update_rx = None; ctx.request_repaint(); }
            Ok(UpdateWorker::Check(Err(error))) => { self.update_message = format!("Update check failed: {error}"); self.update_rx = None; ctx.request_repaint(); }
            Ok(UpdateWorker::Installed(Ok(_message))) => {
                #[cfg(target_os = "windows")]
                std::process::exit(0);
                #[cfg(target_os = "linux")]
                std::process::exit(0);
                #[allow(unreachable_code)]
                { self.update_message = _message; self.update_busy = false; self.update = None; self.update_rx = None; ctx.request_repaint(); }
            }
            Ok(UpdateWorker::Installed(Err(error))) => { self.update_message = format!("Update failed: {error}"); self.update_busy = false; self.update_rx = None; ctx.request_repaint(); }
            Err(mpsc::TryRecvError::Disconnected) => { self.update_message = "Update check stopped unexpectedly.".into(); self.update_rx = None; }
            Err(mpsc::TryRecvError::Empty) => ctx.request_repaint_after(std::time::Duration::from_millis(200)),
        }
    }

    fn start_update(&mut self) {
        let Some(release) = self.update.clone() else { self.check_updates(); return; };
        if self.update_busy { return; }
        self.update_busy = true;
        self.update_message = format!("Downloading and verifying Atlas {}…", release.version);
        let (tx, rx) = mpsc::channel();
        self.update_rx = Some(rx);
        std::thread::spawn(move || { let _ = tx.send(UpdateWorker::Installed(updates::install(&release))); });
    }

    fn update_panel(&mut self, ui: &mut egui::Ui) {
        Self::panel(ui, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(RichText::new("ATLAS UPDATES").size(12.0).color(LIME).monospace());
                    ui.add_space(8.0);
                    ui.label(RichText::new(&self.update_message).size(16.0).strong().color(TEXT));
                    if let Some(release) = &self.update { if !release.notes.trim().is_empty() { ui.add_space(8.0); ui.label(RichText::new(release.notes.lines().take(3).collect::<Vec<_>>().join("  •  ")).size(16.0).color(MUTED)); } }
                });
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if self.update.is_some() && !self.update_busy {
                        if ui.add_sized([152.0, 40.0], egui::Button::new(RichText::new("UPDATE ATLAS").strong().color(Color32::from_rgb(22, 25, 19))).fill(LIME).corner_radius(8)).clicked() { self.start_update(); }
                    } else if !self.update_busy && self.update_rx.is_none() && ui.add(egui::Button::new(RichText::new("CHECK AGAIN").color(LIME)).corner_radius(8)).clicked() { self.check_updates(); }
                });
            });
            if self.update_busy { ui.add_space(12.0); ui.add(egui::ProgressBar::new(0.45).animate(true).text("Downloading and verifying release")); }
        });
    }

    fn poll_versions(&mut self, ctx: &egui::Context) {
        let Some(rx) = &self.receiver else { return; };
        match rx.try_recv() {
            Ok(Ok(versions)) => {
                self.catalog.replace(versions);
                if self.selected.as_ref().is_none_or(|id| !self.catalog.versions.iter().any(|v| &v.id == id)) {
                    self.selected = self.catalog.versions.first().map(|v| v.id.clone());
                }
                self.loading = false;
                self.receiver = None;
                self.message = format!("{} game versions ready. Your list is saved for quicker startup.", self.catalog.versions.len());
                ctx.request_repaint();
            }
            Ok(Err(error)) => {
                self.loading = false;
                self.receiver = None;
                self.message = format!("Could not refresh versions: {error}");
                ctx.request_repaint();
            }
            Err(mpsc::TryRecvError::Disconnected) => {
                self.loading = false;
                self.receiver = None;
                self.message = "Version refresh stopped unexpectedly.".into();
            }
            Err(mpsc::TryRecvError::Empty) => ctx.request_repaint_after(std::time::Duration::from_millis(100)),
        }
    }

    fn nav(&mut self, ui: &mut egui::Ui, page: Page, icon: &str, title: &str) {
        let active = self.page == page;
        let fill = if active { PANEL_HI } else { Color32::TRANSPARENT };
        let color = if active { TEXT } else { MUTED };
        let response = ui.add_sized([ui.available_width(), 40.0], egui::Button::new(RichText::new(format!("{icon}   {title}")).size(16.0).color(color)).fill(fill).stroke(Stroke::NONE).corner_radius(8));
        if response.clicked() { self.page = page; }
    }

    fn instance_dir(version: &str) -> Option<PathBuf> {
        dirs::data_local_dir().map(|dir| dir.join("Atlas").join("instances").join(version))
    }

    fn start_install(&mut self, fabric: bool) {
        if self.operation_active { return; }
        let Some(version) = self.selected.clone() else { self.message = "Choose a Minecraft version first.".into(); return; };
        let Some(instance) = Self::instance_dir(&version) else { self.message = "Atlas could not find this device's data folder.".into(); return; };
        self.operation_active = true;
        self.operation_message = if fabric { format!("Preparing Fabric for Minecraft {version}…") } else { format!("Installing Minecraft {version}…") };
        let (tx, rx) = mpsc::channel();
        self.operation_rx = Some(rx);
        std::thread::spawn(move || {
            let result = (|| -> Result<String, String> {
                let launcher = Launcher::new(&instance);
                let request = if fabric {
                    InstallRequest { minecraft_version: version.clone(), loader: Some(LoaderSpec::Fabric { version: LoaderVersion::LatestStable }), java: JavaInstallPolicy::Auto }
                } else { InstallRequest::vanilla(version.clone()) };
                let sender = tx.clone();
                let mut report = move |event| { let _ = sender.send(WorkerMessage::Progress(format!("{event:?}"))); };
                let installed = launcher.install_with_progress(request, &mut report).map_err(|error| error.to_string())?;
                if fabric { std::fs::write(instance.join("fabric-profile.txt"), &installed.version_id).map_err(|error| error.to_string())?; }
                Ok(if fabric { format!("Fabric is ready for Minecraft {version}. Install the mods you want from the Mods page.") } else { format!("Minecraft {version} is installed and cached on this device.") })
            })();
            let _ = tx.send(WorkerMessage::Complete(result));
        });
    }

    fn start_mod_install(&mut self) {
        if self.operation_active { return; }
        let Some(version) = self.selected.clone() else { self.message = "Choose a Minecraft version first.".into(); return; };
        let Some(instance) = Self::instance_dir(&version) else { self.message = "Atlas could not find this device's data folder.".into(); return; };
        if !instance.join("fabric-profile.txt").exists() { self.message = "Set up Fabric for this version first on the Game Versions page.".into(); return; }
        if self.selected_mods.is_empty() { self.message = "Choose at least one mod to install.".into(); return; }
        let chosen: Vec<&'static str> = self.selected_mods.iter().copied().collect();
        self.operation_active = true;
        self.operation_message = format!("Preparing {} selected mods for Minecraft {version}…", chosen.len());
        let (tx, rx) = mpsc::channel();
        self.operation_rx = Some(rx);
        std::thread::spawn(move || {
            let sender: Sender<WorkerMessage> = tx.clone();
            let result = mods::install_selected(&version, &instance, &chosen, move |text| { let _ = sender.send(WorkerMessage::Progress(text)); })
                .map(|installed| format!("Installed {} selected mods for Minecraft {version}.", installed.len()))
                .map_err(|error| error.to_string());
            let _ = tx.send(WorkerMessage::Complete(result));
        });
    }

    fn poll_operation(&mut self, ctx: &egui::Context) {
        let Some(rx) = &self.operation_rx else { return; };
        loop {
            match rx.try_recv() {
                Ok(WorkerMessage::Progress(message)) => self.operation_message = message,
                Ok(WorkerMessage::Complete(result)) => {
                    self.operation_active = false;
                    self.operation_rx = None;
                    self.operation_message = match result { Ok(message) => { self.message = message.clone(); message }, Err(error) => format!("Setup stopped: {error}") };
                    if let Some(version) = self.selected.as_deref().and_then(Self::instance_dir) { self.installed_mods = mods::installed_mod_names(&version); }
                    ctx.request_repaint();
                    break;
                }
                Err(mpsc::TryRecvError::Disconnected) => { self.operation_active = false; self.operation_rx = None; self.operation_message = "Setup stopped unexpectedly.".into(); break; }
                Err(mpsc::TryRecvError::Empty) => { ctx.request_repaint_after(std::time::Duration::from_millis(150)); break; }
            }
        }
    }

    fn header(&self, ui: &mut egui::Ui, eyebrow: &str, title: &str, subtitle: &str) {
        ui.label(RichText::new(eyebrow.to_uppercase()).size(12.0).color(LIME).monospace());
        ui.add_space(8.0);
        ui.label(RichText::new(title).size(40.0).strong().color(TEXT));
        ui.add_space(8.0);
        ui.label(RichText::new(subtitle).size(16.0).color(MUTED));
        ui.add_space(24.0);
    }

    fn panel<R>(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
        egui::Frame::new().fill(PANEL).stroke(Stroke::new(1.0, LINE)).corner_radius(8).inner_margin(16).show(ui, add).inner
    }

    fn home(&mut self, ui: &mut egui::Ui) {
        self.header(ui, "Your game, your rules", "Your next world awaits.", "A faster, cleaner Minecraft experience starts right here.");
        let version_ids: Vec<String> = self.catalog.all_versions().map(|version| version.id.clone()).collect();
        egui::Frame::new().fill(Color32::from_rgb(29, 31, 27)).stroke(Stroke::new(1.0, LINE)).corner_radius(8).inner_margin(24).show(ui, |ui| {
            ui.set_min_height(160.0);
            ui.vertical(|ui| {
                ui.label(RichText::new("●  BUILT FOR THE WAY YOU PLAY").size(12.0).color(LIME).monospace());
                ui.add_space(16.0);
                ui.label(RichText::new("More game. Less waiting.").size(28.0).strong().color(TEXT));
                ui.add_space(8.0);
                ui.label(RichText::new("A lighter Minecraft setup with useful performance controls, built around your version and your hardware.").size(16.0).color(MUTED));
            });
        });
        ui.add_space(16.0);
        self.update_panel(ui);
        ui.add_space(16.0);
        ui.columns(2, |columns| {
            Self::panel(&mut columns[0], |ui| {
                ui.horizontal(|ui| { ui.vertical(|ui| { ui.label(RichText::new("Ready when you are").size(20.0).strong().color(TEXT)); ui.label(RichText::new("Install a version, add Fabric, then choose your mods.").size(16.0).color(MUTED)); }); ui.with_layout(Layout::right_to_left(Align::Center), |ui| { ui.label(RichText::new("FABRIC + MODS AVAILABLE").size(12.0).color(LIME).monospace()); }); });
                ui.add_space(16.0);
                ui.horizontal(|ui| {
                    let selected_text = self.selected.as_deref().unwrap_or("Loading versions…");
                    egui::ComboBox::from_id_salt("home-version").selected_text(selected_text).width(150.0).show_ui(ui, |ui| {
                        for version in &version_ids {
                            if ui.selectable_label(self.selected.as_deref() == Some(version), version).clicked() { self.selected = Some(version.clone()); }
                        }
                    });
                    let launch = ui.add_sized([ui.available_width(), 40.0], egui::Button::new(RichText::new("SIGN IN TO PLAY").strong().size(16.0).color(Color32::from_rgb(22, 25, 19))).fill(LIME).corner_radius(8));
                    if launch.clicked() {
                        self.message = "Microsoft sign-in is required to launch Minecraft. Install a game version and prepare your Fabric mods from the Game Versions and Mods pages.".into();
                    }
                });
                ui.add_space(16.0);
                ui.horizontal(|ui| { ui.label(RichText::new("●  Version list cached on this device").size(16.0).color(MUTED)); ui.with_layout(Layout::right_to_left(Align::Center), |ui| { ui.label(RichText::new("Latest versions first").size(12.0).monospace().color(MUTED)); }); });
            });
            Self::panel(&mut columns[1], |ui| {
                ui.label(RichText::new("Your local profile").size(20.0).strong().color(TEXT));
                ui.label(RichText::new("Give this setup a name. Microsoft sign-in will connect your game account.").size(16.0).color(MUTED));
                ui.add_space(16.0);
                ui.label(RichText::new("PROFILE NAME").size(12.0).color(MUTED).monospace());
                ui.add_space(8.0);
                ui.add(egui::TextEdit::singleline(&mut self.username).hint_text("Profile name").desired_width(f32::INFINITY).min_size(Vec2::new(0.0, 40.0)));
                ui.add_space(16.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new("RAM ALLOCATION").size(12.0).color(MUTED).monospace());
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| { ui.label(RichText::new(format!("{} GB", self.ram_gb)).size(12.0).color(LIME).monospace()); });
                });
                ui.add(egui::Slider::new(&mut self.ram_gb, 2..=16).show_value(false));
            });
        });
        ui.add_space(16.0);
        Self::panel(ui, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| { ui.label(RichText::new("GAME VERSIONS").size(12.0).color(MUTED).monospace()); ui.label(RichText::new(format!("{} available", self.catalog.versions.len())).size(16.0).strong().color(TEXT)); });
                ui.add_space(24.0); ui.separator(); ui.add_space(24.0);
                ui.vertical(|ui| { ui.label(RichText::new("MEMORY").size(12.0).color(MUTED).monospace()); ui.label(RichText::new(format!("{} GB allocated", self.ram_gb)).size(16.0).strong().color(TEXT)); });
                ui.add_space(24.0); ui.separator(); ui.add_space(24.0);
                ui.vertical(|ui| { ui.label(RichText::new("MOD SUPPORT").size(12.0).color(MUTED).monospace()); ui.label(RichText::new("Fabric ready").size(16.0).strong().color(TEXT)); });
            });
        });
    }

    fn performance(&mut self, ui: &mut egui::Ui) {
        self.header(ui, "Smooth frames, your way", "Performance", "Tune Atlas for the hardware you have and the way you play.");
        Self::panel(ui, |ui| {
            ui.label(RichText::new("Graphics & memory").size(20.0).strong().color(TEXT));
            ui.label(RichText::new("Set your launch preferences. Atlas saves these on this device.").size(16.0).color(MUTED));
            ui.add_space(16.0);
            egui::Grid::new("perf-grid").num_columns(2).spacing([32.0, 16.0]).show(ui, |ui| {
                ui.vertical(|ui| { ui.label(RichText::new("Maximum memory").size(16.0).color(TEXT)); ui.label(RichText::new("Give Minecraft enough room for your mods.").size(16.0).color(MUTED)); });
                ui.horizontal(|ui| { ui.add(egui::Slider::new(&mut self.ram_gb, 2..=16).suffix(" GB").show_value(false)); ui.label(RichText::new(format!("{} GB", self.ram_gb)).size(12.0).color(LIME).monospace()); }); ui.end_row();
                ui.vertical(|ui| { ui.label(RichText::new("Frame rate limit").size(16.0).color(TEXT)); ui.label(RichText::new("Cap frames to reduce heat and power use.").size(16.0).color(MUTED)); });
                egui::ComboBox::from_id_salt("fps").selected_text(["Unlimited", "240 FPS", "144 FPS", "120 FPS", "60 FPS"][self.fps_limit]).show_ui(ui, |ui| { for (i, label) in ["Unlimited", "240 FPS", "144 FPS", "120 FPS", "60 FPS"].iter().enumerate() { ui.selectable_value(&mut self.fps_limit, i, *label); } }); ui.end_row();
                ui.vertical(|ui| { ui.label(RichText::new("VSync").size(16.0).color(TEXT)); ui.label(RichText::new("Match your monitor refresh rate.").size(16.0).color(MUTED)); }); ui.checkbox(&mut self.vsync, "Enabled"); ui.end_row();
                ui.vertical(|ui| { ui.label(RichText::new("Background saver").size(16.0).color(TEXT)); ui.label(RichText::new("Reduce resource use while you tab out.").size(16.0).color(MUTED)); }); ui.checkbox(&mut self.background_saver, "Enabled"); ui.end_row();
            });
        });
        ui.add_space(16.0);
        egui::Frame::new().fill(PANEL).stroke(Stroke::new(1.0, LINE)).corner_radius(8).inner_margin(16).show(ui, |ui| { ui.label(RichText::new("About performance mods").size(16.0).strong().color(LIME)); ui.add_space(8.0); ui.label(RichText::new("Choose individual Fabric performance mods from the Mods page. Atlas checks the selected Minecraft version and installs required dependencies with each mod.").size(16.0).color(MUTED)); });
    }

    fn versions(&mut self, ui: &mut egui::Ui) {
        self.header(ui, "Pick up where you left off", "Game versions", "Every official release and snapshot, with newest versions at the top.");
        let selected = self.selected.clone();
        ui.horizontal(|ui| {
            ui.label(RichText::new(format!("{} versions", self.catalog.versions.len())).size(16.0).color(MUTED));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let label = if self.loading { "REFRESHING…" } else { "↻  REFRESH VERSIONS" };
                if ui.add_enabled(!self.loading, egui::Button::new(RichText::new(label).size(16.0).color(LIME))).clicked() { self.refresh_versions(); }
            });
        });
        ui.add_space(16.0);
        Self::panel(ui, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(RichText::new("SELECTED VERSION").size(12.0).color(MUTED).monospace());
                    ui.label(RichText::new(selected.as_deref().unwrap_or("Loading versions…")).size(20.0).strong().color(TEXT));
                    ui.label(RichText::new("Game files are cached in an Atlas instance for faster starts.").size(16.0).color(MUTED));
                });
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui.add_enabled(!self.operation_active && selected.is_some(), egui::Button::new(RichText::new("SET UP FABRIC").strong().color(Color32::from_rgb(22, 25, 19))).fill(LIME).corner_radius(8)).clicked() { self.start_install(true); }
                    if ui.add_enabled(!self.operation_active && selected.is_some(), egui::Button::new(RichText::new("INSTALL VANILLA").color(TEXT)).corner_radius(8)).clicked() { self.start_install(false); }
                });
            });
            if self.operation_active { ui.add_space(16.0); ui.horizontal(|ui| { ui.add(egui::Spinner::new().color(LIME)); ui.label(RichText::new(&self.operation_message).size(16.0).color(LIME)); }); }
            else if !self.operation_message.is_empty() { ui.add_space(8.0); ui.label(RichText::new(&self.operation_message).size(16.0).color(MUTED)); }
        });
        ui.add_space(16.0);
        let display_versions = self.catalog.versions.clone();
        Self::panel(ui, |ui| {
            if self.loading && display_versions.is_empty() {
                ui.label(RichText::new("Fetching Mojang's version list").size(16.0).color(TEXT));
                ui.add_space(8.0);
                for _ in 0..6 {
                    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 24.0), egui::Sense::hover());
                    ui.painter().rect_filled(rect, 4, Color32::from_rgb(44, 44, 44));
                    ui.add_space(8.0);
                }
                return;
            }
            if display_versions.is_empty() {
                ui.label(RichText::new("No saved version list yet.").size(20.0).strong().color(TEXT));
                ui.add_space(8.0);
                ui.label(RichText::new("Connect to the internet to load every Minecraft release and snapshot.").size(16.0).color(MUTED));
                ui.add_space(16.0);
                if ui.add_sized([160.0, 40.0], egui::Button::new("Try again").corner_radius(8)).clicked() { self.refresh_versions(); }
                return;
            }
            egui::Grid::new("versions-grid").num_columns(4).striped(true).min_col_width(90.0).spacing([24.0, 8.0]).show(ui, |ui| {
                for head in ["VERSION", "TYPE", "RELEASED", "PROFILE"] { ui.label(RichText::new(head).size(12.0).color(MUTED).monospace()); } ui.end_row();
                for version in &display_versions {
                    let selected = self.selected.as_deref() == Some(&version.id);
                    if ui.selectable_label(selected, RichText::new(&version.id).size(12.0).color(TEXT)).clicked() { self.selected = Some(version.id.clone()); }
                    let kind = match version.kind.as_str() {
                        "release" => "Release",
                        "snapshot" => "Snapshot",
                        "old_beta" => "Old beta",
                        "old_alpha" => "Old alpha",
                        _ => "Other",
                    };
                    ui.label(RichText::new(kind).size(12.0).color(if version.kind == "release" { LIME } else { MUTED }));
                    ui.label(RichText::new(version.release_time.get(..10).unwrap_or("—")).size(12.0).monospace().color(MUTED));
                    let installed = Self::instance_dir(&version.id).is_some_and(|path| path.join("versions").join(&version.id).with_extension("json").exists());
                    ui.label(RichText::new(if installed { "Cached" } else { "Not installed" }).size(16.0).color(if installed { LIME } else { MUTED })); ui.end_row();
                }
            });
        });
    }

    fn mods_page(&mut self, ui: &mut egui::Ui) {
        self.header(ui, "Make your own setup", "Fabric mods", "Choose the performance tools and small quality of life additions you actually want.");
        let selected = self.selected.clone();
        self.installed_mods = selected.as_deref().and_then(Self::instance_dir).map(|path| mods::installed_mod_names(&path)).unwrap_or_default();
        let fabric_ready = selected.as_deref().and_then(Self::instance_dir).is_some_and(|path| path.join("fabric-profile.txt").exists());
        Self::panel(ui, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(RichText::new("FOR SELECTED VERSION").size(12.0).color(MUTED).monospace());
                    ui.label(RichText::new(selected.as_deref().unwrap_or("Choose a game version")).size(20.0).strong().color(TEXT));
                });
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let label = format!("INSTALL {} SELECTED", self.selected_mods.len());
                    if ui.add_enabled(!self.operation_active && !self.selected_mods.is_empty() && fabric_ready, egui::Button::new(RichText::new(label).strong().color(Color32::from_rgb(22, 25, 19))).fill(LIME).corner_radius(8)).clicked() { self.start_mod_install(); }
                });
            });
            if !fabric_ready {
                ui.add_space(16.0);
                ui.label(RichText::new("Set up a Fabric profile for this version on the Game Versions page before installing mods.").size(16.0).color(MUTED));
            }
            if self.operation_active { ui.add_space(16.0); ui.horizontal(|ui| { ui.add(egui::Spinner::new().color(LIME)); ui.label(RichText::new(&self.operation_message).size(16.0).color(LIME)); }); }
            else if !self.operation_message.is_empty() { ui.add_space(8.0); ui.label(RichText::new(&self.operation_message).size(16.0).color(MUTED)); }
        });
        ui.add_space(16.0);
        let mut group = "";
        for option in mods::MODS {
            if group != option.group {
                group = option.group;
                ui.add_space(8.0);
                ui.label(RichText::new(group).size(12.0).color(LIME).monospace());
                ui.add_space(8.0);
            }
            Self::panel(ui, |ui| {
                ui.horizontal(|ui| {
                    let mut enabled = self.selected_mods.contains(option.slug);
                    if ui.checkbox(&mut enabled, "").changed() {
                        if enabled { self.selected_mods.insert(option.slug); } else { self.selected_mods.remove(option.slug); }
                    }
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(option.name).size(20.0).strong().color(TEXT));
                            if self.installed_mods.iter().any(|filename| filename.to_lowercase().contains(&option.slug.replace('-', ""))) {
                                ui.label(RichText::new("INSTALLED").size(12.0).color(LIME).monospace());
                            }
                        });
                        ui.label(RichText::new(option.description).size(16.0).color(MUTED));
                    });
                });
            });
            ui.add_space(8.0);
        }
        ui.add_space(8.0);
        ui.label(RichText::new("Mods are downloaded from Modrinth. Atlas checks each mod against the selected Minecraft version and Fabric, and installs required dependencies automatically.").size(16.0).color(MUTED));
    }
}

impl eframe::App for AtlasApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_versions(ctx);
        self.poll_operation(ctx);
        self.poll_updates(ctx);
        egui::SidePanel::left("sidebar").exact_width(224.0).frame(egui::Frame::new().fill(Color32::from_rgb(12, 12, 12)).inner_margin(16)).show(ctx, |ui| {
            ui.add_space(16.0);
            ui.horizontal(|ui| { ui.label(RichText::new("▦").size(22.0).strong().color(LIME)); ui.label(RichText::new("atlas").size(22.0).strong().color(TEXT)); ui.label(RichText::new(".").size(22.0).strong().color(LIME)); });
            ui.add_space(32.0);
            ui.label(RichText::new("YOUR CLIENT").size(12.0).color(Color32::from_rgb(99, 110, 125)).monospace()); ui.add_space(8.0);
            self.nav(ui, Page::Home, "◫", "Overview"); self.nav(ui, Page::Versions, "◷", "Game versions"); self.nav(ui, Page::Performance, "⚙", "Performance"); self.nav(ui, Page::Mods, "⊞", "Mods");
            ui.add_space(24.0); ui.label(RichText::new("SET UP").size(12.0).color(Color32::from_rgb(99, 110, 125)).monospace()); ui.add_space(8.0);
            ui.with_layout(Layout::bottom_up(Align::Min), |ui| {
                ui.label(RichText::new("●  Atlas Preview     v0.1.0").size(16.0).color(MUTED)); ui.add_space(16.0);
                ui.separator(); ui.add_space(8.0);
                ui.horizontal(|ui| { ui.label(RichText::new("A").size(20.0).strong().color(LIME)); ui.vertical(|ui| { ui.label(RichText::new(if self.username.is_empty() { "Set up your profile" } else { &self.username }).size(16.0).strong().color(TEXT)); ui.label(RichText::new("Local profile").size(12.0).color(MUTED)); }); });
            });
        });
        egui::TopBottomPanel::top("topbar").exact_height(48.0).frame(egui::Frame::new().fill(BG).inner_margin(egui::Margin::symmetric(24, 0))).show(ctx, |ui| {
            ui.with_layout(Layout::left_to_right(Align::Center), |ui| { ui.label(RichText::new("ATLAS CLIENT").size(12.0).color(MUTED).monospace()); ui.label(RichText::new("  /  ").size(16.0).color(LINE)); ui.label(RichText::new(match self.page { Page::Home => "OVERVIEW", Page::Performance => "PERFORMANCE", Page::Versions => "GAME VERSIONS", Page::Mods => "FABRIC MODS" }).size(16.0).color(TEXT).monospace()); });
        });
        egui::CentralPanel::default().frame(egui::Frame::new().fill(BG).inner_margin(egui::Margin::symmetric(24, 24))).show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                match self.page { Page::Home => self.home(ui), Page::Performance => self.performance(ui), Page::Versions => self.versions(ui), Page::Mods => self.mods_page(ui) }
                if !self.message.is_empty() { ui.add_space(16.0); ui.label(RichText::new(&self.message).size(16.0).color(MUTED)); }
            });
        });
        let preferences = UserPreferences {
            ram_gb: self.ram_gb,
            fps_limit: self.fps_limit,
            vsync: self.vsync,
            background_saver: self.background_saver,
            username: self.username.clone(),
        };
        if preferences != self.last_saved {
            match save_preferences(&preferences) {
                Ok(()) => { self.last_saved = preferences; self.message = "Preferences saved on this device.".into(); }
                Err(error) => self.message = format!("Could not save preferences: {error}"),
            }
        }
    }
}

fn set_theme(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();
    style.visuals = egui::Visuals::dark();
    style.visuals.window_fill = PANEL;
    style.visuals.panel_fill = BG;
    style.visuals.widgets.noninteractive.fg_stroke.color = TEXT;
    style.visuals.widgets.inactive.bg_fill = PANEL_HI;
    style.visuals.widgets.inactive.fg_stroke.color = TEXT;
    style.visuals.widgets.hovered.bg_fill = Color32::from_rgb(43, 43, 43);
    style.visuals.selection.bg_fill = Color32::from_rgb(67, 75, 59);
    style.visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, LINE);
    style.visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, LINE);
    style.visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, MUTED);
    style.visuals.widgets.active.bg_stroke = Stroke::new(1.0, LIME);
    style.visuals.widgets.inactive.corner_radius = 4.into();
    style.visuals.widgets.hovered.corner_radius = 8.into();
    style.visuals.widgets.active.corner_radius = 8.into();
    style.spacing.item_spacing = Vec2::new(8.0, 8.0);
    style.spacing.interact_size = Vec2::new(40.0, 40.0);
    style.text_styles.insert(egui::TextStyle::Body, FontId::proportional(16.0));
    ctx.set_style(style);
}

fn preferences_path() -> Option<std::path::PathBuf> {
    dirs::config_dir().map(|dir| dir.join("Atlas").join("preferences.json"))
}

fn load_preferences() -> UserPreferences {
    let Some(path) = preferences_path() else { return UserPreferences::default(); };
    std::fs::read(path).ok().and_then(|bytes| serde_json::from_slice(&bytes).ok()).unwrap_or_default()
}

fn save_preferences(preferences: &UserPreferences) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(path) = preferences_path() {
        if let Some(parent) = path.parent() { std::fs::create_dir_all(parent)?; }
        std::fs::write(path, serde_json::to_vec_pretty(preferences)?)?;
    }
    Ok(())
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions { viewport: egui::ViewportBuilder::default().with_title("Atlas — Minecraft, your way").with_inner_size([1120.0, 800.0]).with_min_inner_size([800.0, 600.0]), ..Default::default() };
    eframe::run_native("Atlas", options, Box::new(|cc| Ok(Box::new(AtlasApp::new(cc)))))
}
