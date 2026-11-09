//! Noob's own terminal: the part of the app that runs your real shell.
//!
//! It's built in stages. So far:
//!   - `pty`: starts your shell in a pseudo-terminal, sends it keys, reads what it prints.
//!   - `parser`: turns those printed bytes into a list of plain instructions
//!     ("print A", "move the cursor", "turn text blue").
//!   - `grid`: carries out those instructions on a grid of cells, so we know
//!     exactly what the screen shows (letters, colours, cursor, history).

pub mod grid;
pub mod parser;
pub mod pty;
