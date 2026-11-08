//! `github-broker serve` holds the App and answers its socket;
//! `github-broker token` asks it, and prints the token alone.

use std::path::PathBuf;
use std::process::ExitCode;

use github_broker::app::App;
use github_broker::mint::Minter;
use github_broker::wire;

const USAGE: &str = "usage: github-broker serve --app FILE --installation FILE --identity FILE \
                     --socket PATH [--api URL]\n       github-broker token --socket PATH";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let named = |name: &str| {
        args.iter()
            .position(|arg| arg == name)
            .and_then(|at| args.get(at + 1))
            .cloned()
    };
    let Some(socket) = named("--socket").map(PathBuf::from) else {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    };
    match args.first().map(String::as_str) {
        Some("serve") => {
            let (Some(app), Some(installation), Some(identity)) =
                (named("--app"), named("--installation"), named("--identity"))
            else {
                eprintln!("{USAGE}");
                return ExitCode::from(2);
            };
            let api = named("--api").unwrap_or_else(|| "https://api.github.com".to_owned());
            let listener = match wire::listener(&socket) {
                Ok(listener) => listener,
                Err(why) => {
                    eprintln!("github-broker: {why}");
                    return ExitCode::FAILURE;
                }
            };
            let mut minter = Minter::new(
                &api,
                Box::new(move || {
                    App::open(
                        &PathBuf::from(&app),
                        &PathBuf::from(&installation),
                        &PathBuf::from(&identity),
                    )
                }),
            );
            wire::serve(&listener, &mut minter);
            ExitCode::SUCCESS
        }
        Some("token") => match wire::ask(&socket) {
            Ok(token) => {
                println!("{}", token.value);
                ExitCode::SUCCESS
            }
            Err(why) => {
                eprintln!("github-broker: {why}");
                ExitCode::FAILURE
            }
        },
        _ => {
            eprintln!("{USAGE}");
            ExitCode::from(2)
        }
    }
}
