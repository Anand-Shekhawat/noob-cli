//! Tab completion for the `noob>` prompt: connects Noob's `complete` module to
//! rustyline, the crate that handles the keyboard.

use noob_cli::complete::complete;
use rustyline::completion::{Completer, Pair};
use rustyline::highlight::Highlighter;
use rustyline::hint::Hinter;
use rustyline::validate::Validator;
use rustyline::{Context, Helper};

pub struct NoobHelper;

impl Completer for NoobHelper {
    type Candidate = Pair;

    fn complete(
        &self,
        line: &str,
        cursor: usize,
        _context: &Context<'_>,
    ) -> rustyline::Result<(usize, Vec<Pair>)> {
        let (start, choices) = complete(&line[..cursor]);
        let pairs = choices
            .into_iter()
            .map(|choice| Pair {
                display: choice.display,
                replacement: choice.replacement,
            })
            .collect();
        Ok((start, pairs))
    }
}

// rustyline wants these too. Empty `impl`s mean "use the default: do nothing extra".
impl Hinter for NoobHelper {
    type Hint = String;
}
impl Highlighter for NoobHelper {}
impl Validator for NoobHelper {}
impl Helper for NoobHelper {}
