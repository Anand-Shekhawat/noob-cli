//! Noob's own terminal: the part of the app that runs your real shell.
//!
//! It's built in stages. So far:
//!   - `pty`: starts your shell in a pseudo-terminal, sends it keys, reads what it prints.
//!   - `parser`: turns those printed bytes into a list of plain instructions
//!     ("print A", "move the cursor", "turn text blue").

pub mod parser;
pub mod pty;
