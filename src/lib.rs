//! Noob's engine: everything except the screen it's shown on.
//!
//! Two programs use it: `noob` (in a terminal, see `main.rs`) and the Noob app
//! (in its own window, see `app/main.rs`). Both send what you type to `engine`.

pub mod actions;
pub mod chat;
pub mod command;
pub mod complete;
pub mod engine;
pub mod help;
pub mod parser;
pub mod real;
pub mod style;
pub mod suggest;
pub mod table;
// Noob's own terminal (your real shell inside Noob's window), only in the app.
#[cfg(feature = "app")]
pub mod term;
pub mod terminal;
pub mod tools;
