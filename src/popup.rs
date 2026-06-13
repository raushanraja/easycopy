use crate::config::Config;
use crate::desktop::DesktopApp;
use crate::history::ClipItem;
use crate::storage;
use crate::theme::{self, ThemeColors};
use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::Arc;

// ================================================================
//  LoadedData — sent from background thread when history+caches done
// ================================================================

struct LoadedData {
    clips: Vec<ClipItem>,
    cached_clip_char_counts: Vec<usize>,
    cached_clip_previews: Vec<String>,
    cached_clip_search: Vec<String>,
    cached_clip_file_sizes: HashMap<String, u64>,
}

// ================================================================
//  DisplayItem — unified list entry for clips and apps
// ================================================================

#[derive(Debug, Clone)]
enum DisplayItem {
    Clip { clip_idx: usize },
    App { app_idx: usize },
}

const SEARCH_HINT: &str = "Search clips, image size, or filename…";
const FOOTER_HELP: &str =
    "Esc close · Enter paste · Del remove · Ctrl+O open · Ctrl+K clear search";

/// Entry point for the popup window. Blocks until the window is closed.
/// If `should_paste` is set to true, this function simulates Ctrl+V after
/// the user chooses an item.
pub fn run_popup(config: Config, should_paste: Arc<AtomicBool>) {
    let width = config.general.popup_width;
    let height = config.general.popup_height;

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([width, height])
            .with_decorations(false)
            .with_always_on_top()
            .with_resizable(false)
            .with_transparent(true),
        ..Default::default()
    };

    let should_paste_after_window = should_paste.clone();
    let auto_paste = config.general.auto_paste;
    let paste_delay_ms = config.general.paste_delay_ms;

    theme::set_debug_logging(config.general.debug_logging);

    let selected_item = Arc::new(std::sync::Mutex::new(None));
    let selected_item_for_app = selected_item.clone();

    let _ = eframe::run_native(
        "easycopy",
        options,
        Box::new(move |cc| {
            theme::apply_theme_and_fonts(&cc.egui_ctx, &config);
            Ok(Box::new(PopupApp::new(
                cc,
                config,
                should_paste_after_window,
                selected_item_for_app,
            )))
        }),
    );

    let item_to_write = {
        let mut lock = selected_item.lock().unwrap();
        lock.take()
    };

    if let Some(item) = item_to_write {
        let sent_via_ipc = is_daemon_running() && crate::ipc::send_paste_request(&item).is_ok();
        if !sent_via_ipc {
            if let Ok(mut cb) = arboard::Clipboard::new() {
                let is_image = item.is_image();
                let write_result = match item {
                    ClipItem::Text { content, .. } => cb.set_text(content),
                    ClipItem::Image { filename, .. } => {
                        if let Ok((w, h, data)) = storage::load_image(&filename) {
                            let img_data = arboard::ImageData {
                                width: w as usize,
                                height: h as usize,
                                bytes: std::borrow::Cow::Owned(data),
                            };
                            cb.set_image(img_data)
                        } else {
                            Err(arboard::Error::ContentNotAvailable)
                        }
                    }
                };

                if write_result.is_ok() {
                    if auto_paste && should_paste.load(Ordering::Relaxed) {
                        std::thread::sleep(std::time::Duration::from_millis(paste_delay_ms));
                        simulate_paste();
                    }
                    // Keep the clipboard connection alive so target application has time to copy it.
                    let keep_alive_duration = if is_image {
                        std::time::Duration::from_secs(3)
                    } else {
                        std::time::Duration::from_secs(1)
                    };
                    std::thread::sleep(keep_alive_duration);
                }
            }
        }
    }
}

fn is_daemon_running() -> bool {
    let pid_file = Config::data_dir().join("daemon.pid");
    if let Ok(pid_str) = std::fs::read_to_string(pid_file) {
        if let Ok(pid) = pid_str.trim().parse::<u32>() {
            return std::path::Path::new(&format!("/proc/{}", pid)).exists();
        }
    }
    false
}

/// Run the popup UI inside the daemon process, on its own thread.
/// The window starts hidden. Returns (thread handle, show_tx, shutdown_tx).
pub fn run_popup_in_daemon(
    config: Config,
    paste_tx: mpsc::Sender<ClipItem>,
    history_rx: mpsc::Receiver<Vec<ClipItem>>,
) -> (
    std::thread::JoinHandle<()>,
    mpsc::Sender<()>,
    mpsc::Sender<()>,
) {
    let width = config.general.popup_width;
    let height = config.general.popup_height;

    theme::set_debug_logging(config.general.debug_logging);

    let visible = Arc::new(AtomicBool::new(false));
    let (show_tx, show_rx) = mpsc::channel();
    let (shutdown_tx, shutdown_rx) = mpsc::channel();

    let handle = std::thread::spawn(move || {
        // Construct NativeOptions inside the thread (it may not be Send)
        let options = eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_inner_size([width, height])
                .with_decorations(false)
                .with_always_on_top()
                .with_resizable(false)
                .with_transparent(true)
                .with_visible(false), // hidden at startup
            #[cfg(target_os = "linux")]
            event_loop_builder: Some(Box::new(|builder| {
                // Allow running the event loop on a non-main thread.
                use winit::platform::x11::EventLoopBuilderExtX11;
                builder.with_any_thread(true);
            })),
            ..Default::default()
        };

        let _ = eframe::run_native(
            "easycopy",
            options,
            Box::new(move |cc| {
                theme::apply_theme_and_fonts(&cc.egui_ctx, &config);
                Ok(Box::new(PopupApp::new_daemon(
                    cc,
                    config,
                    show_rx,
                    shutdown_rx,
                    paste_tx,
                    history_rx,
                    visible,
                )))
            }),
        );
    });

    (handle, show_tx, shutdown_tx)
}

fn simulate_paste() {
    // xdotool is the most reliable simple option on X11. Wayland users can
    // disable auto_paste or provide their own compositor-level paste binding.
    let _ = std::process::Command::new("xdotool")
        .args(["key", "ctrl+v"])
        .status();
}

struct PopupApp {
    clips: Vec<ClipItem>,
    apps: Vec<DesktopApp>,
    filtered: Vec<DisplayItem>,
    query: String,
    selected: usize,
    textures: HashMap<String, egui::TextureHandle>,
    app_icon_textures: HashMap<String, egui::TextureHandle>,
    should_paste: Arc<AtomicBool>,
    selected_item_out: Arc<std::sync::Mutex<Option<ClipItem>>>,
    preview_chars: usize,
    focus_search_once: bool,
    scroll_to_selected_once: bool,
    config: Config,
    theme_colors: Option<ThemeColors>,
    preview_image: Option<(String, egui::TextureHandle)>,
    lightbox_zoom: f32,
    lightbox_pan: egui::Vec2,
    focused_once: bool,
    rx: std::sync::mpsc::Receiver<(String, egui::ColorImage)>,
    clip_rx: std::sync::mpsc::Receiver<LoadedData>,
    clips_loaded: bool,
    app_rx: std::sync::mpsc::Receiver<Vec<DesktopApp>>,
    apps_loaded: bool,
    cached_clip_char_counts: Vec<usize>,
    cached_clip_previews: Vec<String>,
    cached_clip_search: Vec<String>,
    cached_clip_file_sizes: HashMap<String, u64>,
    cached_app_search: Vec<String>,
    // Daemon-mode fields (used when popup runs inside the daemon process)
    daemon_mode: bool,
    visible: Arc<AtomicBool>,
    show_rx: Option<mpsc::Receiver<()>>,
    shutdown_rx: Option<mpsc::Receiver<()>>,
    daemon_paste_tx: Option<mpsc::Sender<ClipItem>>,
    history_update_rx: Option<mpsc::Receiver<Vec<ClipItem>>>,
    hide_command_sent: bool,
}

impl PopupApp {
    fn new(
        _cc: &eframe::CreationContext<'_>,
        config: Config,
        should_paste: Arc<AtomicBool>,
        selected_item_out: Arc<std::sync::Mutex<Option<ClipItem>>>,
    ) -> Self {
        // ── All heavy I/O happens in background threads ──

        // History load + cache computation thread
        let (clip_tx, clip_rx) = std::sync::mpsc::channel();
        let preview_chars = config.general.preview_chars;
        std::thread::spawn(move || {
            let clips: Vec<ClipItem> = storage::load_history().into_iter().collect();

            // Compute caches in the background alongside loading
            let cached_clip_char_counts: Vec<usize> = clips
                .iter()
                .map(|item| match item {
                    ClipItem::Text { content, .. } => content.chars().count(),
                    _ => 0,
                })
                .collect();

            let cached_clip_previews: Vec<String> = clips
                .iter()
                .map(|item| match item {
                    ClipItem::Text { content, .. } => preview_text(content, preview_chars),
                    _ => String::new(),
                })
                .collect();

            let cached_clip_search: Vec<String> = clips
                .iter()
                .map(|item| match item {
                    ClipItem::Text { content, .. } => content.to_lowercase(),
                    ClipItem::Image {
                        width,
                        height,
                        filename,
                        ..
                    } => format!(
                        "{}\u{00d7}{} {}x{} {}",
                        width, height, width, height, filename
                    )
                    .to_lowercase(),
                })
                .collect();

            let mut cached_clip_file_sizes = HashMap::new();
            for item in &clips {
                if let ClipItem::Image { filename, .. } = item {
                    if !filename.is_empty() {
                        if let Ok(meta) = std::fs::metadata(Config::images_dir().join(filename)) {
                            cached_clip_file_sizes.insert(filename.clone(), meta.len());
                        }
                    }
                }
            }

            let _ = clip_tx.send(LoadedData {
                clips,
                cached_clip_char_counts,
                cached_clip_previews,
                cached_clip_search,
                cached_clip_file_sizes,
            });
        });

        // ── Async image thumbnail loading ──
        let (tx, rx) = std::sync::mpsc::channel();
        let clips_for_images: Vec<ClipItem> = storage::load_history().into_iter().collect();
        std::thread::spawn(move || {
            for item in clips_for_images {
                if let ClipItem::Image { filename, .. } = item {
                    if filename.is_empty() {
                        continue;
                    }
                    let images_dir = Config::images_dir();
                    let thumb_path = images_dir.join(format!("thumb_{}", filename));

                    if let Ok(img) = image::open(&thumb_path) {
                        let rgba = img.to_rgba8();
                        let size = [rgba.width() as usize, rgba.height() as usize];
                        let ci = egui::ColorImage::from_rgba_unmultiplied(size, rgba.as_raw());
                        let _ = tx.send((filename, ci));
                    } else {
                        // Fallback: load original image, resize, save as thumbnail, and send.
                        let path = images_dir.join(&filename);
                        if let Ok(img) = image::open(path) {
                            let thumb = img.resize(52, 52, image::imageops::FilterType::Triangle);
                            let rgba = thumb.to_rgba8();
                            let size = [rgba.width() as usize, rgba.height() as usize];
                            let ci = egui::ColorImage::from_rgba_unmultiplied(size, rgba.as_raw());

                            // Save thumbnail for future loads
                            let _ = thumb.save(&thumb_path);

                            let _ = tx.send((filename, ci));
                        }
                    }
                }
            }
        });

        // ── Async desktop app loading (from cache first, refresh in bg) ──
        let (app_tx, app_rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            // Try cache first – should be nearly instant
            if let Some(cached) = crate::desktop::load_apps_cache() {
                let _ = app_tx.send(cached);
                // Then refresh the cache in background for next time
                crate::desktop::refresh_and_cache_apps();
            } else {
                // No cache yet – do the full scan (daemon may still be starting)
                let apps = crate::desktop::refresh_and_cache_apps();
                let _ = app_tx.send(apps);
            }
        });

        let theme_colors = ThemeColors::from_config(&config);

        Self {
            clips: Vec::new(),
            apps: Vec::new(),
            filtered: Vec::new(),
            query: String::new(),
            selected: 0,
            textures: HashMap::new(),
            app_icon_textures: HashMap::new(),
            should_paste,
            selected_item_out,
            preview_chars: config.general.preview_chars,
            focus_search_once: true,
            scroll_to_selected_once: false,
            config,
            theme_colors,
            preview_image: None,
            lightbox_zoom: 1.0,
            lightbox_pan: egui::Vec2::ZERO,
            focused_once: false,
            rx,
            clip_rx,
            clips_loaded: false,
            app_rx,
            apps_loaded: false,
            cached_clip_char_counts: Vec::new(),
            cached_clip_previews: Vec::new(),
            cached_clip_search: Vec::new(),
            cached_clip_file_sizes: HashMap::new(),
            cached_app_search: Vec::new(),
            daemon_mode: false,
            visible: Arc::new(AtomicBool::new(false)),
            show_rx: None,
            shutdown_rx: None,
            daemon_paste_tx: None,
            history_update_rx: None,
            hide_command_sent: false,
        }
    }

    /// Constructor for daemon-mode (popup lives inside the daemon process).
    /// The window starts hidden; external channels drive show/hide/shutdown.
    fn new_daemon(
        _cc: &eframe::CreationContext<'_>,
        config: Config,
        show_rx: mpsc::Receiver<()>,
        shutdown_rx: mpsc::Receiver<()>,
        daemon_paste_tx: mpsc::Sender<ClipItem>,
        history_update_rx: mpsc::Receiver<Vec<ClipItem>>,
        visible: Arc<AtomicBool>,
    ) -> Self {
        // ── All heavy I/O happens in background threads ──
        // (Same background threads as new(), copied inline for simplicity)

        // History load + cache computation thread
        let (clip_tx, clip_rx) = std::sync::mpsc::channel();
        let preview_chars = config.general.preview_chars;
        std::thread::spawn(move || {
            let clips: Vec<ClipItem> = storage::load_history().into_iter().collect();
            let cached_clip_char_counts: Vec<usize> = clips
                .iter()
                .map(|item| match item {
                    ClipItem::Text { content, .. } => content.chars().count(),
                    _ => 0,
                })
                .collect();
            let cached_clip_previews: Vec<String> = clips
                .iter()
                .map(|item| match item {
                    ClipItem::Text { content, .. } => preview_text(content, preview_chars),
                    _ => String::new(),
                })
                .collect();
            let cached_clip_search: Vec<String> = clips
                .iter()
                .map(|item| match item {
                    ClipItem::Text { content, .. } => content.to_lowercase(),
                    ClipItem::Image {
                        width,
                        height,
                        filename,
                        ..
                    } => format!(
                        "{}\u{00d7}{} {}x{} {}",
                        width, height, width, height, filename
                    )
                    .to_lowercase(),
                })
                .collect();
            let mut cached_clip_file_sizes = HashMap::new();
            for item in &clips {
                if let ClipItem::Image { filename, .. } = item {
                    if !filename.is_empty() {
                        if let Ok(meta) = std::fs::metadata(Config::images_dir().join(filename)) {
                            cached_clip_file_sizes.insert(filename.clone(), meta.len());
                        }
                    }
                }
            }
            let _ = clip_tx.send(LoadedData {
                clips,
                cached_clip_char_counts,
                cached_clip_previews,
                cached_clip_search,
                cached_clip_file_sizes,
            });
        });

        // ── Async image thumbnail loading ──
        let (tx, rx) = std::sync::mpsc::channel();
        let clips_for_images: Vec<ClipItem> = storage::load_history().into_iter().collect();
        std::thread::spawn(move || {
            for item in clips_for_images {
                if let ClipItem::Image { filename, .. } = item {
                    if filename.is_empty() {
                        continue;
                    }
                    let images_dir = Config::images_dir();
                    let thumb_path = images_dir.join(format!("thumb_{}", filename));
                    if let Ok(img) = image::open(&thumb_path) {
                        let rgba = img.to_rgba8();
                        let size = [rgba.width() as usize, rgba.height() as usize];
                        let ci = egui::ColorImage::from_rgba_unmultiplied(size, rgba.as_raw());
                        let _ = tx.send((filename, ci));
                    } else {
                        let path = images_dir.join(&filename);
                        if let Ok(img) = image::open(path) {
                            let thumb = img.resize(52, 52, image::imageops::FilterType::Triangle);
                            let rgba = thumb.to_rgba8();
                            let size = [rgba.width() as usize, rgba.height() as usize];
                            let ci = egui::ColorImage::from_rgba_unmultiplied(size, rgba.as_raw());
                            let _ = thumb.save(&thumb_path);
                            let _ = tx.send((filename, ci));
                        }
                    }
                }
            }
        });

        // ── Async desktop app loading ──
        let (app_tx, app_rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            if let Some(cached) = crate::desktop::load_apps_cache() {
                let _ = app_tx.send(cached);
                crate::desktop::refresh_and_cache_apps();
            } else {
                let apps = crate::desktop::refresh_and_cache_apps();
                let _ = app_tx.send(apps);
            }
        });

        let theme_colors = ThemeColors::from_config(&config);

        Self {
            clips: Vec::new(),
            apps: Vec::new(),
            filtered: Vec::new(),
            query: String::new(),
            selected: 0,
            textures: HashMap::new(),
            app_icon_textures: HashMap::new(),
            should_paste: Arc::new(AtomicBool::new(false)),
            selected_item_out: Arc::new(std::sync::Mutex::new(None)),
            preview_chars: config.general.preview_chars,
            focus_search_once: true,
            scroll_to_selected_once: false,
            config,
            theme_colors,
            preview_image: None,
            lightbox_zoom: 1.0,
            lightbox_pan: egui::Vec2::ZERO,
            focused_once: false,
            rx,
            clip_rx,
            clips_loaded: false,
            app_rx,
            apps_loaded: false,
            cached_clip_char_counts: Vec::new(),
            cached_clip_previews: Vec::new(),
            cached_clip_search: Vec::new(),
            cached_clip_file_sizes: HashMap::new(),
            cached_app_search: Vec::new(),
            daemon_mode: true,
            visible,
            show_rx: Some(show_rx),
            shutdown_rx: Some(shutdown_rx),
            daemon_paste_tx: Some(daemon_paste_tx),
            history_update_rx: Some(history_update_rx),
            hide_command_sent: false,
        }
    }

    fn weak_color(&self, ui: &egui::Ui) -> egui::Color32 {
        self.theme_colors
            .as_ref()
            .map_or(ui.visuals().weak_text_color(), |t| t.weak_text_color)
    }

    fn hide_daemon_popup(&mut self, ctx: &egui::Context) {
        self.visible.store(false, Ordering::Relaxed);
        if !self.hide_command_sent {
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
            self.hide_command_sent = true;
        }
    }

    fn load_large_image(&self, ctx: &egui::Context, filename: &str) -> Option<egui::TextureHandle> {
        let path = Config::images_dir().join(filename);
        if let Ok(img) = image::open(path) {
            let large = img.resize(500, 500, image::imageops::FilterType::Triangle);
            let rgba = large.to_rgba8();
            let size = [rgba.width() as usize, rgba.height() as usize];
            let ci = egui::ColorImage::from_rgba_unmultiplied(size, rgba.as_raw());
            Some(ctx.load_texture(
                format!("large_{}", filename),
                ci,
                egui::TextureOptions::LINEAR,
            ))
        } else {
            None
        }
    }

    fn apply_filter(&mut self) {
        let q = self.query.trim().to_lowercase();
        let filtered: Vec<DisplayItem> = self
            .clips
            .iter()
            .enumerate()
            .filter_map(|(i, _)| {
                if q.is_empty() || self.cached_clip_search[i].contains(q.as_str()) {
                    Some(DisplayItem::Clip { clip_idx: i })
                } else {
                    None
                }
            })
            .chain(self.apps.iter().enumerate().filter_map(|(i, _app)| {
                if q.is_empty() || self.cached_app_search[i].contains(q.as_str()) {
                    Some(DisplayItem::App { app_idx: i })
                } else {
                    None
                }
            }))
            .collect();
        self.filtered = filtered;
        self.selected = self.selected.min(self.filtered.len().saturating_sub(1));
        self.scroll_to_selected_once = true;
    }

    /// Recompute all per-item caches after the item list changes.
    fn rebuild_caches(&mut self) {
        self.cached_clip_char_counts = self
            .clips
            .iter()
            .map(|item| match item {
                ClipItem::Text { content, .. } => content.chars().count(),
                _ => 0,
            })
            .collect();

        self.cached_clip_previews = self
            .clips
            .iter()
            .map(|item| match item {
                ClipItem::Text { content, .. } => preview_text(content, self.preview_chars),
                _ => String::new(),
            })
            .collect();

        self.cached_clip_search = self
            .clips
            .iter()
            .map(|item| match item {
                ClipItem::Text { content, .. } => content.to_lowercase(),
                ClipItem::Image {
                    width,
                    height,
                    filename,
                    ..
                } => format!(
                    "{}\u{00d7}{} {}x{} {}",
                    width, height, width, height, filename
                )
                .to_lowercase(),
            })
            .collect();

        self.cached_clip_file_sizes.clear();
        for item in &self.clips {
            if let ClipItem::Image { filename, .. } = item {
                if !filename.is_empty() && !self.cached_clip_file_sizes.contains_key(filename) {
                    if let Ok(meta) = std::fs::metadata(Config::images_dir().join(filename)) {
                        self.cached_clip_file_sizes
                            .insert(filename.clone(), meta.len());
                    }
                }
            }
        }

        self.cached_app_search = self
            .apps
            .iter()
            .map(|app| {
                format!(
                    "{} {} {}",
                    app.name.to_lowercase(),
                    app.comment.to_lowercase(),
                    app.exec.to_lowercase()
                )
            })
            .collect();
    }

    fn move_selection_down(&mut self) {
        if !self.filtered.is_empty() {
            self.selected = (self.selected + 1).min(self.filtered.len() - 1);
            self.scroll_to_selected_once = true;
        }
    }

    fn move_selection_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
            self.scroll_to_selected_once = true;
        }
    }

    fn select_and_close(&mut self, ctx: &egui::Context) {
        if let Some(item) = self.selected_clip() {
            if self.daemon_mode {
                // Daemon mode: send item via channel and hide window
                if let Some(ref tx) = self.daemon_paste_tx {
                    let _ = tx.send(item.clone());
                }
                self.hide_daemon_popup(ctx);
            } else {
                // Standalone mode: write to shared output and close process
                if let Ok(mut out) = self.selected_item_out.lock() {
                    *out = Some(item.clone());
                }
                self.should_paste.store(true, Ordering::Relaxed);
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        } else if let Some(DisplayItem::App { app_idx }) = self.filtered.get(self.selected) {
            // Launch app, keep popup open
            if let Some(app) = self.apps.get(*app_idx) {
                let exec = app.exec.clone();
                std::thread::spawn(move || {
                    let _ = std::process::Command::new("sh")
                        .arg("-c")
                        .arg(&exec)
                        .spawn();
                });
            }
        }
    }

    fn launch_selected_app(&self) {
        if let Some(DisplayItem::App { app_idx }) = self.filtered.get(self.selected) {
            if let Some(app) = self.apps.get(*app_idx) {
                let exec = app.exec.clone();
                std::thread::spawn(move || {
                    let _ = std::process::Command::new("sh")
                        .arg("-c")
                        .arg(&exec)
                        .spawn();
                });
            }
        }
    }

    fn selected_item(&self) -> Option<&ClipItem> {
        self.selected_clip()
    }

    fn selected_clip(&self) -> Option<&ClipItem> {
        if let Some(DisplayItem::Clip { clip_idx }) = self.filtered.get(self.selected) {
            self.clips.get(*clip_idx)
        } else {
            None
        }
    }

    fn delete_current(&mut self) {
        let Some(DisplayItem::Clip { clip_idx }) = self.filtered.get(self.selected) else {
            return; // apps can't be deleted
        };
        let orig_idx = *clip_idx;

        if let Some(ClipItem::Image { filename, .. }) = self.clips.get(orig_idx) {
            if !filename.is_empty() {
                storage::delete_image_file(filename);
                self.textures.remove(filename);
            }
        }

        self.clips.remove(orig_idx);
        self.persist_all();
        self.rebuild_caches();
        self.apply_filter();
    }

    fn clear_history(&mut self) {
        for item in &self.clips {
            if let ClipItem::Image { filename, .. } = item {
                if !filename.is_empty() {
                    storage::delete_image_file(filename);
                }
            }
        }
        self.clips.clear();
        self.query.clear();
        self.persist_all();
        self.textures.clear();
        self.cached_clip_char_counts.clear();
        self.cached_clip_previews.clear();
        self.cached_clip_search.clear();
        self.cached_clip_file_sizes.clear();
        self.apply_filter();
    }

    fn persist_all(&self) {
        let items: VecDeque<ClipItem> = self.clips.iter().cloned().collect();
        let _ = storage::save_history(&items);
    }

    fn draw_header(&mut self, ui: &mut egui::Ui) {
        let show_main = !self.config.general.hide_main_header;
        let show_sec = !self.config.general.hide_secondary_header;

        if !show_main && !show_sec {
            return;
        }

        egui::Frame::none()
            .inner_margin(egui::Margin::symmetric(20.0, 16.0))
            .show(ui, |ui| {
                ui.vertical(|ui| {
                    if show_main {
                        ui.label(egui::RichText::new("EasyCopy").heading().strong());
                    }
                    if show_sec {
                        if show_main {
                            ui.add_space(2.0);
                        }
                        ui.label(
                            egui::RichText::new("Clipboard history")
                                .size(13.0)
                                .color(self.weak_color(ui)),
                        );
                    }
                });
            });
    }

    fn draw_search(&mut self, ui: &mut egui::Ui) {
        let has_no_header =
            self.config.general.hide_main_header && self.config.general.hide_secondary_header;
        let top_margin = if has_no_header { 18.0 } else { 8.0 };

        egui::Frame::none()
            .inner_margin(egui::Margin {
                left: 20.0,
                right: 20.0,
                top: top_margin,
                bottom: 8.0,
            })
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    let label_text =
                        format!("{} clips · {} apps", self.clips.len(), self.apps.len());
                    let label_width = 75.0;
                    let search_width = (ui.available_width() - label_width - 8.0).max(100.0);

                    let bg_fill = ui.visuals().extreme_bg_color;
                    let stroke = ui.visuals().widgets.noninteractive.bg_stroke;

                    egui::Frame::none()
                        .fill(bg_fill)
                        .stroke(stroke)
                        .rounding(10.0)
                        .inner_margin(egui::Margin::symmetric(12.0, 8.0))
                        .show(ui, |ui| {
                            ui.set_width(search_width);
                            ui.horizontal(|ui| {
                                // Search icon
                                let (icon_rect, _) = ui.allocate_exact_size(
                                    egui::vec2(16.0, 16.0),
                                    egui::Sense {
                                        click: false,
                                        drag: false,
                                        focusable: false,
                                    },
                                );
                                theme::paint_search_icon(ui, icon_rect, self.weak_color(ui));

                                ui.add_space(6.0);

                                let available_width = if self.query.is_empty() {
                                    ui.available_width()
                                } else {
                                    (ui.available_width() - 32.0).max(100.0)
                                };

                                let response = ui.add_sized(
                                    [available_width, 22.0],
                                    egui::TextEdit::singleline(&mut self.query)
                                        .font(egui::TextStyle::Body)
                                        .frame(false)
                                        .hint_text(SEARCH_HINT),
                                );

                                if self.focus_search_once {
                                    response.request_focus();
                                    self.focus_search_once = false;
                                }

                                if response.changed() {
                                    self.apply_filter();
                                    self.selected = 0;
                                }

                                if !self.query.is_empty() {
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            let (btn_rect, btn_resp) = ui.allocate_exact_size(
                                                egui::vec2(18.0, 18.0),
                                                egui::Sense::click(),
                                            );
                                            let btn_color = if btn_resp.hovered() {
                                                ui.visuals().text_color()
                                            } else {
                                                self.weak_color(ui)
                                            };
                                            let close_icon_rect = egui::Rect::from_center_size(
                                                btn_rect.center(),
                                                egui::vec2(8.0, 8.0),
                                            );
                                            theme::paint_close_icon(ui, close_icon_rect, btn_color);
                                            if btn_resp.clicked() {
                                                self.query.clear();
                                                self.apply_filter();
                                            }
                                        },
                                    );
                                }
                            });
                        });

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            egui::RichText::new(label_text)
                                .size(13.0)
                                .color(self.weak_color(ui)),
                        );
                    });
                });
            });
    }

    fn draw_body(&mut self, ui: &mut egui::Ui) {
        egui::Frame::none()
            .inner_margin(egui::Margin::symmetric(20.0, 6.0))
            .show(ui, |ui| {
                let weak_color = self.weak_color(ui);

                if !self.clips_loaded {
                    draw_empty_state(ui, "Loading…", "Reading clipboard history.", weak_color);
                    ui.ctx().request_repaint(); // keep animating until loaded
                    return;
                }

                if self.clips.is_empty() && self.apps.is_empty() {
                    draw_empty_state(
                        ui,
                        "No clips yet",
                        "Copy text or an image while the daemon is running.",
                        weak_color,
                    );
                    return;
                }

                if self.filtered.is_empty() {
                    draw_empty_state(ui, "No matches", "Try a shorter search term.", weak_color);
                    return;
                }

                egui::ScrollArea::vertical()
                    .id_source("clipit_clip_list")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.spacing_mut().item_spacing.y = 8.0;
                        ui.set_width(ui.available_width());
                        for row in 0..self.filtered.len() {
                            if self.draw_display_row(ui, row) {
                                self.selected = row;
                                // Only close popup if a clip was selected (apps keep popup open)
                                if matches!(self.filtered[row], DisplayItem::Clip { .. }) {
                                    self.select_and_close(ui.ctx());
                                    break;
                                }
                            }
                        }
                    });
            });
    }

    fn draw_footer(&mut self, ui: &mut egui::Ui) {
        egui::Frame::none()
            .inner_margin(egui::Margin::symmetric(20.0, 12.0))
            .show(ui, |ui| {
                ui.horizontal_centered(|ui| {
                    if self.config.footer.show_help {
                        ui.label(
                            egui::RichText::new(FOOTER_HELP)
                                .size(12.5)
                                .color(self.weak_color(ui)),
                        );
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let mut first_drawn = false;

                        if self.config.footer.show_clear {
                            // Clear History Button (icon-only)
                            let enabled = !self.clips.is_empty();
                            let clear_btn_rect = ui
                                .allocate_exact_size(egui::vec2(32.0, 32.0), egui::Sense::click())
                                .0;
                            let clear_resp = ui.interact(
                                clear_btn_rect,
                                ui.id().with("clear_history_btn"),
                                egui::Sense::click(),
                            );

                            let fill = if !enabled {
                                ui.visuals().widgets.noninteractive.bg_fill
                            } else if clear_resp.clicked() {
                                ui.visuals().widgets.active.bg_fill
                            } else if clear_resp.hovered() {
                                ui.visuals().widgets.hovered.bg_fill
                            } else {
                                ui.visuals().widgets.inactive.bg_fill
                            };

                            let stroke = if !enabled {
                                ui.visuals().widgets.noninteractive.bg_stroke
                            } else if clear_resp.hovered() {
                                ui.visuals().widgets.hovered.bg_stroke
                            } else {
                                ui.visuals().widgets.noninteractive.bg_stroke
                            };

                            ui.painter().rect(
                                clear_btn_rect,
                                egui::Rounding::same(8.0),
                                fill,
                                stroke,
                            );

                            let text_color = if !enabled {
                                self.weak_color(ui)
                            } else {
                                ui.visuals().text_color()
                            };

                            let icon_rect = egui::Rect::from_center_size(
                                clear_btn_rect.center(),
                                egui::vec2(14.0, 14.0),
                            );
                            theme::paint_trash_icon(ui, icon_rect, text_color);

                            if enabled && clear_resp.clicked() {
                                self.clear_history();
                            }
                            first_drawn = true;
                        }

                        if self.config.footer.show_settings {
                            if first_drawn {
                                ui.add_space(8.0);
                            }

                            // Settings Button (opens config)
                            let settings_btn_rect = ui
                                .allocate_exact_size(egui::vec2(32.0, 32.0), egui::Sense::click())
                                .0;
                            let settings_resp = ui.interact(
                                settings_btn_rect,
                                ui.id().with("settings_btn"),
                                egui::Sense::click(),
                            );

                            let settings_fill = if settings_resp.clicked() {
                                ui.visuals().widgets.active.bg_fill
                            } else if settings_resp.hovered() {
                                ui.visuals().widgets.hovered.bg_fill
                            } else {
                                ui.visuals().widgets.inactive.bg_fill
                            };

                            let settings_stroke = if settings_resp.hovered() {
                                ui.visuals().widgets.hovered.bg_stroke
                            } else {
                                ui.visuals().widgets.noninteractive.bg_stroke
                            };

                            ui.painter().rect(
                                settings_btn_rect,
                                egui::Rounding::same(8.0),
                                settings_fill,
                                settings_stroke,
                            );

                            let settings_color = ui.visuals().text_color();

                            let settings_icon_rect = egui::Rect::from_center_size(
                                settings_btn_rect.center(),
                                egui::vec2(16.0, 16.0),
                            );
                            theme::paint_settings_icon(ui, settings_icon_rect, settings_color);

                            if settings_resp.clicked() {
                                let path = Config::config_path();
                                let _ = std::process::Command::new("xdg-open").arg(path).spawn();
                            }
                            first_drawn = true;
                        }

                        if self.config.footer.show_theme {
                            if first_drawn {
                                ui.add_space(8.0);
                            }

                            // Theme Selector Button (opens dropdown)
                            let theme_btn_rect = ui
                                .allocate_exact_size(egui::vec2(32.0, 32.0), egui::Sense::click())
                                .0;
                            let theme_resp = ui.interact(
                                theme_btn_rect,
                                ui.id().with("theme_selector_btn"),
                                egui::Sense::click(),
                            );

                            let theme_fill = if theme_resp.clicked() {
                                ui.visuals().widgets.active.bg_fill
                            } else if theme_resp.hovered() {
                                ui.visuals().widgets.hovered.bg_fill
                            } else {
                                ui.visuals().widgets.inactive.bg_fill
                            };

                            let theme_stroke = if theme_resp.hovered() {
                                ui.visuals().widgets.hovered.bg_stroke
                            } else {
                                ui.visuals().widgets.noninteractive.bg_stroke
                            };

                            ui.painter().rect(
                                theme_btn_rect,
                                egui::Rounding::same(8.0),
                                theme_fill,
                                theme_stroke,
                            );

                            let theme_color = ui.visuals().text_color();

                            let theme_icon_rect = egui::Rect::from_center_size(
                                theme_btn_rect.center(),
                                egui::vec2(16.0, 16.0),
                            );
                            theme::paint_palette_icon(ui, theme_icon_rect, theme_color);

                            let popup_id = ui.make_persistent_id("theme_dropdown");
                            if theme_resp.clicked() {
                                ui.memory_mut(|mem| mem.toggle_popup(popup_id));
                            }

                            egui::popup_below_widget(
                                ui,
                                popup_id,
                                &theme_resp,
                                egui::PopupCloseBehavior::CloseOnClick,
                                |ui| {
                                    let draw_item = |ui: &mut egui::Ui,
                                                     label: &str,
                                                     is_selected: bool,
                                                     fg: egui::Color32,
                                                     tc: Option<&ThemeColors>|
                                     -> bool {
                                        let (rect, response) = ui.allocate_exact_size(
                                            egui::vec2(ui.available_width(), 26.0),
                                            egui::Sense::click(),
                                        );

                                        let bg = if is_selected {
                                            tc.map_or(ui.visuals().selection.bg_fill, |c| {
                                                c.selection_bg
                                            })
                                        } else if response.hovered() {
                                            tc.map_or(ui.visuals().widgets.hovered.bg_fill, |c| {
                                                c.widget_hovered_bg
                                            })
                                        } else {
                                            egui::Color32::TRANSPARENT
                                        };

                                        ui.painter().rect_filled(
                                            rect,
                                            egui::Rounding::same(6.0),
                                            bg,
                                        );

                                        let text_pos = rect.left_center() + egui::vec2(8.0, 0.0);
                                        ui.painter().text(
                                            text_pos,
                                            egui::Align2::LEFT_CENTER,
                                            label,
                                            egui::FontId::proportional(14.0),
                                            fg,
                                        );

                                        response.clicked()
                                    };

                                    ui.set_width(160.0);
                                    ui.spacing_mut().item_spacing.y = 2.0;

                                    // ── Themes ──
                                    ui.label(
                                        egui::RichText::new("THEMES").size(11.0).color(
                                            self.theme_colors
                                                .as_ref()
                                                .map_or(ui.visuals().weak_text_color(), |c| {
                                                    c.weak_text_color
                                                }),
                                        ),
                                    );
                                    ui.separator();
                                    let themes = ["dark", "light", "nord", "catppuccin", "dracula"];
                                    for t_name in &themes {
                                        let selected = self.config.general.theme == *t_name;
                                        let fg = if selected {
                                            egui::Color32::WHITE
                                        } else {
                                            self.theme_colors
                                                .as_ref()
                                                .map_or(ui.visuals().text_color(), |c| c.text_color)
                                        };
                                        if draw_item(
                                            ui,
                                            t_name,
                                            selected,
                                            fg,
                                            self.theme_colors.as_ref(),
                                        ) {
                                            self.config.general.theme = t_name.to_string();
                                            theme::apply_theme_and_fonts(ui.ctx(), &self.config);
                                            self.theme_colors =
                                                ThemeColors::from_config(&self.config);
                                            let _ = self.config.save();
                                            ui.close_menu();
                                        }
                                    }

                                    // ── Fonts ──
                                    ui.add_space(6.0);
                                    ui.label(
                                        egui::RichText::new("FONTS").size(11.0).color(
                                            self.theme_colors
                                                .as_ref()
                                                .map_or(ui.visuals().weak_text_color(), |c| {
                                                    c.weak_text_color
                                                }),
                                        ),
                                    );
                                    ui.separator();
                                    let font_presets = [
                                        "default",
                                        "dejavu",
                                        "liberation",
                                        "fira",
                                        "jetbrains",
                                        "iosevka",
                                    ];
                                    let display_names = [
                                        "System Default",
                                        "DejaVu",
                                        "Liberation",
                                        "Fira Code",
                                        "JetBrains Mono",
                                        "Iosevka",
                                    ];
                                    for (i, f_name) in font_presets.iter().enumerate() {
                                        let available = f_name == &"default"
                                            || theme::is_font_preset_available(f_name);
                                        let selected = self.config.general.font_preset == *f_name;
                                        let label = if available {
                                            display_names[i].to_string()
                                        } else {
                                            format!("{} (not installed)", display_names[i])
                                        };
                                        let fg = if selected {
                                            egui::Color32::WHITE
                                        } else if !available {
                                            self.theme_colors
                                                .as_ref()
                                                .map_or(egui::Color32::GRAY, |c| c.weak_text_color)
                                        } else {
                                            self.theme_colors
                                                .as_ref()
                                                .map_or(ui.visuals().text_color(), |c| c.text_color)
                                        };
                                        if draw_item(
                                            ui,
                                            &label,
                                            selected,
                                            fg,
                                            self.theme_colors.as_ref(),
                                        ) {
                                            if available || selected {
                                                self.config.general.font_preset =
                                                    f_name.to_string();
                                                theme::apply_theme_and_fonts(
                                                    ui.ctx(),
                                                    &self.config,
                                                );
                                                let _ = self.config.save();
                                                ui.close_menu();
                                            }
                                        }
                                    }

                                    // ── Font Size ──
                                    ui.add_space(6.0);
                                    ui.label(
                                        egui::RichText::new("FONT SIZE").size(11.0).color(
                                            self.theme_colors
                                                .as_ref()
                                                .map_or(ui.visuals().weak_text_color(), |c| {
                                                    c.weak_text_color
                                                }),
                                        ),
                                    );
                                    ui.separator();
                                    let sizes = ["small", "medium", "large"];
                                    let size_display = ["Small", "Medium", "Large"];
                                    for (i, s_name) in sizes.iter().enumerate() {
                                        let selected = self.config.general.font_size == *s_name;
                                        let fg = if selected {
                                            egui::Color32::WHITE
                                        } else {
                                            self.theme_colors
                                                .as_ref()
                                                .map_or(ui.visuals().text_color(), |c| c.text_color)
                                        };
                                        if draw_item(
                                            ui,
                                            size_display[i],
                                            selected,
                                            fg,
                                            self.theme_colors.as_ref(),
                                        ) {
                                            self.config.general.font_size = s_name.to_string();
                                            theme::apply_theme_and_fonts(ui.ctx(), &self.config);
                                            let _ = self.config.save();
                                            ui.close_menu();
                                        }
                                    }

                                    // ── Font Weight ──
                                    ui.add_space(6.0);
                                    ui.label(
                                        egui::RichText::new("FONT WEIGHT").size(11.0).color(
                                            self.theme_colors
                                                .as_ref()
                                                .map_or(ui.visuals().weak_text_color(), |c| {
                                                    c.weak_text_color
                                                }),
                                        ),
                                    );
                                    ui.separator();
                                    let weights = ["normal", "bold"];
                                    let weight_display = ["Normal", "Bold"];
                                    for (i, w_name) in weights.iter().enumerate() {
                                        let selected = self.config.general.font_weight == *w_name;
                                        let fg = if selected {
                                            egui::Color32::WHITE
                                        } else {
                                            self.theme_colors
                                                .as_ref()
                                                .map_or(ui.visuals().text_color(), |c| c.text_color)
                                        };
                                        if draw_item(
                                            ui,
                                            weight_display[i],
                                            selected,
                                            fg,
                                            self.theme_colors.as_ref(),
                                        ) {
                                            self.config.general.font_weight = w_name.to_string();
                                            theme::apply_theme_and_fonts(ui.ctx(), &self.config);
                                            let _ = self.config.save();
                                            ui.close_menu();
                                        }
                                    }
                                },
                            );
                        }
                    });
                });
            });
    }

    fn draw_display_row(&mut self, ui: &mut egui::Ui, row: usize) -> bool {
        match self.filtered[row] {
            DisplayItem::Clip { clip_idx } => {
                let Some(item) = self.clips.get(clip_idx).cloned() else {
                    return false;
                };
                self.draw_clip_row(ui, row, clip_idx, item)
            }
            DisplayItem::App { app_idx } => {
                let Some(app) = self.apps.get(app_idx).cloned() else {
                    return false;
                };
                self.draw_app_row(ui, row, app_idx, app)
            }
        }
    }

    fn draw_clip_row(
        &mut self,
        ui: &mut egui::Ui,
        row: usize,
        orig_idx: usize,
        item: ClipItem,
    ) -> bool {
        let is_selected = row == self.selected;
        let row_height = match &item {
            ClipItem::Image { .. } => 84.0,
            ClipItem::Text { .. } => 76.0,
        };

        let card_id = ui.make_persistent_id(format!("row_{}", row));
        let rect = egui::Rect::from_min_size(
            ui.next_widget_position(),
            egui::vec2(ui.available_width(), row_height),
        );
        let response = ui.interact(rect, card_id, egui::Sense::click());

        if response.secondary_clicked() {
            if let ClipItem::Image { filename, .. } = &item {
                if !filename.is_empty() {
                    if let Some(tex) = self.load_large_image(ui.ctx(), filename) {
                        self.lightbox_zoom = 1.0;
                        self.lightbox_pan = egui::Vec2::ZERO;
                        self.preview_image = Some((filename.clone(), tex));
                    }
                }
            }
        }

        if is_selected && self.scroll_to_selected_once {
            ui.scroll_to_rect(rect, Some(egui::Align::Center));
            self.scroll_to_selected_once = false;
        }

        let theme = self.theme_colors.as_ref();

        let fill = if is_selected {
            theme.map_or_else(|| ui.visuals().selection.bg_fill, |t| t.card_bg_selected)
        } else if response.hovered() {
            theme.map_or_else(
                || ui.visuals().widgets.hovered.bg_fill,
                |t| t.card_bg_hovered,
            )
        } else {
            theme.map_or_else(
                || ui.visuals().widgets.noninteractive.bg_fill,
                |t| t.card_bg,
            )
        };

        let stroke = if is_selected {
            theme.map_or_else(
                || egui::Stroke::new(1.5, ui.visuals().selection.bg_fill),
                |t| egui::Stroke::new(1.5, t.card_stroke_selected),
            )
        } else {
            egui::Stroke::new(1.0, ui.visuals().widgets.noninteractive.bg_stroke.color)
        };

        let rounding = theme.map_or(0.0, |t| t.card_rounding);

        egui::Frame::none()
            .fill(fill)
            .rounding(rounding)
            .stroke(stroke)
            .inner_margin(egui::Margin::symmetric(14.0, 10.0))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.set_height(row_height - 20.0);

                if is_selected && self.config.general.enable_theming {
                    let bar_width = 4.0;
                    let bar_height = 24.0;
                    let bar_rect = egui::Rect::from_center_size(
                        egui::pos2(ui.min_rect().left() - 12.0, ui.min_rect().center().y),
                        egui::vec2(bar_width, bar_height),
                    );
                    ui.painter().rect_filled(
                        bar_rect,
                        egui::Rounding::same(2.0),
                        self.theme_colors
                            .as_ref()
                            .map_or(egui::Color32::from_rgb(99, 102, 241), |t| t.selection_bar),
                    );
                }

                self.draw_clip_row_inner(ui, row, orig_idx, item, is_selected, row_height);
            });

        response.clicked()
    }

    fn draw_clip_row_inner(
        &mut self,
        ui: &mut egui::Ui,
        row: usize,
        orig_idx: usize,
        item: ClipItem,
        is_selected: bool,
        row_height: f32,
    ) {
        match &item {
            ClipItem::Text { content, timestamp } => {
                ui.allocate_ui(egui::vec2(ui.available_width(), row_height - 20.0), |ui| {
                    ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                        theme::draw_icon_badge(ui, "text", is_selected, self.theme_colors.as_ref());
                        ui.add_space(8.0);

                        let available_width = (ui.available_width() - 40.0).max(120.0);
                        ui.allocate_ui(egui::vec2(available_width, row_height - 20.0), |ui| {
                            ui.vertical(|ui| {
                                let content_height = 35.0;
                                let available_h = ui.available_height();
                                if available_h > content_height {
                                    ui.add_space((available_h - content_height) / 2.0);
                                }
                                ui.set_width(available_width);
                                let preview = self
                                    .cached_clip_previews
                                    .get(orig_idx)
                                    .cloned()
                                    .unwrap_or_else(|| preview_text(content, self.preview_chars));
                                ui.add(
                                    egui::Label::new(
                                        egui::RichText::new(preview)
                                            .size(15.0)
                                            .monospace()
                                            .strong(),
                                    )
                                    .truncate(),
                                );
                                ui.add_space(2.0);
                                let char_count = self
                                    .cached_clip_char_counts
                                    .get(orig_idx)
                                    .copied()
                                    .unwrap_or_else(|| content.chars().count());
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{} chars · {}",
                                        char_count,
                                        relative_time(*timestamp)
                                    ))
                                    .size(12.5)
                                    .color(self.weak_color(ui)),
                                );
                            });
                        });

                        if row < 9 {
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    let shortcut_color = if is_selected {
                                        self.theme_colors.as_ref().map_or(
                                            egui::Color32::from_rgba_unmultiplied(
                                                255, 255, 255, 180,
                                            ),
                                            |t| t.shortcut_color,
                                        )
                                    } else {
                                        self.weak_color(ui)
                                    };
                                    ui.label(
                                        egui::RichText::new(format!("^{}", row + 1))
                                            .size(13.0)
                                            .monospace()
                                            .color(shortcut_color),
                                    );
                                },
                            );
                        }
                    });
                });
            }
            ClipItem::Image {
                width,
                height,
                timestamp,
                filename,
                ..
            } => {
                ui.allocate_ui(egui::vec2(ui.available_width(), row_height - 20.0), |ui| {
                    ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                        if let Some(tex) = self.textures.get(filename) {
                            let img = egui::Image::new(tex)
                                .max_size(egui::vec2(36.0, 36.0))
                                .rounding(egui::Rounding::same(6.0));
                            ui.add(img);
                        } else {
                            theme::draw_icon_badge(
                                ui,
                                "image",
                                is_selected,
                                self.theme_colors.as_ref(),
                            );
                        }
                        ui.add_space(12.0);

                        let available_width = (ui.available_width() - 40.0).max(120.0);
                        ui.allocate_ui(egui::vec2(available_width, row_height - 20.0), |ui| {
                            ui.vertical(|ui| {
                                let content_height = 35.0;
                                let available_h = ui.available_height();
                                if available_h > content_height {
                                    ui.add_space((available_h - content_height) / 2.0);
                                }
                                ui.set_width(available_width);
                                ui.add(
                                    egui::Label::new(
                                        egui::RichText::new(format!("Image {}×{}", width, height))
                                            .size(15.0)
                                            .strong(),
                                    )
                                    .truncate(),
                                );
                                ui.add_space(2.0);
                                let file_size = self.cached_clip_file_sizes.get(filename).copied();
                                ui.label(
                                    egui::RichText::new(image_subtitle_cached(
                                        filename, *timestamp, file_size,
                                    ))
                                    .size(12.5)
                                    .color(self.weak_color(ui)),
                                );
                            });
                        });

                        if row < 9 {
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    let shortcut_color = if is_selected {
                                        self.theme_colors.as_ref().map_or(
                                            egui::Color32::from_rgba_unmultiplied(
                                                255, 255, 255, 180,
                                            ),
                                            |t| t.shortcut_color,
                                        )
                                    } else {
                                        self.weak_color(ui)
                                    };
                                    ui.label(
                                        egui::RichText::new(format!("^{}", row + 1))
                                            .size(13.0)
                                            .monospace()
                                            .color(shortcut_color),
                                    );
                                },
                            );
                        }
                    });
                });
            }
        }
    }

    fn draw_app_row(
        &mut self,
        ui: &mut egui::Ui,
        row: usize,
        app_idx: usize,
        app: DesktopApp,
    ) -> bool {
        let is_selected = row == self.selected;
        let row_height = 68.0;

        let card_id = ui.make_persistent_id(format!("app_row_{}", row));
        let rect = egui::Rect::from_min_size(
            ui.next_widget_position(),
            egui::vec2(ui.available_width(), row_height),
        );
        let response = ui.interact(rect, card_id, egui::Sense::click());

        if is_selected && self.scroll_to_selected_once {
            ui.scroll_to_rect(rect, Some(egui::Align::Center));
            self.scroll_to_selected_once = false;
        }

        let theme = self.theme_colors.as_ref();

        let fill = if is_selected {
            theme.map_or_else(|| ui.visuals().selection.bg_fill, |t| t.card_bg_selected)
        } else if response.hovered() {
            theme.map_or_else(
                || ui.visuals().widgets.hovered.bg_fill,
                |t| t.card_bg_hovered,
            )
        } else {
            theme.map_or_else(
                || ui.visuals().widgets.noninteractive.bg_fill,
                |t| t.card_bg,
            )
        };

        let stroke = if is_selected {
            theme.map_or_else(
                || egui::Stroke::new(1.5, ui.visuals().selection.bg_fill),
                |t| egui::Stroke::new(1.5, t.card_stroke_selected),
            )
        } else {
            egui::Stroke::new(1.0, ui.visuals().widgets.noninteractive.bg_stroke.color)
        };

        let rounding = theme.map_or(0.0, |t| t.card_rounding);

        egui::Frame::none()
            .fill(fill)
            .rounding(rounding)
            .stroke(stroke)
            .inner_margin(egui::Margin::symmetric(14.0, 10.0))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.set_height(row_height - 20.0);

                if is_selected && self.config.general.enable_theming {
                    let bar_width = 4.0;
                    let bar_height = 24.0;
                    let bar_rect = egui::Rect::from_center_size(
                        egui::pos2(ui.min_rect().left() - 12.0, ui.min_rect().center().y),
                        egui::vec2(bar_width, bar_height),
                    );
                    ui.painter().rect_filled(
                        bar_rect,
                        egui::Rounding::same(2.0),
                        self.theme_colors
                            .as_ref()
                            .map_or(egui::Color32::from_rgb(99, 102, 241), |t| t.selection_bar),
                    );
                }

                ui.horizontal(|ui| {
                    if let Some(ref icon_path) = app.icon_path {
                        if let Some(tex) = self.app_icon_textures.get(icon_path) {
                            ui.add(
                                egui::Image::new(tex)
                                    .max_size(egui::vec2(36.0, 36.0))
                                    .rounding(egui::Rounding::same(6.0)),
                            );
                        } else if let Ok(img) = image::open(icon_path) {
                            let rgba = img.to_rgba8();
                            let ci = egui::ColorImage::from_rgba_unmultiplied(
                                [rgba.width() as usize, rgba.height() as usize],
                                rgba.as_raw(),
                            );
                            let tex = ui.ctx().load_texture(
                                format!("app_{}", app_idx),
                                ci,
                                egui::TextureOptions::LINEAR,
                            );
                            self.app_icon_textures
                                .insert(icon_path.clone(), tex.clone());
                            ui.add(
                                egui::Image::new(&tex)
                                    .max_size(egui::vec2(36.0, 36.0))
                                    .rounding(egui::Rounding::same(6.0)),
                            );
                        } else {
                            theme::draw_icon_badge(
                                ui,
                                "application",
                                is_selected,
                                self.theme_colors.as_ref(),
                            );
                        }
                    } else {
                        theme::draw_icon_badge(
                            ui,
                            "application",
                            is_selected,
                            self.theme_colors.as_ref(),
                        );
                    }
                    ui.add_space(8.0);

                    ui.vertical(|ui| {
                        ui.set_width((ui.available_width() - 40.0).max(120.0));
                        ui.add(
                            egui::Label::new(egui::RichText::new(&app.name).size(15.0).strong())
                                .truncate(),
                        );
                        if !app.comment.is_empty() {
                            ui.add_space(2.0);
                            ui.add(
                                egui::Label::new(
                                    egui::RichText::new(&app.comment)
                                        .size(12.5)
                                        .color(self.weak_color(ui)),
                                )
                                .truncate(),
                            );
                        }
                    });
                });
            });

        response.clicked()
    }

    fn draw_lightbox(&mut self, ui: &mut egui::Ui) {
        let Some((ref filename, ref texture)) = self.preview_image else {
            return;
        };
        let texture = texture.clone();
        let filename = filename.clone();

        let mut close_preview = false;
        let ctx = ui.ctx();

        egui::Area::new(egui::Id::new("lightbox_overlay"))
            .order(egui::Order::Foreground)
            .fixed_pos(egui::pos2(0.0, 0.0))
            .show(ctx, |ui| {
                let screen_rect = ctx.screen_rect();
                let backdrop_response = ui.allocate_rect(screen_rect, egui::Sense::click());

                // Dim background
                let bg_color = self.theme_colors.as_ref().map_or(
                    egui::Color32::from_rgba_unmultiplied(11, 15, 25, 220),
                    |t| t.lightbox_overlay,
                );
                let rounding = self.theme_colors.as_ref().map_or(0.0, |t| t.card_rounding);
                ui.painter()
                    .rect_filled(screen_rect, egui::Rounding::same(rounding), bg_color);

                // Calculate original preview size (fits screen with padding)
                let max_img_size =
                    egui::vec2(screen_rect.width() - 80.0, screen_rect.height() - 100.0);

                let img_size = texture.size_vec2();
                let scale = (max_img_size.x / img_size.x)
                    .min(max_img_size.y / img_size.y)
                    .min(1.0);
                let scaled_size = img_size * scale;

                // Apply zoom and pan to get final image rect
                let current_size = scaled_size * self.lightbox_zoom;
                let current_center = screen_rect.center() + self.lightbox_pan;
                let image_rect = egui::Rect::from_center_size(current_center, current_size);

                // Draw image
                let img = egui::Image::new(&texture)
                    .fit_to_exact_size(current_size)
                    .rounding(egui::Rounding::same(8.0));
                ui.put(image_rect, img);

                // Interact with image for panning/double-clicking
                let image_response = ui.interact(
                    image_rect,
                    egui::Id::new("lightbox_image_interact"),
                    egui::Sense::click_and_drag(),
                );

                if image_response.dragged() {
                    self.lightbox_pan += image_response.drag_delta();
                }

                if image_response.double_clicked() {
                    self.lightbox_zoom = 1.0;
                    self.lightbox_pan = egui::Vec2::ZERO;
                }

                // Backdrop click closes the preview only if click is outside the image rect
                if backdrop_response.clicked() {
                    if let Some(hover_pos) = ctx.input(|i| i.pointer.hover_pos()) {
                        if !image_rect.contains(hover_pos) {
                            close_preview = true;
                        }
                    }
                }

                // Close button at top-right of the screen
                let btn_size = egui::vec2(28.0, 28.0);
                let close_btn_pos =
                    egui::pos2(screen_rect.right() - 36.0, screen_rect.top() + 36.0);
                let close_rect = egui::Rect::from_center_size(close_btn_pos, btn_size);

                let close_response = ui.interact(
                    close_rect,
                    egui::Id::new("lightbox_close_btn"),
                    egui::Sense::click(),
                );
                let btn_bg = if close_response.clicked() {
                    ui.visuals().widgets.active.bg_fill
                } else if close_response.hovered() {
                    ui.visuals().widgets.hovered.bg_fill
                } else {
                    self.theme_colors.as_ref().map_or(
                        egui::Color32::from_rgba_unmultiplied(255, 255, 255, 30),
                        |t| t.lightbox_close_btn_bg,
                    )
                };

                ui.painter()
                    .circle_filled(close_rect.center(), 14.0, btn_bg);
                let close_icon_rect =
                    egui::Rect::from_center_size(close_rect.center(), egui::vec2(10.0, 10.0));
                theme::paint_close_icon(ui, close_icon_rect, egui::Color32::WHITE);

                if close_response.clicked() {
                    close_preview = true;
                }

                // Control bar panel at the bottom center of the screen
                let control_bar_rect = egui::Rect::from_center_size(
                    egui::pos2(screen_rect.center().x, screen_rect.bottom() - 50.0),
                    egui::vec2(250.0, 36.0),
                );

                let control_bg = self
                    .theme_colors
                    .as_ref()
                    .map_or(ui.visuals().widgets.inactive.bg_fill, |t| {
                        t.lightbox_control_bg
                    });

                ui.painter().rect(
                    control_bar_rect,
                    egui::Rounding::same(18.0),
                    control_bg,
                    egui::Stroke::new(1.0, ui.visuals().widgets.noninteractive.bg_stroke.color),
                );

                ui.allocate_ui_at_rect(control_bar_rect, |ui| {
                    ui.spacing_mut().item_spacing.x = 0.0;
                    ui.horizontal_centered(|ui| {
                        ui.add_space(16.0);

                        // Zoom Out (-)
                        let (minus_rect, minus_resp) =
                            ui.allocate_exact_size(egui::vec2(24.0, 24.0), egui::Sense::click());
                        let minus_color = if minus_resp.hovered() {
                            self.theme_colors
                                .as_ref()
                                .map_or(egui::Color32::WHITE, |t| t.lightbox_icon_hovered)
                        } else {
                            self.theme_colors
                                .as_ref()
                                .map_or(egui::Color32::from_rgb(200, 200, 200), |t| t.lightbox_icon)
                        };
                        let minus_stroke = egui::Stroke::new(2.0, minus_color);
                        ui.painter().line_segment(
                            [
                                egui::pos2(minus_rect.left() + 6.0, minus_rect.center().y),
                                egui::pos2(minus_rect.right() - 6.0, minus_rect.center().y),
                            ],
                            minus_stroke,
                        );
                        if minus_resp.clicked() {
                            self.lightbox_zoom = (self.lightbox_zoom / 1.2).clamp(0.2, 10.0);
                            ctx.request_repaint();
                        }

                        ui.add_space(6.0);

                        // Zoom percentage / Reset
                        let percent_text =
                            format!("{}%", (self.lightbox_zoom * 100.0).round() as i32);
                        let (lbl_rect, lbl_resp) =
                            ui.allocate_exact_size(egui::vec2(50.0, 24.0), egui::Sense::click());
                        let text_color = if lbl_resp.hovered() {
                            self.theme_colors
                                .as_ref()
                                .map_or(egui::Color32::WHITE, |t| t.lightbox_icon_hovered)
                        } else {
                            self.theme_colors
                                .as_ref()
                                .map_or(egui::Color32::from_rgb(200, 200, 200), |t| t.lightbox_icon)
                        };
                        ui.painter().text(
                            lbl_rect.center(),
                            egui::Align2::CENTER_CENTER,
                            percent_text,
                            egui::FontId::proportional(13.0),
                            text_color,
                        );
                        if lbl_resp.clicked() {
                            self.lightbox_zoom = 1.0;
                            self.lightbox_pan = egui::Vec2::ZERO;
                            ctx.request_repaint();
                        }

                        ui.add_space(6.0);

                        // Zoom In (+)
                        let (plus_rect, plus_resp) =
                            ui.allocate_exact_size(egui::vec2(24.0, 24.0), egui::Sense::click());
                        let plus_color = if plus_resp.hovered() {
                            self.theme_colors
                                .as_ref()
                                .map_or(egui::Color32::WHITE, |t| t.lightbox_icon_hovered)
                        } else {
                            self.theme_colors
                                .as_ref()
                                .map_or(egui::Color32::from_rgb(200, 200, 200), |t| t.lightbox_icon)
                        };
                        let plus_stroke = egui::Stroke::new(2.0, plus_color);
                        ui.painter().line_segment(
                            [
                                egui::pos2(plus_rect.left() + 6.0, plus_rect.center().y),
                                egui::pos2(plus_rect.right() - 6.0, plus_rect.center().y),
                            ],
                            plus_stroke,
                        );
                        ui.painter().line_segment(
                            [
                                egui::pos2(plus_rect.center().x, plus_rect.top() + 6.0),
                                egui::pos2(plus_rect.center().x, plus_rect.bottom() - 6.0),
                            ],
                            plus_stroke,
                        );
                        if plus_resp.clicked() {
                            self.lightbox_zoom = (self.lightbox_zoom * 1.2).clamp(0.2, 10.0);
                            ctx.request_repaint();
                        }

                        ui.add_space(16.0);

                        // Separator
                        let (sep_rect, _) = ui.allocate_exact_size(
                            egui::vec2(1.0, 16.0),
                            egui::Sense {
                                click: false,
                                drag: false,
                                focusable: false,
                            },
                        );
                        ui.painter().vline(
                            sep_rect.center().x,
                            sep_rect.y_range(),
                            egui::Stroke::new(
                                1.0,
                                egui::Color32::from_rgba_unmultiplied(255, 255, 255, 50),
                            ),
                        );

                        ui.add_space(16.0);

                        // Open / Open With Button
                        let open_btn_rect = ui
                            .allocate_exact_size(egui::vec2(75.0, 24.0), egui::Sense::click())
                            .0;
                        let open_resp = ui.interact(
                            open_btn_rect,
                            ui.id().with("open_btn"),
                            egui::Sense::click(),
                        );

                        let open_bg = if open_resp.clicked() {
                            egui::Color32::from_rgba_unmultiplied(255, 255, 255, 60)
                        } else if open_resp.hovered() {
                            egui::Color32::from_rgba_unmultiplied(255, 255, 255, 30)
                        } else {
                            egui::Color32::TRANSPARENT
                        };

                        ui.painter()
                            .rect_filled(open_btn_rect, egui::Rounding::same(6.0), open_bg);

                        let icon_color = if open_resp.hovered() {
                            self.theme_colors
                                .as_ref()
                                .map_or(egui::Color32::WHITE, |t| t.lightbox_icon_hovered)
                        } else {
                            self.theme_colors
                                .as_ref()
                                .map_or(egui::Color32::from_rgb(200, 200, 200), |t| t.lightbox_icon)
                        };

                        // Draw Icon via painter
                        let icon_rect = egui::Rect::from_min_size(
                            egui::pos2(open_btn_rect.left() + 12.5, open_btn_rect.center().y - 7.0),
                            egui::vec2(14.0, 14.0),
                        );
                        theme::paint_open_icon(ui, icon_rect, icon_color);

                        // Draw Text via painter
                        let text_pos =
                            egui::pos2(open_btn_rect.left() + 32.5, open_btn_rect.center().y);
                        ui.painter().text(
                            text_pos,
                            egui::Align2::LEFT_CENTER,
                            "Open",
                            egui::FontId::proportional(13.0),
                            icon_color,
                        );

                        if open_resp.clicked() {
                            let _ = crate::opener::open_item(&crate::opener::OpenTarget::Image(
                                filename.clone(),
                            ));
                        }
                    });
                });
            });

        if close_preview {
            self.preview_image = None;
        }
    }
}

impl eframe::App for PopupApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let mut reveal_after_render = false;

        // ── Daemon-mode preamble ──
        if self.daemon_mode {
            // Window-manager close shortcuts (for example i3 mod+shift+q) must
            // hide the resident popup, not terminate its event loop.
            if ctx.input(|i| i.viewport().close_requested()) {
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                self.hide_daemon_popup(ctx);
                return;
            }

            // 1. Check shutdown signal
            let should_shutdown = self
                .shutdown_rx
                .as_mut()
                .and_then(|rx| rx.try_recv().ok())
                .is_some();
            if should_shutdown {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                return;
            }

            // 2. Drain history updates (new clips discovered while hidden)
            {
                let mut new_items: Vec<ClipItem> = Vec::new();
                if let Some(rx) = self.history_update_rx.as_mut() {
                    while let Ok(clips) = rx.try_recv() {
                        new_items.extend(clips);
                    }
                }
                if !new_items.is_empty() {
                    let start_idx = self.clips.len();
                    self.clips.extend(new_items);
                    for item in &self.clips[start_idx..] {
                        match item {
                            ClipItem::Text { content, .. } => {
                                self.cached_clip_char_counts.push(content.chars().count());
                                self.cached_clip_previews
                                    .push(preview_text(content, self.preview_chars));
                                self.cached_clip_search.push(content.to_lowercase());
                            }
                            ClipItem::Image { filename, .. } => {
                                self.cached_clip_char_counts.push(0);
                                self.cached_clip_previews.push(String::new());
                                self.cached_clip_search.push(filename.to_lowercase());
                            }
                        }
                    }
                    self.apply_filter();
                    ctx.request_repaint();
                }
            }

            // 3. Check show signal
            let show_requested = self
                .show_rx
                .as_mut()
                .and_then(|rx| rx.try_recv().ok())
                .is_some();
            if show_requested {
                self.visible.store(true, Ordering::Relaxed);
                self.focused_once = false;
                self.focus_search_once = true;
                self.hide_command_sent = false;
                reveal_after_render = true;
                ctx.request_repaint();
            }

            // 4. If hidden and no show signal, skip rendering entirely
            if !self.visible.load(Ordering::Relaxed) {
                if !self.hide_command_sent {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
                    self.hide_command_sent = true;
                }

                // Still drain thumbnail/channel caches even when hidden
                // (handled below by the unconditional drain loops)
                // But skip the rest of update if nothing to show
                let mut loaded_any = false;
                while let Ok((filename, ci)) = self.rx.try_recv() {
                    let tex = ctx.load_texture(&filename, ci, egui::TextureOptions::LINEAR);
                    self.textures.insert(filename, tex);
                    loaded_any = true;
                }
                // Still drain clip_rx and app_rx to populate data for when we show
                if !self.clips_loaded {
                    if let Ok(data) = self.clip_rx.try_recv() {
                        self.clips = data.clips;
                        self.cached_clip_char_counts = data.cached_clip_char_counts;
                        self.cached_clip_previews = data.cached_clip_previews;
                        self.cached_clip_search = data.cached_clip_search;
                        self.cached_clip_file_sizes = data.cached_clip_file_sizes;
                        self.clips_loaded = true;
                        self.filtered = (0..self.clips.len())
                            .map(|clip_idx| DisplayItem::Clip { clip_idx })
                            .collect();
                        self.apply_filter();
                    }
                }
                if !self.apps_loaded {
                    if let Ok(apps) = self.app_rx.try_recv() {
                        self.apps = apps;
                        self.cached_app_search = self
                            .apps
                            .iter()
                            .map(|app| {
                                format!(
                                    "{} {} {}",
                                    app.name.to_lowercase(),
                                    app.comment.to_lowercase(),
                                    app.exec.to_lowercase()
                                )
                            })
                            .collect();
                        self.apps_loaded = true;
                        self.apply_filter();
                    }
                }
                if loaded_any {
                    ctx.request_repaint();
                }
                // Keep the event loop alive while hidden so we can detect
                // show signals on show_rx. Poll every 50ms.
                ctx.request_repaint_after(std::time::Duration::from_millis(50));
                return; // skip rendering while hidden
            }
        }

        let mut loaded_any = false;
        while let Ok((filename, ci)) = self.rx.try_recv() {
            let tex = ctx.load_texture(&filename, ci, egui::TextureOptions::LINEAR);
            self.textures.insert(filename, tex);
            loaded_any = true;
        }
        if loaded_any {
            ctx.request_repaint();
        }

        // ── Async clip loading: receive history+caches once loaded ──
        if !self.clips_loaded {
            if let Ok(data) = self.clip_rx.try_recv() {
                self.clips = data.clips;
                self.cached_clip_char_counts = data.cached_clip_char_counts;
                self.cached_clip_previews = data.cached_clip_previews;
                self.cached_clip_search = data.cached_clip_search;
                self.cached_clip_file_sizes = data.cached_clip_file_sizes;
                self.clips_loaded = true;
                // Build initial filtered list now that we have clips
                self.filtered = (0..self.clips.len())
                    .map(|clip_idx| DisplayItem::Clip { clip_idx })
                    .collect();
                self.apply_filter();
                ctx.request_repaint();
            }
        }

        // ── Async app loading: receive apps once they're done scanning ──
        if !self.apps_loaded {
            if let Ok(apps) = self.app_rx.try_recv() {
                self.apps = apps;
                self.cached_app_search = self
                    .apps
                    .iter()
                    .map(|app| {
                        format!(
                            "{} {} {}",
                            app.name.to_lowercase(),
                            app.comment.to_lowercase(),
                            app.exec.to_lowercase()
                        )
                    })
                    .collect();
                self.apps_loaded = true;
                self.apply_filter();
                ctx.request_repaint();
            }
        }

        if self.config.general.close_on_focus_out {
            let focused = ctx.input(|i| i.focused);
            if focused {
                self.focused_once = true;
            }
            if self.focused_once && !focused {
                if self.daemon_mode {
                    self.hide_daemon_popup(ctx);
                } else {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
                return;
            }
        }

        let close = ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape));
        let down = ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown));
        let up = ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp));
        let enter = ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Enter));
        let del = ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Delete));
        let ctrl_k = ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::K));
        let ctrl_o = ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::O));

        let mut select_digit = None;
        ctx.input_mut(|i| {
            let digits = [
                egui::Key::Num1,
                egui::Key::Num2,
                egui::Key::Num3,
                egui::Key::Num4,
                egui::Key::Num5,
                egui::Key::Num6,
                egui::Key::Num7,
                egui::Key::Num8,
                egui::Key::Num9,
            ];
            for (index, &key) in digits.iter().enumerate() {
                if i.consume_key(egui::Modifiers::CTRL, key) {
                    select_digit = Some(index);
                    break;
                }
            }
        });

        if close {
            if self.preview_image.is_some() {
                self.preview_image = None;
            } else if self.daemon_mode {
                self.hide_daemon_popup(ctx);
            } else {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            return;
        }
        if down {
            self.move_selection_down();
        }
        if up {
            self.move_selection_up();
        }
        if enter {
            self.select_and_close(ctx);
            return;
        }
        if del {
            self.delete_current();
        }
        if ctrl_k {
            self.query.clear();
            self.apply_filter();
        }
        if ctrl_o {
            if let Some((ref filename, _)) = self.preview_image {
                let _ =
                    crate::opener::open_item(&crate::opener::OpenTarget::Image(filename.clone()));
            } else if let Some(item) = self.selected_clip() {
                let target = match item {
                    ClipItem::Text { content, .. } => {
                        Some(crate::opener::OpenTarget::Text(content.clone()))
                    }
                    ClipItem::Image { filename, .. } => {
                        Some(crate::opener::OpenTarget::Image(filename.clone()))
                    }
                };
                if let Some(t) = target {
                    let _ = crate::opener::open_item(&t);
                }
            } else if let Some(DisplayItem::App { app_idx }) = self.filtered.get(self.selected) {
                if let Some(app) = self.apps.get(*app_idx) {
                    let exec = app.exec.clone();
                    let _ = std::process::Command::new("sh")
                        .arg("-c")
                        .arg(&exec)
                        .spawn();
                }
            }
        }
        if let Some(digit) = select_digit {
            if digit < self.filtered.len() {
                self.selected = digit;
                self.select_and_close(ctx);
                return;
            }
        }

        if self.scroll_to_selected_once {
            ctx.request_repaint();
        }

        let panel_rounding = self.theme_colors.as_ref().map_or(0.0, |t| t.card_rounding);
        egui::CentralPanel::default()
            .frame(
                egui::Frame::none()
                    .fill(ctx.style().visuals.window_fill)
                    .rounding(panel_rounding)
                    .stroke(egui::Stroke::new(
                        1.0,
                        ctx.style().visuals.widgets.noninteractive.bg_stroke.color,
                    )),
            )
            .show(ctx, |ui| {
                if self.config.footer.enable {
                    egui::TopBottomPanel::bottom("footer_panel")
                        .frame(egui::Frame::none())
                        .show_inside(ui, |ui| {
                            self.draw_footer(ui);
                        });
                }
                ui.vertical(|ui| {
                    self.draw_header(ui);
                    self.draw_search(ui);
                    self.draw_body(ui);
                });
                self.draw_lightbox(ui);
            });

        if reveal_after_render {
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
        }
    }
}

fn draw_empty_state(ui: &mut egui::Ui, title: &str, subtitle: &str, weak_color: egui::Color32) {
    ui.vertical_centered(|ui| {
        ui.add_space(100.0);
        let (icon_rect, _) = ui.allocate_exact_size(egui::vec2(44.0, 44.0), egui::Sense::hover());
        theme::paint_text_icon(ui, icon_rect, weak_color);

        ui.add_space(16.0);
        ui.label(egui::RichText::new(title).heading().strong());
        ui.add_space(8.0);
        ui.label(egui::RichText::new(subtitle).size(15.0).color(weak_color));
    });
}

#[cfg(test)]
fn item_matches_query(item: &ClipItem, q: &str) -> bool {
    if q.is_empty() {
        return true;
    }

    match item {
        ClipItem::Text { content, .. } => content.to_lowercase().contains(q),
        ClipItem::Image {
            width,
            height,
            filename,
            ..
        } => format!("{}×{} {}x{} {}", width, height, width, height, filename)
            .to_lowercase()
            .contains(q),
    }
}

fn preview_text(text: &str, max_chars: usize) -> String {
    let normalized = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut preview = normalized.chars().take(max_chars).collect::<String>();
    if normalized.chars().count() > max_chars {
        preview.push('…');
    }
    preview
}

#[cfg(test)]
#[allow(dead_code)]
fn image_subtitle(filename: &str, ts: u64) -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    image_subtitle_with_now(filename, ts, now)
}

/// Like `image_subtitle` but uses a pre-cached file size instead of
/// calling `std::fs::metadata` on every frame.
fn image_subtitle_cached(filename: &str, ts: u64, cached_size: Option<u64>) -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    if filename.is_empty() {
        relative_time_with_now(ts, now)
    } else {
        let size_str = cached_size.map(format_size).unwrap_or_default();
        if size_str.is_empty() {
            format!("{} · {}", filename, relative_time_with_now(ts, now))
        } else {
            format!(
                "{} · {} · {}",
                filename,
                size_str,
                relative_time_with_now(ts, now)
            )
        }
    }
}

fn format_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{} KB", bytes / 1024)
    } else {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}

#[cfg(test)]
fn image_subtitle_with_now(filename: &str, ts: u64, now: u64) -> String {
    if filename.is_empty() {
        relative_time_with_now(ts, now)
    } else {
        let size_str = if let Ok(meta) = std::fs::metadata(Config::images_dir().join(filename)) {
            let bytes = meta.len();
            format_size(bytes)
        } else {
            "".to_string()
        };

        if size_str.is_empty() {
            format!("{} · {}", filename, relative_time_with_now(ts, now))
        } else {
            format!(
                "{} · {} · {}",
                filename,
                size_str,
                relative_time_with_now(ts, now)
            )
        }
    }
}

fn relative_time(ts: u64) -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    relative_time_with_now(ts, now)
}

fn relative_time_with_now(ts: u64, now: u64) -> String {
    let diff = now.saturating_sub(ts);
    if diff < 60 {
        format!("{}s ago", diff)
    } else if diff < 3_600 {
        format!("{}m ago", diff / 60)
    } else if diff < 86_400 {
        format!("{}h ago", diff / 3_600)
    } else {
        format!("{}d ago", diff / 86_400)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preview_collapses_whitespace_and_truncates() {
        assert_eq!(preview_text("hello\n   world", 50), "hello world");
        assert_eq!(preview_text("abcdef", 3), "abc…");
    }

    #[test]
    fn relative_time_formats_expected_units() {
        assert_eq!(relative_time_with_now(95, 100), "5s ago");
        assert_eq!(relative_time_with_now(0, 120), "2m ago");
        assert_eq!(relative_time_with_now(0, 7_200), "2h ago");
        assert_eq!(relative_time_with_now(0, 172_800), "2d ago");
    }

    #[test]
    fn item_query_matches_text_and_image_metadata() {
        let text = ClipItem::Text {
            content: "Hello world".into(),
            timestamp: 1,
        };
        let img = ClipItem::Image {
            width: 640,
            height: 480,
            timestamp: 2,
            filename: "shot.png".into(),
            data: None,
        };
        assert!(item_matches_query(&text, "hello"));
        assert!(item_matches_query(&img, "640x480"));
        assert!(item_matches_query(&img, "shot"));
        assert!(!item_matches_query(&text, "missing"));
    }

    #[test]
    fn image_subtitle_handles_empty_filename() {
        assert_eq!(image_subtitle_with_now("", 90, 100), "10s ago");
        assert_eq!(
            image_subtitle_with_now("shot.png", 90, 100),
            "shot.png · 10s ago"
        );
    }

    #[test]
    fn format_size_works_for_all_ranges() {
        assert_eq!(format_size(500), "500 B");
        assert_eq!(format_size(1024), "1 KB");
        assert_eq!(format_size(2048), "2 KB");
        assert_eq!(format_size(150000), "146 KB");
        assert_eq!(format_size(1024 * 1024), "1.0 MB");
        assert_eq!(format_size(2500000), "2.4 MB");
    }
}
