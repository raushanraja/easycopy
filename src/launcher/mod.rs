//! Desktop application discovery and launching.
//!
//! Domain: scanning .desktop files, resolving icons, and
//! recording app launch statistics.

pub mod desktop;
pub mod icon;
pub mod parser;

pub use desktop::DesktopApp;
pub use icon::{find_case_insensitive, icon_search_dirs, resolve_icon, resolve_icon_in};
pub use parser::{parse_desktop_file, parse_desktop_file_with, strip_exec_codes};
