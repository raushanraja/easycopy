//! Browser action shortcuts and URL resolution.
//!
//! Domain: resolving user queries to URLs (shortcuts, domains,
//! Google fallback), searching saved actions, and opening URLs.

pub mod action;
pub mod open;

pub use action::{
    filter_query, open_url, percent_encode, BrowserAction, QueryMode, SHORTCUTS,
};
pub use open::{open_item, open_url as open_url_system, OpenTarget};
