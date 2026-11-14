//! What the command line says of itself: its tree of resources and what
//! each answers, apart from the reads and acts behind them, since the
//! words change for other reasons than the work.

use terminal_kit::{Exit, Help};

/// The help, declared once for the text and for the commands check.
pub fn help() -> Help {
    Help::new("spend-watch", "what AI use costs")
        .example("spend-watch", "the week at a glance")
        .example(
            "spend-watch report last week by quest",
            "a period, cut one way",
        )
        .example("spend-watch report waste", "context rebuilt, by cause")
        .example("spend-watch budgets --json", "each budget, for a machine")
        .subcommand(
            reads("report", "a period's spend, cut one way, or one focus")
                .example("spend-watch report", "the week at a glance")
                .example(
                    "spend-watch report last week by quest",
                    "a period, cut one way",
                )
                .example("spend-watch report waste", "context rebuilt, by cause")
                .argument(
                    "[words]...",
                    "a period and by a cut, or waste, caps, quests, outliers or forecast",
                ),
        )
        .subcommand(
            reads("budgets", "each budget against its allowances")
                .example("spend-watch budgets", "each budget, and how it stands")
                .example(
                    "spend-watch budgets --session ID",
                    "with that session's spend",
                )
                .option("--session <id>", "one session's spend too"),
        )
        .subcommand(findings_help())
        .subcommand(
            reads("backtest", "the forecast and the notices, replayed")
                .example("spend-watch backtest", "how the forecast would have done"),
        )
        .subcommand(tokens_help())
        .subcommand(
            reads("refresh", "ask the daemon for a pass now")
                .example("spend-watch refresh", "the shell's figures, read again")
                .exit(Exit::Unavailable, "the daemon isn't running to ask"),
        )
        .subcommand(sessions_help())
        .subcommand(old_go_help())
        .subcommand(
            Help::new("contract", "a feed's contract")
                .example(
                    "spend-watch contract check feed.jsonl",
                    "each line it breaks",
                )
                .exit(Exit::BadInput, "a line doesn't hold")
                .subcommand(
                    Help::new("check", "a feed's lines against its contract")
                        .example(
                            "spend-watch contract check feed.jsonl",
                            "each line it breaks",
                        )
                        .argument("<file>", "the feed's lines")
                        .exit(Exit::BadInput, "a line doesn't hold"),
                ),
        )
        .subcommand(
            Help::new("check-config", "whether a config can run")
                .example(
                    "spend-watch check-config config.toml",
                    "read as the daemon would",
                )
                .argument("<file>", "the config")
                .exit(Exit::BadInput, "the config doesn't hold"),
        )
        .subcommand(
            reads("show", "the status the shell would draw now")
                .example("spend-watch show", "the shell's figures, as it reads them")
                // The launcher's collection, which only it asks for.
                .subcommand(reads("collection", "the launcher's collection").unlisted()),
        )
        .subcommand(
            reads("import", "read what's new from pi and the harness")
                .example("spend-watch import", "what the daemon reads each pass"),
        )
        .subcommand(
            reads("run", "the daemon")
                .example("spend-watch run", "as its service runs it")
                .exit(Exit::TryAgain, "another spend-watch holds the history"),
        )
        .subcommand(
            reads("serve", "serve the daemon's last files, for fixtures")
                .example("spend-watch serve", "its files, until stopped"),
        )
        .option("--config <file>", "the config; the module's by default")
        .form("markdown")
        .form("csv")
        .exit(Exit::Failed, "the history couldn't be read or kept")
        .exit(Exit::BadInput, "a feed's line or a config doesn't hold")
        .exit(Exit::NotFound, "a file, finding or session isn't there")
        .exit(Exit::Unavailable, "the daemon isn't running to ask")
        .exit(Exit::TryAgain, "another spend-watch holds the history")
        .exit(Exit::Config, "the config can't be read")
}

/// `findings`, one of them, and marking one expected.
fn findings_help() -> Help {
    reads("findings", "what holds now, or one finding")
        .alias("finding")
        .example("spend-watch findings", "each finding standing")
        .example("spend-watch findings KEY", "the one whose key starts so")
        .example("spend-watch findings KEY expect", "KEY stops notifying")
        .argument("<key>", "a finding's key, or its start")
        .exit(Exit::NotFound, "no finding has that key")
        .subcommand(
            reads("expect", "marks the finding as expected")
                .example("spend-watch findings KEY expect", "KEY stops notifying")
                .exit(Exit::NotFound, "no finding has that key"),
        )
        // The order notices written before still say.
        .subcommand(
            reads("expect", "marks a finding as expected")
                .argument("<key>", "the finding's key")
                .unlisted(),
        )
}

/// `go <session>`, which notices written before `sessions ID go` still
/// say.
fn old_go_help() -> Help {
    reads("go", "bring a session's terminal forward")
        .example("spend-watch go ID", "its window, in front")
        .argument("<session>", "the session's id")
        .exit(Exit::NotFound, "no such session is open")
        .unlisted()
}

/// `sessions`: the week by session, one's spend, or its window.
fn sessions_help() -> Help {
    reads("sessions", "the week by session, or one session")
        .alias("session")
        .example("spend-watch sessions", "the week, cut by session")
        .example("spend-watch sessions ID", "all it spent, and from what")
        .example("spend-watch sessions ID go", "its window, in front")
        .argument("<session>", "a session's whole id")
        .exit(Exit::NotFound, "no session by that id has spent")
        .subcommand(
            reads("go", "bring its terminal forward")
                .example("spend-watch sessions ID go", "its window, in front")
                .exit(Exit::NotFound, "no such session is open"),
        )
}

/// `tokens`, whose one action replays its warning.
fn tokens_help() -> Help {
    reads("tokens", "the tokens' warnings")
        .example(
            "spend-watch tokens replay 2026-11-01 2026-11-08",
            "as it would have warned then",
        )
        .subcommand(
            reads("replay", "the tokens' warning again, over a span")
                .example(
                    "spend-watch tokens replay 2026-11-01 2026-11-08",
                    "as it would have warned then",
                )
                .argument("<from>", "the first day")
                .argument("<to>", "the last"),
        )
}

/// A subcommand reading the config, as most do.
fn reads(name: &str, about: &str) -> Help {
    Help::new(name, about)
        .option("--config <file>", "the config; the module's by default")
        .exit(Exit::NotFound, "there's no config")
        .exit(Exit::Config, "the config can't be read")
}
