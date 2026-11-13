//! The disk guard's command line; `cli` reads it and runs it, and the
//! terminal kit gives it the shape every command of ours has.

use std::process::ExitCode;

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn main() -> ExitCode {
    use disk_guard::cli::{Env, help, parse, run};
    terminal_kit::main(&help(), env!("CARGO_PKG_VERSION"), |context| {
        run(
            parse(&context.args)?,
            &Env::from_environment(),
            context,
            &mut std::io::stdout().lock(),
            &mut std::io::stderr(),
        )
    })
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
fn main() -> ExitCode {
    use terminal_kit::{Exit, Failure, Help};
    let help = Help::new(
        "disk-guard",
        "keeps a disk from filling without breaking a build",
    );
    terminal_kit::main(&help, env!("CARGO_PKG_VERSION"), |_| {
        Err(Failure::new(
            Exit::Unavailable,
            "this system isn't supported yet",
        ))
    })
}
