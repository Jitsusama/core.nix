//! wl-copy and wl-paste for pi's fenced commands, in one program told
//! apart by the name it's run as, asking the clipboard door outside the
//! fence (src/shim.rs).

use std::env;
use std::ffi::OsString;
use std::io::{self, Write};
use std::process::ExitCode;

use pi_clipboard_door::shim;
use terminal_kit::{Environment, Exit, Failure, Help};

fn main() -> ExitCode {
    let mut args = env::args_os();
    let name = args.next().unwrap_or_default();
    let args: Result<Vec<String>, OsString> = args.map(OsString::into_string).collect();
    let environment = Environment::here();
    let ending = match args {
        Ok(args) => shim::ending(
            &name.to_string_lossy(),
            args,
            &environment,
            shim::door().as_deref(),
            &mut io::stdin().lock(),
            &mut io::stdout().lock(),
        ),
        // wl-clipboard takes text it can't read too, but a fenced command
        // says what it copies through stdin as readily, so this is refused
        // rather than guessed at.
        Err(word) => terminal_kit::run(
            &Help::new("pi-clipboard-shim", "wl-copy and wl-paste for pi's fence"),
            env!("CARGO_PKG_VERSION"),
            &environment,
            Vec::new(),
            |_| {
                Err(Failure::new(Exit::Failed, "an argument isn't UTF-8 text")
                    .because(format!("{} can't be read as text", word.to_string_lossy()))
                    .try_this("give the text on stdin"))
            },
        ),
    };
    // Nothing is left to say if a stream has closed, as it has when a
    // pipe's reader stops early.
    let _ = io::stdout().lock().write_all(ending.stdout.as_bytes());
    let _ = io::stderr().lock().write_all(ending.stderr.as_bytes());
    ending.exit.into()
}
