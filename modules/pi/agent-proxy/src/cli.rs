//! The proxy's command line, in the terminal kit's shape: what
//! `pi-agent-proxy --help` says, and the refusal for words it doesn't
//! take. Why each flag exists is main.rs's note.

use terminal_kit::{Exit, Failure, Help};

/// The flags it takes, each with a value.
pub const FLAGS: [&str; 5] = ["--hosts", "--upstream", "--key", "--lent", "--socket"];

/// What `pi-agent-proxy --help` says.
pub fn help() -> Help {
    Help::new(
        "pi-agent-proxy",
        "the SSH agent a fenced pi command reaches, signing only logins to the hosts it's told of",
    )
    .example(
        "pi-agent-proxy --hosts hosts --upstream \"$SSH_AUTH_SOCK\" --socket proxy.sock",
        "signs logins to the hosts in hosts with the agent's keys, listening on proxy.sock",
    )
    .option(
        "--hosts FILE",
        "the hosts it signs logins to, an OpenSSH public key a line",
    )
    .option(
        "--upstream SOCKET",
        "an agent whose keys it signs with; again for each",
    )
    .option(
        "--key FILE",
        "the builders' key, signing only a login to a builder lent now",
    )
    .option(
        "--lent FILE",
        "a file of the builders lent now, read afresh at each login; again for each",
    )
    .option(
        "--socket PATH",
        "where it listens; the socket systemd passes",
    )
    .exit(Exit::Failed, "it had nowhere to listen")
    .exit(Exit::Config, "its hosts file couldn't be read")
}

/// The refusal for words it doesn't take: asked wrongly, 64, saying
/// where its help is.
pub fn wrongly(why: impl Into<String>) -> Failure {
    Failure::new(Exit::Usage, why).try_this("pi-agent-proxy --help")
}
