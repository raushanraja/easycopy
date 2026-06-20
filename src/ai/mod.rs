pub mod client;
pub mod session;
pub mod worker;

/// A single rendered chat message (user turn or assistant reply).
#[derive(Debug, Clone)]
pub enum ChatMessage {
    User(String),
    Assistant(String),
}
