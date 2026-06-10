use crate::config::Config;
use crate::history::ClipItem;
use crate::storage;
use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

const SEARCH_HINT: &str = "Search clips, image size, or filename…";
const FOOTER_HELP: &str = "Esc close · ↑↓ navigate · Enter paste · Del remove · Ctrl+K clear search";

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

    let selected_item = Arc::new(std::sync::Mutex::new(None));
    let selected_item_for_app = selected_item.clone();

    let _ = eframe::run_native(
        "clipit-rs",
        options,
        Box::new(move |cc| {
            apply_theme_and_fonts(&cc.egui_ctx, config.general.theme.as_str());
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
        if is_daemon_running() {
            let pending_file = Config::data_dir().join("pending_paste.json");
            if let Ok(json) = serde_json::to_string(&item) {
                let _ = std::fs::write(pending_file, json);
            }
        } else {
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

fn simulate_paste() {
    // xdotool is the most reliable simple option on X11. Wayland users can
    // disable auto_paste or provide their own compositor-level paste binding.
    let _ = std::process::Command::new("xdotool")
        .args(["key", "ctrl+v"])
        .status();
}

fn apply_theme_and_fonts(ctx: &egui::Context, theme: &str) {
    let mut visuals = if theme == "light" {
        egui::Visuals::light()
    } else {
        egui::Visuals::dark()
    };

    if theme == "light" {
        visuals.window_fill = egui::Color32::from_rgb(248, 250, 252); // Slate 50
        visuals.panel_fill = egui::Color32::from_rgb(248, 250, 252);
        visuals.extreme_bg_color = egui::Color32::from_rgb(241, 245, 249); // Slate 100
        visuals.widgets.noninteractive.bg_fill = egui::Color32::from_rgb(241, 245, 249);
        visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(226, 232, 240)); // Slate 200
        visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(241, 245, 249);
        visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(226, 232, 240);
        visuals.widgets.active.bg_fill = egui::Color32::from_rgb(203, 213, 225);
        visuals.selection.bg_fill = egui::Color32::from_rgb(79, 70, 229); // Indigo 600
        visuals.selection.stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(99, 102, 241));
    } else {
        visuals.window_fill = egui::Color32::from_rgb(11, 15, 25); // Slate 950
        visuals.panel_fill = egui::Color32::from_rgb(11, 15, 25);
        visuals.extreme_bg_color = egui::Color32::from_rgb(20, 26, 38); // Slate 900
        visuals.widgets.noninteractive.bg_fill = egui::Color32::from_rgb(20, 26, 38);
        visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(33, 41, 54)); // Slate 800
        visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(20, 26, 38);
        visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(28, 35, 51);
        visuals.widgets.active.bg_fill = egui::Color32::from_rgb(51, 65, 85);
        visuals.selection.bg_fill = egui::Color32::from_rgb(79, 70, 229); // Indigo 600
        visuals.selection.stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(129, 140, 248)); // Indigo 400
    }

    visuals.window_rounding = egui::Rounding::same(16.0);
    ctx.set_visuals(visuals);

    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = egui::vec2(8.0, 8.0);
    style.spacing.button_padding = egui::vec2(12.0, 8.0);
    style.spacing.window_margin = egui::Margin::same(0.0);
    style.visuals.window_rounding = egui::Rounding::same(16.0);

    style.text_styles.insert(egui::TextStyle::Heading, egui::FontId::proportional(22.0));
    style.text_styles.insert(egui::TextStyle::Body, egui::FontId::proportional(16.0));
    style.text_styles.insert(egui::TextStyle::Button, egui::FontId::proportional(14.5));
    style.text_styles.insert(egui::TextStyle::Small, egui::FontId::proportional(13.0));
    style.text_styles.insert(egui::TextStyle::Monospace, egui::FontId::monospace(15.0));

    ctx.set_style(style);
}

// ── custom vector icons drawn programmatically ───────────────────────

fn paint_search_icon(ui: &mut egui::Ui, rect: egui::Rect, color: egui::Color32) {
    let painter = ui.painter();
    let stroke = egui::Stroke::new(1.8, color);

    // Search circle
    let radius = (rect.width() * 0.32).min(rect.height() * 0.32);
    let center = rect.center() - egui::vec2(1.5, 1.5);
    painter.circle_stroke(center, radius, stroke);

    // Handle line
    let start = center + egui::vec2(radius * 0.707, radius * 0.707);
    let end = rect.right_bottom() - egui::vec2(1.5, 1.5);
    painter.line_segment([start, end], stroke);
}

fn paint_close_icon(ui: &mut egui::Ui, rect: egui::Rect, color: egui::Color32) {
    let painter = ui.painter();
    let stroke = egui::Stroke::new(1.8, color);
    painter.line_segment([rect.left_top(), rect.right_bottom()], stroke);
    painter.line_segment([rect.right_top(), rect.left_bottom()], stroke);
}

fn paint_text_icon(ui: &mut egui::Ui, rect: egui::Rect, color: egui::Color32) {
    let painter = ui.painter();
    let stroke = egui::Stroke::new(1.5, color);

    // Document boundary
    painter.rect_stroke(rect, egui::Rounding::same(1.5), stroke);

    // Document text lines
    let line_y1 = rect.top() + rect.height() * 0.3;
    let line_y2 = rect.top() + rect.height() * 0.55;
    let line_y3 = rect.top() + rect.height() * 0.8;

    painter.line_segment([egui::pos2(rect.left() + 3.0, line_y1), egui::pos2(rect.right() - 3.0, line_y1)], stroke);
    painter.line_segment([egui::pos2(rect.left() + 3.0, line_y2), egui::pos2(rect.right() - 3.0, line_y2)], stroke);
    painter.line_segment([egui::pos2(rect.left() + 3.0, line_y3), egui::pos2(rect.right() - 6.0, line_y3)], stroke);
}

fn paint_image_icon(ui: &mut egui::Ui, rect: egui::Rect, color: egui::Color32) {
    let painter = ui.painter();
    let stroke = egui::Stroke::new(1.5, color);

    // Image border frame
    painter.rect_stroke(rect, egui::Rounding::same(1.5), stroke);

    // Sun
    let sun_center = rect.left_top() + egui::vec2(rect.width() * 0.3, rect.height() * 0.3);
    painter.circle_stroke(sun_center, rect.width() * 0.1, egui::Stroke::new(1.2, color));

    // Mountains
    let p1 = egui::pos2(rect.left() + 2.0, rect.bottom() - 2.0);
    let p2 = egui::pos2(rect.left() + rect.width() * 0.4, rect.top() + rect.height() * 0.45);
    let p3 = egui::pos2(rect.left() + rect.width() * 0.6, rect.bottom() - 4.0);
    let p4 = egui::pos2(rect.left() + rect.width() * 0.8, rect.top() + rect.height() * 0.55);
    let p5 = egui::pos2(rect.right() - 2.0, rect.bottom() - 2.0);

    painter.line_segment([p1, p2], stroke);
    painter.line_segment([p2, p3], stroke);
    painter.line_segment([p3, p4], stroke);
    painter.line_segment([p4, p5], stroke);
}

fn paint_trash_icon(ui: &mut egui::Ui, rect: egui::Rect, color: egui::Color32) {
    let painter = ui.painter();
    let stroke = egui::Stroke::new(1.5, color);

    // Lid line
    painter.line_segment(
        [egui::pos2(rect.left() - 1.0, rect.top() + rect.height() * 0.2),
         egui::pos2(rect.right() + 1.0, rect.top() + rect.height() * 0.2)],
        stroke
    );

    // Lid handle on top
    let handle_w = rect.width() * 0.3;
    let handle_h = rect.height() * 0.15;
    let handle_rect = egui::Rect::from_center_size(
        egui::pos2(rect.center().x, rect.top() + handle_h / 2.0),
        egui::vec2(handle_w, handle_h)
    );
    painter.rect_stroke(handle_rect, egui::Rounding::same(0.5), stroke);

    // Trash body
    let bin_rect = egui::Rect::from_min_max(
        egui::pos2(rect.left() + 2.0, rect.top() + rect.height() * 0.25),
        egui::pos2(rect.right() - 2.0, rect.bottom())
    );
    painter.rect_stroke(bin_rect, egui::Rounding::same(1.0), stroke);

    // Ribs
    painter.line_segment(
        [egui::pos2(rect.center().x - 1.5, rect.top() + rect.height() * 0.4),
         egui::pos2(rect.center().x - 1.5, rect.bottom() - 2.0)],
        stroke
    );
    painter.line_segment(
        [egui::pos2(rect.center().x + 1.5, rect.top() + rect.height() * 0.4),
         egui::pos2(rect.center().x + 1.5, rect.bottom() - 2.0)],
        stroke
    );
}

fn draw_icon_badge(ui: &mut egui::Ui, icon_type: &str, is_selected: bool) {
    let size = egui::vec2(36.0, 36.0);
    let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());

    let bg_color = if is_selected {
        if ui.visuals().dark_mode {
            egui::Color32::from_rgba_unmultiplied(255, 255, 255, 25)
        } else {
            egui::Color32::from_rgba_unmultiplied(0, 0, 0, 15)
        }
    } else {
        if ui.visuals().dark_mode {
            egui::Color32::from_rgb(20, 26, 38) // Slate 900
        } else {
            egui::Color32::from_rgb(241, 245, 249) // Slate 100
        }
    };

    let icon_color = if is_selected {
        egui::Color32::WHITE
    } else {
        egui::Color32::from_rgb(99, 102, 241) // Indigo 500
    };

    ui.painter().circle_filled(rect.center(), 18.0, bg_color);

    // Draw the actual icon centered inside the badge (16x16 size)
    let icon_rect = egui::Rect::from_center_size(rect.center(), egui::vec2(16.0, 16.0));
    match icon_type {
        "text" => paint_text_icon(ui, icon_rect, icon_color),
        "image" => paint_image_icon(ui, icon_rect, icon_color),
        _ => {}
    }
}

struct PopupApp {
    all: Vec<ClipItem>,
    filtered: Vec<usize>,
    query: String,
    selected: usize,
    textures: HashMap<String, egui::TextureHandle>,
    textures_loaded: bool,
    should_paste: Arc<AtomicBool>,
    selected_item_out: Arc<std::sync::Mutex<Option<ClipItem>>>,
    preview_chars: usize,
    focus_search_once: bool,
    scroll_to_selected_once: bool,
}

impl PopupApp {
    fn new(
        _cc: &eframe::CreationContext<'_>,
        config: Config,
        should_paste: Arc<AtomicBool>,
        selected_item_out: Arc<std::sync::Mutex<Option<ClipItem>>>,
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
            selected_item_out,
            preview_chars: config.general.preview_chars,
            focus_search_once: true,
            scroll_to_selected_once: false,
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
                    let thumb = img.resize(52, 52, image::imageops::FilterType::Triangle);
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
        self.scroll_to_selected_once = true;
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

    fn select_and_close(&self, ctx: &egui::Context) {
        if let Some(item) = self.selected_item() {
            if let Ok(mut out) = self.selected_item_out.lock() {
                *out = Some(item.clone());
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

    fn draw_header(&mut self, ui: &mut egui::Ui) {
        egui::Frame::none()
            .inner_margin(egui::Margin::symmetric(20.0, 16.0))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(egui::RichText::new("Clipit").heading().strong());
                        ui.add_space(2.0);
                        ui.label(
                            egui::RichText::new("Clipboard history")
                                .size(13.0)
                                .weak(),
                        );
                    });

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // Round hoverable close button
                        let button_size = egui::vec2(28.0, 28.0);
                        let (rect, resp) = ui.allocate_exact_size(button_size, egui::Sense::click());
                        let bg_fill = if resp.clicked() {
                            ui.visuals().widgets.active.bg_fill
                        } else if resp.hovered() {
                            ui.visuals().widgets.hovered.bg_fill
                        } else {
                            egui::Color32::TRANSPARENT
                        };
                        ui.painter().circle_filled(rect.center(), 14.0, bg_fill);
                        
                        // Render programmatically drawn close icon inside
                        let close_icon_rect = egui::Rect::from_center_size(rect.center(), egui::vec2(10.0, 10.0));
                        paint_close_icon(ui, close_icon_rect, ui.visuals().text_color());

                        if resp.clicked() {
                            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                        }

                        ui.add_space(12.0);
                        ui.label(
                            egui::RichText::new(format!(
                                "{} shown / {} total",
                                self.filtered.len(),
                                self.all.len()
                            ))
                            .size(13.0)
                            .weak(),
                        );
                    });
                });
            });
    }

    fn draw_search(&mut self, ui: &mut egui::Ui) {
        egui::Frame::none()
            .inner_margin(egui::Margin::symmetric(20.0, 8.0))
            .show(ui, |ui| {
                let bg_fill = ui.visuals().extreme_bg_color;
                let stroke = ui.visuals().widgets.noninteractive.bg_stroke;

                egui::Frame::none()
                    .fill(bg_fill)
                    .stroke(stroke)
                    .rounding(10.0)
                    .inner_margin(egui::Margin::symmetric(12.0, 8.0))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            // Search icon
                            let (icon_rect, _) = ui.allocate_exact_size(egui::vec2(16.0, 16.0), egui::Sense::hover());
                            paint_search_icon(ui, icon_rect, ui.visuals().weak_text_color());

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
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    let (btn_rect, btn_resp) = ui.allocate_exact_size(egui::vec2(18.0, 18.0), egui::Sense::click());
                                    let btn_color = if btn_resp.hovered() {
                                        ui.visuals().text_color()
                                    } else {
                                        ui.visuals().weak_text_color()
                                    };
                                    let close_icon_rect = egui::Rect::from_center_size(btn_rect.center(), egui::vec2(8.0, 8.0));
                                    paint_close_icon(ui, close_icon_rect, btn_color);
                                    if btn_resp.clicked() {
                                        self.query.clear();
                                        self.apply_filter();
                                    }
                                });
                            }
                        });
                    });
            });
    }

    fn draw_body(&mut self, ui: &mut egui::Ui) {
        egui::Frame::none()
            .inner_margin(egui::Margin::symmetric(20.0, 6.0))
            .show(ui, |ui| {
                if self.all.is_empty() {
                    draw_empty_state(
                        ui,
                        "No clips yet",
                        "Copy text or an image while the daemon is running.",
                    );
                    return;
                }

                if self.filtered.is_empty() {
                    draw_empty_state(ui, "No matches", "Try a shorter search term.");
                    return;
                }

                egui::ScrollArea::vertical()
                    .id_source("clipit_clip_list")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.spacing_mut().item_spacing.y = 8.0;
                        ui.set_width(ui.available_width());
                        let visible = self.filtered.clone();
                        for (row, orig_idx) in visible.into_iter().enumerate() {
                            let Some(item) = self.all.get(orig_idx).cloned() else { continue };
                            if self.draw_row(ui, row, item) {
                                self.selected = row;
                                self.select_and_close(ui.ctx());
                                break;
                            }
                        }
                    });
            });
    }

    fn draw_footer(&mut self, ui: &mut egui::Ui) {
        egui::Frame::none()
            .inner_margin(egui::Margin::symmetric(20.0, 12.0))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(FOOTER_HELP).size(12.5).weak());

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let enabled = !self.all.is_empty();

                        let button_rect = ui.allocate_exact_size(egui::vec2(120.0, 32.0), egui::Sense::click()).0;
                        let response = ui.interact(button_rect, ui.id().with("clear_history_btn"), egui::Sense::click());

                        let fill = if !enabled {
                            ui.visuals().widgets.noninteractive.bg_fill
                        } else if response.clicked() {
                            ui.visuals().widgets.active.bg_fill
                        } else if response.hovered() {
                            ui.visuals().widgets.hovered.bg_fill
                        } else {
                            ui.visuals().widgets.inactive.bg_fill
                        };

                        let stroke = if !enabled {
                            ui.visuals().widgets.noninteractive.bg_stroke
                        } else if response.hovered() {
                            ui.visuals().widgets.hovered.bg_stroke
                        } else {
                            ui.visuals().widgets.noninteractive.bg_stroke
                        };

                        ui.painter().rect(button_rect, egui::Rounding::same(8.0), fill, stroke);

                        ui.allocate_ui_at_rect(button_rect, |ui| {
                            ui.horizontal_centered(|ui| {
                                ui.add_space(8.0);
                                let (icon_rect, _) = ui.allocate_exact_size(egui::vec2(14.0, 14.0), egui::Sense::hover());
                                let icon_color = if !enabled {
                                    ui.visuals().weak_text_color()
                                } else {
                                    ui.visuals().text_color()
                                };
                                paint_trash_icon(ui, icon_rect, icon_color);

                                ui.add_space(4.0);

                                let text_color = if !enabled {
                                    ui.visuals().weak_text_color()
                                } else {
                                    ui.visuals().text_color()
                                };
                                ui.label(
                                    egui::RichText::new("Clear History")
                                        .size(13.0)
                                        .color(text_color)
                                );
                            });
                        });

                        if enabled && response.clicked() {
                            self.clear_history();
                        }
                    });
                });
            });
    }

    fn draw_row(&mut self, ui: &mut egui::Ui, row: usize, item: ClipItem) -> bool {
        let is_selected = row == self.selected;
        let row_height = match &item {
            ClipItem::Image { .. } => 84.0,
            ClipItem::Text { .. } => 76.0,
        };

        let card_id = ui.make_persistent_id(format!("row_{}", row));
        let rect = egui::Rect::from_min_size(
            ui.next_widget_position(),
            egui::vec2(ui.available_width(), row_height)
        );
        let response = ui.interact(rect, card_id, egui::Sense::click());

        if is_selected && self.scroll_to_selected_once {
            ui.scroll_to_rect(rect, Some(egui::Align::Center));
            self.scroll_to_selected_once = false;
        }

        let fill = if is_selected {
            if ui.visuals().dark_mode {
                egui::Color32::from_rgb(30, 27, 75) // Indigo 950
            } else {
                egui::Color32::from_rgb(224, 231, 255) // Indigo 100
            }
        } else if response.hovered() {
            if ui.visuals().dark_mode {
                egui::Color32::from_rgb(20, 26, 38) // Slate 900
            } else {
                egui::Color32::from_rgb(241, 245, 249) // Slate 100
            }
        } else {
            if ui.visuals().dark_mode {
                egui::Color32::from_rgb(15, 20, 30) // Slate 950
            } else {
                egui::Color32::from_rgb(255, 255, 255) // White
            }
        };

        let stroke = if is_selected {
            egui::Stroke::new(1.5, egui::Color32::from_rgb(99, 102, 241)) // Indigo 500
        } else {
            egui::Stroke::new(1.0, ui.visuals().widgets.noninteractive.bg_stroke.color)
        };

        egui::Frame::none()
            .fill(fill)
            .rounding(12.0)
            .stroke(stroke)
            .inner_margin(egui::Margin::symmetric(14.0, 10.0))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.set_height(row_height - 20.0);

                if is_selected {
                    let bar_width = 4.0;
                    let bar_height = 24.0;
                    let bar_rect = egui::Rect::from_center_size(
                        egui::pos2(ui.min_rect().left() - 12.0, ui.min_rect().center().y),
                        egui::vec2(bar_width, bar_height)
                    );
                    ui.painter().rect_filled(
                        bar_rect,
                        egui::Rounding::same(2.0),
                        egui::Color32::from_rgb(99, 102, 241) // Indigo 500
                    );
                }
            match &item {
                ClipItem::Text { content, timestamp } => {
                    ui.allocate_ui(egui::vec2(ui.available_width(), row_height - 20.0), |ui| {
                        ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                            draw_icon_badge(ui, "text", is_selected);
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
                                    ui.add(
                                        egui::Label::new(
                                            egui::RichText::new(preview_text(content, self.preview_chars))
                                                .size(15.0)
                                                .monospace()
                                                .strong()
                                        )
                                        .truncate()
                                    );
                                    ui.add_space(2.0);
                                    ui.label(
                                        egui::RichText::new(format!(
                                            "{} chars · {}",
                                            content.chars().count(),
                                            relative_time(*timestamp)
                                        ))
                                        .size(12.5)
                                        .weak(),
                                    );
                                });
                            });

                            if row < 9 {
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    let shortcut_color = if is_selected {
                                        egui::Color32::from_rgba_unmultiplied(255, 255, 255, 180)
                                    } else {
                                        ui.visuals().weak_text_color()
                                    };
                                    ui.label(
                                        egui::RichText::new(format!("^{}", row + 1))
                                            .size(13.0)
                                            .monospace()
                                            .color(shortcut_color),
                                    );
                                });
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
                                draw_icon_badge(ui, "image", is_selected);
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
                                                .strong()
                                        )
                                        .truncate()
                                    );
                                    ui.add_space(2.0);
                                    ui.label(
                                        egui::RichText::new(image_subtitle(filename, *timestamp))
                                            .size(12.5)
                                            .weak(),
                                    );
                                });
                            });

                            if row < 9 {
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    let shortcut_color = if is_selected {
                                        egui::Color32::from_rgba_unmultiplied(255, 255, 255, 180)
                                    } else {
                                        ui.visuals().weak_text_color()
                                    };
                                    ui.label(
                                        egui::RichText::new(format!("^{}", row + 1))
                                            .size(13.0)
                                            .monospace()
                                            .color(shortcut_color),
                                    );
                                });
                            }
                        });
                    });
                }
            }
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
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
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

        egui::CentralPanel::default()
            .frame(egui::Frame::none()
                .fill(ctx.style().visuals.window_fill)
                .rounding(16.0)
                .stroke(egui::Stroke::new(1.0, ctx.style().visuals.widgets.noninteractive.bg_stroke.color))
            )
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    self.draw_header(ui);
                    self.draw_search(ui);
                    self.draw_body(ui);
                    ui.add_space(6.0);
                    self.draw_footer(ui);
                });
            });
    }
}

fn draw_empty_state(ui: &mut egui::Ui, title: &str, subtitle: &str) {
    ui.vertical_centered(|ui| {
        ui.add_space(100.0);
        let (icon_rect, _) = ui.allocate_exact_size(egui::vec2(44.0, 44.0), egui::Sense::hover());
        paint_text_icon(ui, icon_rect, ui.visuals().weak_text_color());

        ui.add_space(16.0);
        ui.label(egui::RichText::new(title).heading().strong());
        ui.add_space(8.0);
        ui.label(egui::RichText::new(subtitle).size(15.0).weak());
    });
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

fn image_subtitle(filename: &str, ts: u64) -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    image_subtitle_with_now(filename, ts, now)
}

fn image_subtitle_with_now(filename: &str, ts: u64, now: u64) -> String {
    if filename.is_empty() {
        relative_time_with_now(ts, now)
    } else {
        format!("{} · {}", filename, relative_time_with_now(ts, now))
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

    #[test]
    fn image_subtitle_handles_empty_filename() {
        assert_eq!(image_subtitle_with_now("", 90, 100), "10s ago");
        assert_eq!(image_subtitle_with_now("shot.png", 90, 100), "shot.png · 10s ago");
    }
}
