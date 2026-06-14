//! X11 event-driven clipboard monitoring using the XFixes extension.
//!
//! This replaces timer-based polling with true event-driven notifications:
//! when any application takes ownership of CLIPBOARD or PRIMARY, X11 sends
//! us an event and we wake up immediately to read the new content.
//!
//! If X11 is unavailable (Wayland pure, no $DISPLAY, etc.) the constructor
//! returns `None` and the caller falls back to traditional polling.

use std::os::unix::io::{AsRawFd, RawFd};
use x11rb::connection::Connection;
use x11rb::protocol::xfixes::{self, ConnectionExt as _};
use x11rb::protocol::xproto::ConnectionExt as XProtoConnectionExt;
use x11rb::protocol::Event;
use x11rb::xcb_ffi::XCBConnection;

/// Describes which selection changed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectionEvent {
    /// The CLIPBOARD selection owner changed.
    Clipboard,
    /// The PRIMARY selection owner changed.
    Primary,
}

/// An X11 event source for clipboard changes.
///
/// Usage:
/// 1. Call [`X11Watcher::try_new()`] – returns `None` if X11 is unavailable.
/// 2. Get the file descriptor via [`fd()`](Self::fd) for use with `poll(2)`.
/// 3. When the fd is readable, call [`poll_events()`](Self::poll_events).
pub struct X11Watcher {
    conn: XCBConnection,
    clipboard_atom: u32,
    primary_atom: u32,
}

impl X11Watcher {
    /// Connect to the X11 display and subscribe to clipboard owner-change
    /// events via the XFixes extension.
    ///
    /// Returns `None` when X11 is unreachable (Wayland, no $DISPLAY, …) or
    /// the XFixes extension is missing.
    pub fn try_new() -> Option<Self> {
        let (conn, screen_num) = XCBConnection::connect(None).ok()?;
        let screen = &conn.setup().roots[screen_num];

        // Resolve selection atoms
        let clipboard_atom = conn
            .intern_atom(false, b"CLIPBOARD")
            .ok()?
            .reply()
            .ok()?
            .atom;
        let primary_atom = conn.intern_atom(false, b"PRIMARY").ok()?.reply().ok()?.atom;

        // Negotiate XFixes extension version (required before using requests)
        let ver = conn.xfixes_query_version(2, 0).ok()?.reply().ok()?;
        if ver.major_version < 2 {
            eprintln!(
                "[x11] XFixes {} too old, need ≥2.0",
                ver.major_version
            );
            return None;
        }

        // Subscribe to XFixes selection-notify events
        let r1 = conn
            .xfixes_select_selection_input(
                screen.root,
                clipboard_atom,
                xfixes::SelectionEventMask::SET_SELECTION_OWNER,
            )
            .ok()?;
        let r2 = conn
            .xfixes_select_selection_input(
                screen.root,
                primary_atom,
                xfixes::SelectionEventMask::SET_SELECTION_OWNER,
            )
            .ok()?;

        // Flush and check for async errors
        conn.flush().ok()?;
        if r1.check().is_err() || r2.check().is_err() {
            eprintln!("[x11] failed to subscribe to selection events (check errors)");
            return None;
        }

        Some(Self {
            conn,
            clipboard_atom,
            primary_atom,
        })
    }

    /// The X11 connection's file descriptor — pass this to `poll(2)`.
    pub fn fd(&self) -> RawFd {
        self.conn.as_raw_fd()
    }

    /// Process all pending X11 events and return any selection changes.
    ///
    /// Call this when `fd()` becomes readable.
    pub fn poll_events(&mut self) -> Vec<SelectionEvent> {
        let mut events = Vec::new();
        loop {
            match self.conn.poll_for_event() {
                Ok(Some(Event::XfixesSelectionNotify(xf))) => {
                    if crate::ui::theme::is_debug_logging() {
                        eprintln!(
                            "[x11] SelectionNotify: selection={}, owner={}",
                            xf.selection, xf.owner
                        );
                    }
                    if xf.selection == self.clipboard_atom {
                        events.push(SelectionEvent::Clipboard);
                    } else if xf.selection == self.primary_atom {
                        events.push(SelectionEvent::Primary);
                    }
                }
                Ok(Some(other)) => {
                    if crate::ui::theme::is_debug_logging() {
                        eprintln!("[x11] other event: {:?}", other);
                    }
                    continue;
                }
                Ok(None) => break,
                Err(e) => {
                    if crate::ui::theme::is_debug_logging() {
                        eprintln!("[x11] poll_for_event error: {:?}", e);
                    }
                    break;
                }
            }
        }
        events
    }

    /// Flush the X11 connection (ensures requests are sent).
    pub fn flush(&self) {
        let _ = self.conn.flush();
    }
}
