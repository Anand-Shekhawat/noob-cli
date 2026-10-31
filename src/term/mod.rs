//! Noob's own terminal: the part of the app that runs your real shell.
//!
//! It's built in stages. So far:
//!   - `pty`: starts your shell in a pseudo-terminal, sends it keys, reads what it prints.

pub mod pty;
