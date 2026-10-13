//! The disk guard's command line; `cli` reads it and runs it.

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn main() {
    use disk_guard::cli::{Env, REFUSED, parse, run};
    let args: Vec<std::ffi::OsString> = std::env::args_os().skip(1).collect();
    let command = match parse(&args) {
        Ok(command) => command,
        Err(why) => {
            eprintln!("disk-guard: {why}");
            std::process::exit(REFUSED);
        }
    };
    let code = run(
        command,
        &Env::from_environment(),
        &mut std::io::stdout().lock(),
        &mut std::io::stderr(),
    );
    std::process::exit(code);
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
fn main() {
    eprintln!("disk-guard: this system isn't supported yet");
    std::process::exit(1);
}
