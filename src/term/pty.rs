//! Running your shell inside Noob.
//!
//! A program like zsh or PowerShell only behaves like it does in a terminal app
//! (prompt, colours, Ctrl+C, password prompts) when it believes it's talking to a
//! real terminal. A *pseudo-terminal* ("PTY") is a pretend one that the operating
//! system gives us: the shell sits on one end, Noob holds the other. What we write
//! to our end arrives as keystrokes; what the shell prints comes back to us as bytes.
//!
//! The `portable-pty` crate does the operating-system part for us (on Windows it
//! uses "ConPTY"). Everything we do with the bytes is Noob's own work.
//!
//! Heads-up for Windows: ConPTY first prints `ESC[6n` ("where is your cursor?")
//! and starts the shell only once it gets an answer. Answering is the screen's
//! job (it knows where the cursor is), so whatever reads this output must reply.

use std::io::{Read, Write};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::thread;
use std::time::{Duration, Instant};

use portable_pty::{Child, CommandBuilder, ExitStatus, MasterPty, PtySize, native_pty_system};

/// How big the terminal is, counted in letters: `rows` lines of `cols` letters each.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Size {
    pub rows: u16,
    pub cols: u16,
}

impl Size {
    fn for_pty(self) -> PtySize {
        PtySize {
            rows: self.rows,
            cols: self.cols,
            // Some systems also want the size in pixels; nobody reads it, so 0 is fine.
            pixel_width: 0,
            pixel_height: 0,
        }
    }
}

/// How often `wait_for_exit` checks whether the shell has ended.
const EXIT_CHECK_EVERY: Duration = Duration::from_millis(20);

/// How much we read from the shell in one go.
const READ_CHUNK: usize = 8 * 1024;

/// Your shell, running in a pseudo-terminal that Noob holds the other end of.
pub struct Pty {
    /// Our end of the pseudo-terminal: used to resize it.
    master: Box<dyn MasterPty + Send>,
    /// Where our "keystrokes" go.
    writer: Box<dyn Write + Send>,
    /// The shell itself, so we can see whether it has ended (or end it).
    child: Box<dyn Child + Send + Sync>,
    /// What the shell prints, handed over by the reading thread (see `start`).
    output: Receiver<Vec<u8>>,
}

impl Pty {
    /// Starts your usual shell: the one in `$SHELL` on a Mac or Linux (zsh, bash…),
    /// PowerShell on Windows.
    pub fn start_shell(size: Size) -> Result<Pty, String> {
        Pty::start(shell_command(), size)
    }

    /// Starts any program in a pseudo-terminal of the given size.
    pub fn start(command: CommandBuilder, size: Size) -> Result<Pty, String> {
        let pair = native_pty_system()
            .openpty(size.for_pty())
            .map_err(|error| format!("couldn't make a pseudo-terminal: {error}"))?;
        let child = pair
            .slave
            .spawn_command(command)
            .map_err(|error| format!("couldn't start the shell: {error}"))?;
        // The shell now holds its own end, so we let go of ours. Otherwise the
        // pseudo-terminal would stay open after the shell ends, and reading
        // would wait forever.
        drop(pair.slave);

        let reader = pair
            .master
            .try_clone_reader()
            .map_err(|error| format!("couldn't read from the shell: {error}"))?;
        let writer = pair
            .master
            .take_writer()
            .map_err(|error| format!("couldn't type into the shell: {error}"))?;

        Ok(Pty {
            master: pair.master,
            writer,
            child,
            output: read_in_background(reader),
        })
    }

    /// Sends bytes to the shell as if they were typed. Enter is `\r`, like a
    /// real keyboard sends.
    pub fn write(&mut self, bytes: &[u8]) -> std::io::Result<()> {
        self.writer.write_all(bytes)?;
        self.writer.flush()
    }

    /// Whatever the shell printed since we last asked, waiting up to `timeout`
    /// for something to arrive. `None` means nothing came (or the shell is gone).
    pub fn read_timeout(&self, timeout: Duration) -> Option<Vec<u8>> {
        let mut bytes = match self.output.recv_timeout(timeout) {
            Ok(bytes) => bytes,
            Err(RecvTimeoutError::Timeout | RecvTimeoutError::Disconnected) => return None,
        };
        // Grab anything else that's already waiting too, so callers get it in one piece.
        bytes.extend(self.output.try_iter().flatten());
        Some(bytes)
    }

    /// Tells the shell the window changed size, so programs like `nano` redraw to fit.
    pub fn resize(&self, size: Size) -> Result<(), String> {
        self.master
            .resize(size.for_pty())
            .map_err(|error| format!("couldn't resize the terminal: {error}"))
    }

    /// The size the operating system thinks the terminal is.
    pub fn size(&self) -> Result<Size, String> {
        let size = self
            .master
            .get_size()
            .map_err(|error| format!("couldn't read the terminal size: {error}"))?;
        Ok(Size {
            rows: size.rows,
            cols: size.cols,
        })
    }

    /// Waits up to `timeout` for the shell to end (say, after you typed `exit`).
    /// `None` means it's still running.
    pub fn wait_for_exit(&mut self, timeout: Duration) -> Option<ExitStatus> {
        let started = Instant::now();
        loop {
            if let Ok(Some(status)) = self.child.try_wait() {
                return Some(status);
            }
            if started.elapsed() >= timeout {
                return None;
            }
            thread::sleep(EXIT_CHECK_EVERY);
        }
    }

    /// Ends the shell straight away (like closing a terminal tab).
    pub fn stop(mut self) {
        self.end_shell();
    }

    fn end_shell(&mut self) {
        if matches!(self.child.try_wait(), Ok(None)) {
            // If ending it fails, it already ended on its own: nothing left to do.
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}

impl Drop for Pty {
    /// A shell must never outlive Noob's terminal, so dropping a `Pty` ends it.
    fn drop(&mut self) {
        self.end_shell();
    }
}

/// The command for your usual shell.
fn shell_command() -> CommandBuilder {
    if cfg!(windows) {
        // `-NoLogo` skips the copyright banner PowerShell prints at the start.
        let mut command = CommandBuilder::new("powershell.exe");
        command.arg("-NoLogo");
        command
    } else {
        // portable-pty's default is the shell in `$SHELL` (or, failing that, the one
        // in your user account), started as a "login shell" just like Terminal does.
        CommandBuilder::new_default_prog()
    }
}

/// Reading waits until the shell prints something, which could be never. So a
/// helper thread does the waiting and passes each piece over a channel, and
/// Noob's window never freezes. It stops when the pseudo-terminal closes.
fn read_in_background(mut reader: Box<dyn Read + Send>) -> Receiver<Vec<u8>> {
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let mut buffer = [0u8; READ_CHUNK];
        loop {
            match reader.read(&mut buffer) {
                // 0 bytes means the other end closed; an error means the same here.
                Ok(0) | Err(_) => break,
                Ok(count) => {
                    // If nobody's listening any more, the `Pty` is gone: stop reading.
                    if sender.send(buffer[..count].to_vec()).is_err() {
                        break;
                    }
                }
            }
        }
    });
    receiver
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    /// Shells (PowerShell above all) can take a few seconds to start on a busy computer.
    const PATIENCE: Duration = Duration::from_secs(30);

    /// "Where is your cursor?" On Windows, ConPTY asks this before it starts the
    /// shell, and waits for the answer. Real terminals reply "row;column" (here,
    /// row 1 column 1). Later, Noob's screen grid will answer with the real place.
    const WHERE_IS_THE_CURSOR: &[u8] = b"\x1b[6n";
    const CURSOR_IS_TOP_LEFT: &[u8] = b"\x1b[1;1R";

    /// Reads one piece of output, answering any "where is your cursor?" in it.
    fn read_like_a_terminal(shell: &mut Pty, wait: Duration) -> Option<Vec<u8>> {
        let bytes = shell.read_timeout(wait)?;
        let questions = bytes
            .windows(WHERE_IS_THE_CURSOR.len())
            .filter(|piece| *piece == WHERE_IS_THE_CURSOR)
            .count();
        for _ in 0..questions {
            shell.write(CURSOR_IS_TOP_LEFT).expect("we can answer");
        }
        Some(bytes)
    }

    /// Keeps reading what the shell prints until `wanted` shows up (or we give up).
    fn read_until(shell: &mut Pty, wanted: &str) -> String {
        let started = Instant::now();
        let mut seen = Vec::new();
        while started.elapsed() < PATIENCE {
            if let Some(bytes) = read_like_a_terminal(shell, Duration::from_millis(100)) {
                seen.extend(bytes);
            }
            if String::from_utf8_lossy(&seen).contains(wanted) {
                break;
            }
        }
        String::from_utf8_lossy(&seen).into_owned()
    }

    /// Waits until the shell has printed something (its prompt) and then gone
    /// quiet, so it's ready for typing. Every shell's prompt looks different, so
    /// we can't wait for particular words.
    fn wait_until_ready(shell: &mut Pty) {
        let quiet = Duration::from_millis(500);
        let started = Instant::now();
        let mut printed_something = false;
        while started.elapsed() < PATIENCE {
            match read_like_a_terminal(shell, quiet) {
                Some(_) => printed_something = true,
                None if printed_something => return,
                None => {}
            }
        }
    }

    #[test]
    fn the_shell_runs_what_we_type_and_we_read_its_answer() {
        let mut shell = Pty::start_shell(Size { rows: 24, cols: 80 }).expect("shell starts");

        // The terminal shows what we type, so `echo hello` would put "hello" on the
        // screen even if the shell never ran it. `hel""lo` only turns into "hello"
        // when the shell really runs it (zsh, bash and PowerShell all join the pieces).
        wait_until_ready(&mut shell);
        shell.write(b"echo hel\"\"lo\r").expect("we can type");
        let seen = read_until(&mut shell, "hello");
        assert!(seen.contains("hello"), "the shell printed: {seen:?}");

        shell.write(b"exit\r").expect("we can type");
        assert!(
            shell.wait_for_exit(PATIENCE).is_some(),
            "the shell should end after `exit`"
        );
    }

    #[test]
    fn the_terminal_starts_at_the_size_we_ask_for_and_can_be_resized() {
        let shell = Pty::start_shell(Size { rows: 24, cols: 80 }).expect("shell starts");
        assert_eq!(shell.size(), Ok(Size { rows: 24, cols: 80 }));

        shell
            .resize(Size {
                rows: 40,
                cols: 120,
            })
            .expect("resizes");
        assert_eq!(
            shell.size(),
            Ok(Size {
                rows: 40,
                cols: 120
            })
        );
        shell.stop();
    }

    #[test]
    fn stopping_ends_a_shell_that_is_still_running() {
        let mut shell = Pty::start_shell(Size { rows: 24, cols: 80 }).expect("shell starts");
        shell.end_shell();
        assert!(
            shell.wait_for_exit(PATIENCE).is_some(),
            "the shell should be gone after stopping it"
        );
    }
}
