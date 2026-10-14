//! The spend-watch command: the daemon, its reports and its errands
//! (src/cli.rs lists them).

fn main() {
    std::process::exit(spend_watch::cli::main(std::env::args().skip(1).collect()));
}
