//! What a program has open now (open.rs), for its Recent in the launcher.
//!
//! `open-now zathura` prints each zathura with a document open, a line
//! each: its process id, a tab and the document's whole path. With none
//! open it prints nothing.

use std::env;
use std::io::{self, Write};
use std::process::ExitCode;

use open_with::open;
use zbus::Connection;

const USAGE: &str = "usage: open-now zathura";

#[tokio::main(flavor = "current_thread")]
async fn main() -> ExitCode {
    let arguments: Vec<String> = env::args().skip(1).collect();
    let done = match arguments.as_slice() {
        [program] if program == "zathura" => said().await,
        _ => Err(USAGE.to_owned()),
    };
    match done {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("open-now: {error}");
            ExitCode::FAILURE
        }
    }
}

async fn said() -> Result<(), String> {
    let connection = Connection::session()
        .await
        .map_err(|error| error.to_string())?;
    let open = open::zathura(&connection)
        .await
        .map_err(|error| error.to_string())?;
    let mut out = io::stdout().lock();
    open.iter()
        .try_for_each(|(pid, file)| writeln!(out, "{pid}\t{file}"))
        .map_err(|error| error.to_string())
}
