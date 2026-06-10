use crate::config::Config;
use crate::history::ClipItem;
use crate::storage;
use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

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
            .with_resizable(false),
        ..Default::default()
    };

    let should_paste_after_window = should_paste.clone();
    let auto_paste = config.general.auto_paste;
    let paste_delay_ms = config.general.paste_delay_ms;

    let _ = eframe::run_native(
        "clipit-rs",
        options,
        Box::new(move |cc| {
            match config.general.theme.as_str() {
                "light" => cc.egui_ctx.set_visuals(egui::Visuals::light()),
                "system" => {}
                _ => cc.egui_ctx.set_visuals(egui::Visuals::dark()),
            }

            let mut style = (*cc.egui_ctx.style()).clone();
            style.spacing.item_spacing.y = 6.0;
            style.spacing.button_padding = egui::vec2(8.0, 6.0);
            cc.egui_ctx.set_style(style);

            Ok(Box::new(PopupApp::new(cc, config, should_paste_after_window)))
        }),
    );

    if auto_paste && should_paste.load(Ordering::Relaxed) {
        std::thread::sleep(std::time::Duration::from_millis(paste_delay_ms));
        simulate_paste();
    }
}

fn simulate_paste() {
    // xdotool is the most reliable simple option on X11. Wayland users can
    // disable auto_paste or provide their own compositor-level paste binding.
    let _ = std::process::Command::new("xdotool")
        .args(["key", "ctrl+v"])
        .status();
}

struct PopupApp {
    all: Vec<ClipItem>,
    filtered: Vec<usize>,
    query: String,
    selected: usize,
    textures: HashMap<String, egui::TextureHandle>,
    textures_loaded: bool,
    should_paste: Arc<AtomicBool>,
    preview_chars: usize,
}

impl PopupApp {
    fn new(
        _cc: &eframe::CreationContext<'_>,
        config: Config,
        should_paste: Arc<AtomicBool>,
    ) -> Self {
        let all: Vec<ClipItem> = storage::load_history().into_iter().collect();
        let filtered: Vec<usize> = (0..all.len()).collect();
        Self {
            all,
            filtered,
            query: String::new(),
            selected: 0,
            textures: HashMap::new(),
            textures_loaded: false,
            should_paste,
            preview_chars: config.general.preview_chars,
        }
    }

    fn load_textures(&mut self, ctx: &egui::Context) {
        if self.textures_loaded {
            return;
        }

        for item in &self.all {
            if let ClipItem::Image { filename, .. } = item {
                if filename.is_empty() || self.textures.contains_key(filename) {
                    continue;
                }

                if let Ok(img) = image::open(Config::images_dir().join(filename)) {
                    let thumb = img.resize(40, 40, image::imageops::FilterType::Triangle);
                    let rgba = thumb.to_rgba8();
                    let size = [rgba.width() as usize, rgba.height() as usize];
                    let ci = egui::ColorImage::from_rgba_unmultiplied(size, rgba.as_raw());
                    let tex = ctx.load_texture(filename, ci, egui::TextureOptions::LINEAR);
                    self.textures.insert(filename.clone(), tex);
                }
            }
        }

        self.textures_loaded = true;
    }

    fn apply_filter(&mut self) {
        let q = self.query.trim().to_lowercase();
        self.filtered = self
            .all
            .iter()
            .enumerate()
            .filter(|(_, item)| item_matches_query(item, &q))
            .map(|(i, _)| i)
            .collect();
        self.selected = self.selected.min(self.filtered.len().saturating_sub(1));
    }

    fn select_and_close(&self, ctx: &egui::Context) {
        if let Some(item) = self.selected_item() {
            if let Ok(mut cb) = arboard::Clipboard::new() {
                match item {
                    ClipItem::Text { content, .. } => {
                        let _ = cb.set_text(content.clone());
                    }
                    ClipItem::Image { filename, .. } => {
                        if let Ok((w, h, data)) = storage::load_image(filename) {
                            let img_data = arboard::ImageData {
                                width: w as usize,
                                height: h as usize,
                                bytes: std::borrow::Cow::Owned(data),
                            };
                            let _ = cb.set_image(img_data);
                        }
                    }
                }
            }
            self.should_paste.store(true, Ordering::Relaxed);
        }
        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
    }

    fn selected_item(&self) -> Option<&ClipItem> {
        self.filtered
            .get(self.selected)
            .and_then(|idx| self.all.get(*idx))
    }

    fn delete_current(&mut self) {
        let Some(&orig_idx) = self.filtered.get(self.selected) else { return };

        if let Some(ClipItem::Image { filename, .. }) = self.all.get(orig_idx) {
            if !filename.is_empty() {
                storage::delete_image_file(filename);
            }
        }

        self.all.remove(orig_idx);
        self.persist_all();
        self.apply_filter();
        self.textures_loaded = false;
    }

    fn clear_history(&mut self) {
        for item in &self.all {
            if let ClipItem::Image { filename, .. } = item {
                if !filename.is_empty() {
                    storage::delete_image_file(filename);
                }
            }
        }
        self.all.clear();
        self.query.clear();
        self.persist_all();
        self.apply_filter();
        self.textures.clear();
        self.textures_loaded = true;
    }

    fn persist_all(&self) {
        let items: VecDeque<ClipItem> = self.all.clone().into_iter().collect();
        let _ = storage::save_history(&items);
        storage::cleanup_orphaned(&items);
    }

    fn draw_row(&mut self, ui: &mut egui::Ui, row: usize, item: ClipItem) -> bool {
        let is_selected = row == self.selected;
        let row_height = if matches!(&item, ClipItem::Image { .. }) {
            58.0
        } else {
            52.0
        };

        let (rect, response) = ui.allocate_exact_size(
            egui::vec2(ui.available_width(), row_height),
            egui::Sense::click(),
        );

        let fill = if is_selected {
            ui.visuals().selection.bg_fill
        } else if response.hovered() {
            ui.visuals().widgets.hovered.bg_fill
        } else {
            ui.visuals().faint_bg_color
        };

        ui.painter().rect_filled(rect, egui::Rounding::same(8.0), fill);

        ui.allocate_ui_at_rect(rect.shrink(8.0), |ui| {
            ui.horizontal_centered(|ui| match &item {
                ClipItem::Text { content, timestamp } => {
                    ui.label(egui::RichText::new("TXT").monospace().small().strong());
                    ui.add_space(4.0);
                    ui.vertical(|ui| {
                        ui.label(
                            egui::RichText::new(preview_text(content, self.preview_chars))
                                .monospace()
                                .strong(),
                        );
                        ui.label(
                            egui::RichText::new(format!(
                                "{} chars · {}",
                                content.chars().count(),
                                relative_time(*timestamp)
                            ))
                            .small()
                            .weak(),
                        );
                    });
                }
                ClipItem::Image {
                    width,
                    height,
                    timestamp,
                    filename,
                    ..
                } => {
                    if let Some(tex) = self.textures.get(filename) {
                        ui.image(tex);
                    } else {
                        ui.label(egui::RichText::new("IMG").monospace().small().strong());
                    }
                    ui.add_space(8.0);
                    ui.vertical(|ui| {
                        ui.label(egui::RichText::new(format!("Image {}×{}", width, height)).strong());
                        ui.label(
                            egui::RichText::new(format!("{} · {}", filename, relative_time(*timestamp)))
                                .small()
                                .weak(),
                        );
                    });
                }
            });
        });

        response.clicked()
    }
}

impl eframe::App for PopupApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.load_textures(ctx);

        let close = ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape));
        let down = ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown));
        let up = ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp));
        let enter = ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Enter));
        let del = ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Delete));
        let ctrl_k = ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::K));

        if close {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        }
        if down && !self.filtered.is_empty() {
            self.selected = (self.selected + 1).min(self.filtered.len() - 1);
        }
        if up {
            self.selected = self.selected.saturating_sub(1);
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

        egui::TopBottomPanel::top("header").show(ctx, |ui| {
            egui::Frame::none()
                .inner_margin(egui::Margin::symmetric(12.0, 10.0))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.label(egui::RichText::new("Clipit").heading().strong());
                            ui.label(
                                egui::RichText::new("Clipboard history")
                                    .small()
                                    .weak(),
                            );
                        });

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("×").clicked() {
                                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                            }
                            ui.label(
                                egui::RichText::new(format!(
                                    "{} / {}",
                                    self.filtered.len(),
                                    self.all.len()
                                ))
                                .small()
                                .weak(),
                            );
                        });
                    });
                });
        });

        egui::TopBottomPanel::top("search").show(ctx, |ui| {
            egui::Frame::none()
                .inner_margin(egui::Margin::symmetric(12.0, 8.0))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("🔎");
                        let resp = ui.add(
                            egui::TextEdit::singleline(&mut self.query)
                                .desired_width(f32::INFINITY)
                                .hint_text("Search text, image size, or filename..."),
                        );
                        if resp.changed() {
                            self.apply_filter();
                            self.selected = 0;
                        }
                        resp.request_focus();

                        if !self.query.is_empty() && ui.button("Clear").clicked() {
                            self.query.clear();
                            self.apply_filter();
                        }
                    });
                });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            if self.all.is_empty() {
                ui.vertical_centered(|ui| {
                    ui.add_space(96.0);
                    ui.label(egui::RichText::new("No clips yet").heading().weak());
                    ui.label("Copy text or an image while the daemon is running.");
                });
                return;
            }

            if self.filtered.is_empty() {
                ui.vertical_centered(|ui| {
                    ui.add_space(96.0);
                    ui.label(egui::RichText::new("No matches").heading().weak());
                    ui.label("Try a shorter search term.");
                });
                return;
            }

            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    let visible: Vec<usize> = self.filtered.clone();
                    for (row, orig_idx) in visible.into_iter().enumerate() {
                        let Some(item) = self.all.get(orig_idx).cloned() else { continue };
                        if self.draw_row(ui, row, item) {
                            self.selected = row;
                            self.select_and_close(ctx);
                            return;
                        }
                        ui.add_space(6.0);
                    }
                });
        });

        egui::TopBottomPanel::bottom("footer").show(ctx, |ui| {
            egui::Frame::none()
                .inner_margin(egui::Margin::symmetric(12.0, 8.0))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("Esc close · ↑↓ navigate · Enter paste · Del remove · Ctrl+K clear search")
                                .small()
                                .weak(),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if !self.all.is_empty() && ui.button("Clear history").clicked() {
                                self.clear_history();
                            }
                        });
                    });
                });
        });
    }
}

fn item_matches_query(item: &ClipItem, q: &str) -> bool {
    if q.is_empty() {
        return true;
    }

    match item {
        ClipItem::Text { content, .. } => content.to_lowercase().contains(q),
        ClipItem::Image { width, height, filename, .. } => {
            format!("{}×{} {}x{} {}", width, height, width, height, filename)
                .to_lowercase()
                .contains(q)
        }
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
        let text = ClipItem::Text { content: "Hello world".into(), timestamp: 1 };
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
}
