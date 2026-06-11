use std::collections::VecDeque;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Duration;

use clipit_rs::clipboard::ClipboardMonitor;
use clipit_rs::config::Config;
use clipit_rs::history::{ClipItem, HistoryManager};
use clipit_rs::hotkey::parse_hotkey;
use clipit_rs::ipc;
use clipit_rs::popup;
use clipit_rs::storage;
use clipit_rs::theme;

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
    let config = Config::load();
    let should_paste = Arc::new(AtomicBool::new(false));
    popup::run_popup(config, should_paste);
}

fn cmd_clear() {
    let items = storage::load_history();
    for item in &items {
        if let ClipItem::Image { filename, .. } = item {
            if !filename.is_empty() {
                storage::delete_image_file(filename);
            }
        }
    }

    if storage::save_history(&VecDeque::new()).is_ok() {
        println!("History cleared.");
    } else {
        eprintln!("Failed to clear history.");
        std::process::exit(1);
    }
}

fn cmd_version() {
    println!("clipit-rs 0.2.0");
}

fn cmd_help() {
    println!(
        r#"clipit-rs — minimal clipboard history manager

USAGE:
    clipit-rs              Start daemon (monitor clipboard + hotkey)
    clipit-rs --popup      Show the history popup
    clipit-rs --clear      Delete all history and saved images
    clipit-rs --version    Print version
    clipit-rs --help       Show this help

CONFIG:
    ~/.config/clipit/config.toml

DATA:
    ~/.local/share/clipit/index.json
    ~/.local/share/clipit/images/
"#
    );
}

// ── daemon ─────────────────────────────────────────────────────────

fn run_daemon() {
    let config = Config::load();
    theme::set_debug_logging(config.general.debug_logging);

    // Cache directory paths to avoid repeated PathBuf construction in the hot loop
    let data_dir = Config::data_dir();
    let _ = std::fs::create_dir_all(&data_dir);
    let _ = std::fs::create_dir_all(Config::images_dir());

    // Write PID file to allow the popup to verify the daemon is active
    let pid = std::process::id();
    let pid_file = data_dir.join("daemon.pid");
    let _ = std::fs::write(&pid_file, pid.to_string());

    // Start IPC server for event-driven popup→daemon communication
    let ipc_rx = match ipc::start_server(&data_dir.join("daemon.sock")) {
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

    let mut history = HistoryManager::new(
        config.general.max_text_items,
        config.general.max_image_items,
    );
    history.set_items(storage::load_history());
    storage::cleanup_orphaned(history.items());

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
                eprintln!("[daemon] you can still use: clipit-rs --popup");
            }
        }
    } else {
        eprintln!(
            "[daemon] warning: could not parse hotkey '{}', running without hotkey",
            config.general.hotkey
        );
        eprintln!("[daemon] you can still use: clipit-rs --popup");
    }

    eprintln!(
        "[daemon] started — polling every {}ms, max {} text / {} images",
        config.general.poll_interval_ms,
        config.general.max_text_items,
        config.general.max_image_items,
    );

    let hotkey_rx = GlobalHotKeyEvent::receiver();

    let poll_interval = config.general.poll_interval_ms;
    let tick_ms = 50;
    let ticks_per_poll = (poll_interval / tick_ms).max(1);
    let mut tick_count = 0;

    loop {
        // Handle paste requests from popup via IPC (event-driven, no filesystem polling)
        if let Some(ref rx) = ipc_rx {
            while let Ok(item) = rx.try_recv() {
                if let Ok(mut cb) = arboard::Clipboard::new() {
                    let write_ok = match item {
                        ClipItem::Text { content, .. } => cb.set_text(content).is_ok(),
                        ClipItem::Image { filename, .. } => {
                            if let Ok((w, h, data)) = storage::load_image(&filename) {
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

        tick_count += 1;
        if tick_count >= ticks_per_poll {
            tick_count = 0;
            if config.general.enable_clipping {
                if let Some(raw) = monitor.poll() {
                    let item = match raw {
                        ClipItem::Image {
                            data: Some(bytes),
                            width,
                            height,
                            timestamp,
                            ..
                        } => match storage::save_image_owned(bytes, width, height) {
                            Ok(filename) => ClipItem::Image {
                                width,
                                height,
                                timestamp,
                                filename,
                                data: None,
                            },
                            Err(e) => {
                                eprintln!("[daemon] failed to save image: {e}");
                                std::thread::sleep(Duration::from_millis(tick_ms));
                                continue;
                            }
                        },
                        other => other,
                    };

                    if history.add(item) {
                        if let Err(e) = storage::save_history(history.items()) {
                            eprintln!("[daemon] failed to save history: {e}");
                        }
                    }
                }
            }
        }

        if hotkey_registered {
            if let Ok(event) = hotkey_rx.try_recv() {
                if event.state == HotKeyState::Pressed {
                    if let Ok(exe) = std::env::current_exe() {
                        let _ = std::process::Command::new(exe).arg("--popup").spawn();
                    }
                }
            }
        }

        std::thread::sleep(Duration::from_millis(tick_ms));
    }
}
