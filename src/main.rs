use easycopy::clipboard::ClipboardMonitor;
use easycopy::clipboard::x11::{SelectionEvent, X11Watcher};
use easycopy::config::dirs::Directories;
use easycopy::clipboard::history::{ClipItem, HistoryManager};
use easycopy::hotkey::parse_hotkey;
use easycopy::ipc;
use easycopy::launcher::desktop;
use easycopy::store::ImageStore;
use easycopy::store::Store;
use easycopy::ui::popup;
use easycopy::ui::theme;
use std::collections::VecDeque;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::{Duration, Instant};

const HISTORY_SAVE_INTERVAL: Duration = Duration::from_secs(1);

fn main() {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(|s| s.as_str()) {
        Some("--popup") => cmd_popup(),
        Some("--clear") => cmd_clear(),
        Some("--version") | Some("-V") => cmd_version(),
        Some("--help") | Some("-h") => cmd_help(),
        _ => run_daemon(),
    }
}

// ── commands ───────────────────────────────────────────────────────

fn cmd_popup() {
    let dirs = Directories::discover();
    let store = Store::new(dirs);
    let config = store.load_config();
    let should_paste = Arc::new(AtomicBool::new(false));
    popup::run_popup(config, should_paste, store);
}

fn cmd_clear() {
    let dirs = Directories::discover();
    let store = Store::new(dirs);
    let image_store = store.images();
    let items = store.load_history();
    for item in &items {
        if let ClipItem::Image { filename, .. } = item {
            if !filename.is_empty() {
                image_store.delete(filename);
            }
        }
    }

    if store.save_history(&VecDeque::new()).is_ok() {
        println!("History cleared.");
    } else {
        eprintln!("Failed to clear history.");
        std::process::exit(1);
    }
}

fn cmd_version() {
    println!("easycopy 0.2.0");
}

fn cmd_help() {
    println!(
        r#"easycopy — minimal clipboard history manager

USAGE:
    easycopy              Start daemon (monitor clipboard + hotkey)
    easycopy --popup      Show the history popup
    easycopy --clear      Delete all history and saved images
    easycopy --version    Print version
    easycopy --help       Show this help

CONFIG:
    ~/.config/easycopy/config.toml

DATA:
    ~/.local/share/easycopy/index.json
    ~/.local/share/easycopy/images/
"#
    );
}

// ── daemon ─────────────────────────────────────────────────────────

fn run_daemon() {
    let dirs = Directories::discover();
    let store = Store::new(dirs);
    let config = store.load_config();
    theme::set_debug_logging(config.general.debug_logging);

    let image_store = store.images();
    let _ = std::fs::create_dir_all(store.data_dir());
    let _ = std::fs::create_dir_all(image_store.dir());

    // Write PID file to allow the popup to verify the daemon is active
    let pid = std::process::id();
    let _ = std::fs::write(store.history_path().parent().unwrap().join("daemon.pid"), pid.to_string());

    // Start IPC server for event-driven popup→daemon communication
    let ipc_rx = match ipc::start_server(&ipc::socket_path(&store)) {
        Ok(rx) => {
            eprintln!("[daemon] IPC server started");
            Some(rx)
        }
        Err(e) => {
            eprintln!("[daemon] warning: could not start IPC server: {e}");
            eprintln!("[daemon] popup paste requests will be handled directly");
            None
        }
    };

    // ── Pre-cache desktop apps and image thumbnails ──────────────
    {
        let history_items = store.load_history();
        let images_dir = image_store.dir().to_path_buf();
        let store_for_cache = store.clone();
        std::thread::Builder::new()
            .name("precache".into())
            .spawn(move || {
                // Cache desktop apps (slow I/O scan)
                let apps = desktop::load_desktop_apps();
                if let Err(e) = store_for_cache.save_apps_cache(&apps) {
                    eprintln!("[daemon] warning: failed to write apps cache: {e}");
                } else {
                    eprintln!("[daemon] cached {} desktop apps", apps.len());
                }

                // Pre-compute missing thumbnails for all image clips
                for item in &history_items {
                    if let ClipItem::Image { filename, .. } = item {
                        if filename.is_empty() {
                            continue;
                        }
                        let thumb_path = images_dir.join(format!("thumb_{}", filename));
                        if thumb_path.exists() {
                            continue;
                        }
                        let src_path = images_dir.join(filename);
                        if !src_path.exists() {
                            continue;
                        }
                        if let Ok(img) = image::open(&src_path) {
                            let thumb = img.resize(52, 52, image::imageops::FilterType::Triangle);
                            let _ = thumb.save(&thumb_path);
                        }
                    }
                }
            })
            .ok();
    }

    let mut history = HistoryManager::new(
        config.general.max_text_items,
        config.general.max_image_items,
    );
    history.set_items(store.load_history());
    let _ = image_store.cleanup_orphaned(history.items());

    let mut monitor = ClipboardMonitor::new();

    use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};
    let hotkey_mgr = GlobalHotKeyManager::new().ok();
    let parsed_hotkey = parse_hotkey(&config.general.hotkey);
    let mut hotkey_registered = false;

    if let (Some(ref mgr), Some(hk)) = (&hotkey_mgr, parsed_hotkey) {
        match mgr.register(hk) {
            Ok(()) => {
                eprintln!("[daemon] hotkey registered: {}", config.general.hotkey);
                hotkey_registered = true;
            }
            Err(e) => {
                eprintln!("[daemon] warning: could not register hotkey: {e}");
                eprintln!("[daemon] you can still use: easycopy --popup");
            }
        }
    } else {
        eprintln!(
            "[daemon] warning: could not parse hotkey '{}', running without hotkey",
            config.general.hotkey
        );
        eprintln!("[daemon] you can still use: easycopy --popup");
    }

    // Try X11 event-driven clipboard monitoring (falls back to timer polling)
    let mut x11_watcher = X11Watcher::try_new();
    if x11_watcher.is_some() {
        eprintln!("[daemon] X11 clipboard event source active");
    } else {
        eprintln!("[daemon] X11 not available — using polling fallback");
    }

    eprintln!(
        "[daemon] started — max {} text / {} images",
        config.general.max_text_items, config.general.max_image_items,
    );

    let hotkey_rx = GlobalHotKeyEvent::receiver();

    let poll_interval = config.general.poll_interval_ms;
    let tick_ms = 50;
    let ticks_per_poll = (poll_interval / tick_ms).max(1);
    let mut tick_count = 0u64;
    let mut last_history_save: Option<Instant> = None;

    loop {
        // ── IPC paste requests from popup ────────────────────────
        if let Some(ref rx) = ipc_rx {
            while let Ok(item) = rx.try_recv() {
                if let Ok(mut cb) = arboard::Clipboard::new() {
                    let write_ok = match item {
                        ClipItem::Text { content, .. } => cb.set_text(content).is_ok(),
                        ClipItem::Image { filename, .. } => {
                            if let Ok((w, h, data)) = image_store.load(&filename) {
                                let img_data = arboard::ImageData {
                                    width: w as usize,
                                    height: h as usize,
                                    bytes: std::borrow::Cow::Owned(data),
                                };
                                cb.set_image(img_data).is_ok()
                            } else {
                                false
                            }
                        }
                    };
                    if write_ok && config.general.auto_paste {
                        std::thread::sleep(Duration::from_millis(config.general.paste_delay_ms));
                        let _ = std::process::Command::new("xdotool")
                            .args(["key", "ctrl+v"])
                            .status();
                    }
                }
            }
        }

        // ── Clipboard monitoring ──────────────────────────────────
        if let Some(ref mut x11) = x11_watcher {
            // Event-driven: check for XFixes selection events
            let x11_events = x11.poll_events();
            if theme::is_debug_logging() && !x11_events.is_empty() {
                eprintln!("[daemon] X11 events: {:?}", x11_events);
            }
            for event in &x11_events {
                if *event == SelectionEvent::Clipboard && config.general.enable_clipping {
                    if theme::is_debug_logging() {
                        eprintln!("[daemon] clipboard event → checking monitor");
                    }
                    if let Some(raw) = monitor.poll() {
                        if theme::is_debug_logging() {
                            eprintln!("[daemon] monitor.poll() returned: {:?}", raw);
                        }
                        let _ = process_clip_item(raw, &mut history, &mut last_history_save, &image_store, &store);
                    } else if theme::is_debug_logging() {
                        eprintln!("[daemon] monitor.poll() returned None (no change)");
                    }
                }
            }
        } else {
            // Fallback: timer-based polling
            tick_count += 1;
            if tick_count >= ticks_per_poll {
                tick_count = 0;
                if config.general.enable_clipping {
                    if let Some(raw) = monitor.poll() {
                        let _ = process_clip_item(raw, &mut history, &mut last_history_save, &image_store, &store);
                    }
                }
            }
        }

        // ── Hotkey: show popup ───────────────────────────────────
        if hotkey_registered {
            if let Ok(event) = hotkey_rx.try_recv() {
                if event.state == HotKeyState::Pressed {
                    if let Ok(exe) = std::env::current_exe() {
                        let _ = std::process::Command::new(exe).arg("--popup").spawn();
                    }
                }
            }
        }

        // ── Wait ──────────────────────────────────────────────────
        if let Some(ref mut x11) = x11_watcher {
            // Block on the X11 fd (wake immediately on clipboard events)
            let x11_fd = x11.fd();
            let mut pollfd = libc::pollfd {
                fd: x11_fd,
                events: libc::POLLIN,
                revents: 0,
            };
            let ret = unsafe { libc::poll(&mut pollfd, 1, 50) };
            if theme::is_debug_logging() && ret > 0 {
                eprintln!("[daemon] poll() woke (revents={})", pollfd.revents,);
            }
        } else {
            std::thread::sleep(Duration::from_millis(tick_ms));
        }
    }
}

/// Process a clip item detected by the monitor — saves images, adds to history.
/// Returns the processed item if it was added to history, None otherwise.
fn process_clip_item(
    raw: ClipItem,
    history: &mut HistoryManager,
    last_save: &mut Option<Instant>,
    image_store: &ImageStore,
    store: &Store,
) -> Option<ClipItem> {
    let item = match raw {
        ClipItem::Image {
            data: Some(bytes),
            width,
            height,
            timestamp,
            ..
        } => match image_store.save_owned(bytes, width, height) {
            Ok(filename) => ClipItem::Image {
                width,
                height,
                timestamp,
                filename,
                data: None,
                use_count: 0,
            },
            Err(e) => {
                eprintln!("[daemon] failed to save image: {e}");
                return None;
            }
        },
        other => other,
    };

    if history.add(item.clone()) {
        let now = Instant::now();
        if last_save
            .map(|t| now.duration_since(t) > HISTORY_SAVE_INTERVAL)
            .unwrap_or(true)
        {
            if let Err(e) = store.save_history(history.items()) {
                eprintln!("[daemon] failed to save history: {e}");
            }
            *last_save = Some(now);
        }
        Some(item)
    } else {
        None
    }
}
