//! IPC communication between daemon and popup.
//!
//! Domain: Unix domain socket server/client for paste requests.

pub mod socket;

pub use socket::{send_paste_request, socket_path, start_server};
