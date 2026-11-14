//! The command line, in the terminal kit's shape (docs/commands.md).
//! Everything but `run` and `serve` reads the history the daemon keeps, so
//! it answers while the daemon runs, and while it doesn't. A bare run is
//! the week at a glance, the read asked most.

use crate::config::Config;
use crate::findings::Unmarked;
use crate::go::Unbrought;
use crate::history::History;
use crate::picture::Picture;
use crate::report::{self, Cut, Period, Report};
use serde_json::{Value, json};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use terminal_kit::{Context, Exit, Failure, Format, Help, flags};

/// What follows the week at a glance, for a person, to say what else to ask.
const MORE: &str = "Ask for more: spend-watch report last week by quest; report today by model; \
                    report waste; report caps; report outliers; report forecast.";

/// The periods a report covers, in its own words.
const PERIODS: [&str; 7] = [
    "today",
    "yesterday",
    "this week",
    "last week",
    "this month",
    "last month",
    "30 days",
];

/// The ways a report is cut.
const CUTS: [&str; 9] = [
    "quest", "model", "agent", "repo", "session", "tool", "day", "source", "budget",
];

/// Its own flags, beside the kit's.
const OPTIONS: [&str; 2] = ["--config", "--session"];

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
        .subcommand(
            reads("go", "bring a session's terminal forward")
                .example("spend-watch go ID", "its window, in front")
                .argument("<session>", "the session's id")
                .exit(Exit::NotFound, "no such session is open"),
        )
        .subcommand(
            Help::new("contract", "a feed's contract").subcommand(
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

/// The config the home module renders, found the way the module puts it.
fn default_config(context: &Context) -> PathBuf {
    let base = context
        .environment
        .var("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| {
            context
                .environment
                .var("HOME")
                .map(|home| PathBuf::from(home).join(".config"))
        })
        .unwrap_or_else(|| PathBuf::from("."));
    base.join("spend-watch/config.json")
}

/// What's left of the command line once the kit's flags and its own are
/// taken: the config, a session to name, and the words.
struct Asked {
    config: PathBuf,
    session: Option<String>,
    words: Vec<String>,
}

fn asked(context: &Context) -> Result<Asked, Failure> {
    let mut config = default_config(context);
    let mut session = None;
    let mut words = Vec::new();
    let mut args = context.args.iter();
    while let Some(arg) = args.next() {
        let (flag, attached) = match arg.split_once('=') {
            Some((flag, value)) if flag.starts_with("--") => (flag, Some(value.to_owned())),
            _ => (arg.as_str(), None),
        };
        let mut value = |wants: &str| {
            attached
                .clone()
                .or_else(|| args.next().cloned())
                .ok_or_else(|| {
                    Failure::new(Exit::Usage, format!("{flag} needs a value"))
                        .because(format!("it takes {wants}"))
                })
        };
        match flag {
            "--config" => config = PathBuf::from(value("a file")?),
            "--session" => session = Some(value("a session's id")?),
            // The kit reads these itself to say a failure in full.
            "--verbose" | "-v" => {}
            _ if flag.starts_with('-') && flag.len() > 1 => {
                return Err(flags::unknown("option", flag, &OPTIONS));
            }
            _ => words.push(arg.clone()),
        }
    }
    Ok(Asked {
        config,
        session,
        words,
    })
}

/// The command's body.
pub fn act(context: &Context, help: &Help) -> Result<(), Failure> {
    let Asked {
        config: path,
        session,
        words,
    } = asked(context)?;
    let words: Vec<&str> = words.iter().map(String::as_str).collect();
    if session.is_some() && words.first() != Some(&"budgets") {
        return Err(Failure::new(Exit::Usage, "--session is for budgets alone")
            .try_this("spend-watch budgets --session <id>"));
    }
    match words.as_slice() {
        [] => report_command(context, &config(&path)?, &[]),
        ["report", rest @ ..] => report_command(context, &config(&path)?, rest),
        ["budgets"] => budgets(context, &config(&path)?, session.as_deref()),
        ["findings"] => findings(context, &config(&path)?),
        ["findings", "expect", key] => expect(context, &config(&path)?, key),
        ["backtest"] => {
            let config = config(&path)?;
            let (history, picture) = picture(&config)?;
            let replayed =
                crate::backtest::backtest(&history, &picture, &config).map_err(unread)?;
            give_report(context, "spend-watch.backtest/1", &replayed, false)
        }
        ["tokens", "replay", from, to] => replay(context, &config(&path)?, from, to),
        ["refresh"] => refresh(context, &config(&path)?),
        ["go", session] => go(context, &config(&path)?, session),
        ["contract", "check", file] => contract(context, file),
        ["check-config", file] => check_config(context, file),
        ["show"] => show(context, &config(&path)?, false),
        ["show", "collection"] => show(context, &config(&path)?, true),
        ["import"] => import(context, &config(&path)?),
        ["run"] => {
            let config = config(&path)?;
            drop(held(&config)?);
            crate::daemon::run(&config).map_err(broke)
        }
        ["serve"] => crate::daemon::serve(&config(&path)?).map_err(broke),
        [word, ..] if help.command_names().contains(word) => Err(Failure::new(
            Exit::Usage,
            format!("{word} wasn't asked rightly"),
        )
        .try_this(format!("spend-watch {word} --help"))),
        [word, ..] => Err(flags::unknown("command", word, &help.command_names())),
    }
}

/// `findings`, and marking one expected.
fn findings_help() -> Help {
    reads("findings", "what holds now")
        .example("spend-watch findings", "each finding standing")
        .subcommand(
            reads("expect", "marks a finding as expected")
                .example("spend-watch findings expect KEY", "KEY stops notifying")
                .argument("<key>", "the finding's key, as findings gives it")
                .exit(Exit::NotFound, "no finding has that key"),
        )
}

/// `tokens`, whose one action replays its warning.
fn tokens_help() -> Help {
    reads("tokens", "the tokens' warnings").subcommand(
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

/// The config, read: missing and unreadable are each their own exit.
fn config(path: &Path) -> Result<Config, Failure> {
    if !path.exists() {
        return Err(
            Failure::new(Exit::NotFound, format!("no config at {}", path.display()))
                .because("the home module puts it there; --config <file> names another"),
        );
    }
    Config::read(path)
        .map_err(|why| Failure::new(Exit::Config, "the config can't be read").because(why))
}

/// The history couldn't be read.
fn unread(why: String) -> Failure {
    Failure::new(Exit::Failed, "the history couldn't be read").because(why)
}

/// Something the daemon does couldn't be done.
fn broke(why: String) -> Failure {
    Failure::new(Exit::Failed, "spend-watch couldn't carry on").because(why)
}

/// The answer couldn't be written: its reader went, or the disk is full.
fn wrote(written: std::io::Result<()>) -> Result<(), Failure> {
    written.map_err(|error| {
        Failure::new(Exit::Failed, "the answer couldn't be written").because(error.to_string())
    })
}

/// The history's lock, taken while no other spend-watch holds it.
fn held(config: &Config) -> Result<std::fs::File, Failure> {
    crate::daemon::lock(&config.state).map_err(|why| {
        Failure::new(Exit::TryAgain, "another spend-watch holds the history")
            .because(why)
            .try_this("systemctl --user status spend-watch")
    })
}

/// Facts as an answer under their schema.
fn answer(context: &Context, schema: &str, facts: impl serde::Serialize) -> Result<(), Failure> {
    let mut value = serde_json::to_value(facts).map_err(|error| {
        Failure::new(Exit::Bug, "the answer couldn't be put as JSON").because(error.to_string())
    })?;
    if let Some(fields) = value.as_object_mut() {
        fields.insert("schema".into(), schema.into());
    }
    wrote(context.answer(&value))
}

/// A form of spend-watch's own asked of a command that has none.
fn no_own_form(context: &Context, command: &str) -> Result<(), Failure> {
    match context.format() {
        Format::Own(form) => Err(Failure::new(
            Exit::Usage,
            format!("{command} has no {form} form"),
        )
        .because("only report and backtest are tables")
        .try_this(format!("spend-watch {command} --json"))),
        _ => Ok(()),
    }
}

fn picture(config: &Config) -> Result<(History, Picture), Failure> {
    let history = History::open(&config.history()).map_err(unread)?;
    let picture = Picture::read(&history, config, crate::time::now()).map_err(unread)?;
    Ok((history, picture))
}

/// A report in the form asked: laid out for a person, a table of
/// spend-watch's own, or its facts.
fn give_report(
    context: &Context,
    schema: &str,
    report: &Report,
    glance: bool,
) -> Result<(), Failure> {
    match context.format() {
        Format::Json | Format::Jsonl => answer(context, schema, report),
        Format::Own(form) if form == "csv" => {
            wrote(context.say(&report::render(report, report::Format::Csv)))
        }
        Format::Own(_) => wrote(context.say(&report::render(report, report::Format::Markdown))),
        Format::Text => {
            let mut out = report::render_text_in(report, context.room);
            if glance {
                out.push('\n');
                for line in terminal_kit::layout::paragraph(MORE, 0, context.room.for_prose()) {
                    let _ = writeln!(out, "{line}");
                }
            }
            wrote(context.say(&out))
        }
    }
}

/// `report`: the week at a glance, one focus, or a period cut one way.
fn report_command(context: &Context, config: &Config, rest: &[&str]) -> Result<(), Failure> {
    let (history, picture) = picture(config)?;
    let joined = rest.join(" ").to_lowercase();
    let focused = match joined.as_str() {
        "waste" => Some(report::waste(&picture)),
        "caps" => Some(report::caps(&picture)),
        "quests" => Some(report::quests(&picture, Period::Days30)),
        "outliers" => Some(report::outliers(
            &picture,
            config.detectors.false_discovery,
            config.detectors.floor,
        )),
        "forecast" => Some(report::forecast(&picture)),
        "" => Some(report::summary(
            &picture,
            &crate::findings::standing(&history).map_err(unread)?,
        )),
        _ => None,
    };
    if let Some(report) = focused {
        return give_report(context, "spend-watch.report/1", &report, joined.is_empty());
    }
    // "<period> [by <cut>]", or "by <cut>" for this week.
    let (period_words, cut_words) = match joined.split_once(" by ") {
        Some((period, cut)) => (period.trim().to_owned(), Some(cut.trim().to_owned())),
        None => match joined.strip_prefix("by ") {
            Some(cut) => (String::new(), Some(cut.trim().to_owned())),
            None => (joined.clone(), None),
        },
    };
    let period = if period_words.is_empty() {
        Period::ThisWeek
    } else {
        Period::parse(&period_words).ok_or_else(|| {
            Failure::new(Exit::Usage, format!("no period called {period_words}"))
                .because(format!("the periods are {}", flags::and(&PERIODS)))
                .try_this("spend-watch report last week")
        })?
    };
    let cut = match cut_words {
        Some(words) => Cut::parse(&words).ok_or_else(|| flags::unknown("cut", &words, &CUTS))?,
        None => Cut::Quest,
    };
    let report = report::cut(&picture, period, cut, 25);
    give_report(context, "spend-watch.report/1", &report, false)
}

/// How old the daemon's last word on the budgets may be and still be how
/// they stand: past its idle wait between passes, by a minute.
const WRITTEN_LASTS: f64 = 360.0;

/// The budgets as the daemon's last pass wrote them, while that's recent
/// enough to be how they stand. Every pi session asks for them, all at once
/// after an update reloads them, so the daemon's answer spares each a pass
/// of its own over the whole history.
fn written_budgets(config: &Config, now: f64) -> Option<Value> {
    let text = std::fs::read_to_string(config.runtime().ok()?.join("budgets.json")).ok()?;
    let written: Value = serde_json::from_str(&text).ok()?;
    let at = crate::time::parse(written.get("at")?.as_str()?)?;
    let current = written.get("v")?.as_u64()? == u64::from(crate::standing::VERSION);
    (current && (0.0..=WRITTEN_LASTS).contains(&(now - at))).then_some(written)
}

/// `budgets`: how each budget stands, as JSON for the harness, or a line
/// each for a person. Without a session, the JSON is the daemon's last
/// pass's while it's recent.
fn budgets(context: &Context, config: &Config, session: Option<&str>) -> Result<(), Failure> {
    no_own_form(context, "budgets")?;
    let schema = "spend-watch.budgets/1";
    if session.is_none()
        && context.format().is_json()
        && let Some(written) = written_budgets(config, crate::time::now())
    {
        return answer(context, schema, written);
    }
    let (_, picture) = picture(config)?;
    let standing = crate::standing::standing(&picture, config, session);
    if context.format().is_json() {
        return answer(context, schema, &standing);
    }
    let total = &standing.total.periods["this month"];
    let mut out = format!("{} this month, {}.\n\n", total.shown, standing.total.words);
    // Writing to a String can't fail, so what `writeln!` returns is let go.
    for budget in &standing.budgets {
        let default = if budget.default { ", the default" } else { "" };
        _ = writeln!(out, "  {}{default}: {}", budget.label, budget.standing);
    }
    if let Some(session) = &standing.session {
        _ = writeln!(
            out,
            "\nSession {} draws on {}: {} in all.",
            session.id,
            session.budget.as_deref().unwrap_or("nothing yet"),
            session.spent.shown
        );
    }
    wrote(context.say(&out))
}

/// `findings`: every standing finding.
fn findings(context: &Context, config: &Config) -> Result<(), Failure> {
    no_own_form(context, "findings")?;
    let history = History::open(&config.history()).map_err(unread)?;
    let findings = crate::findings::standing(&history).map_err(unread)?;
    if context.format().is_json() {
        return answer(
            context,
            "spend-watch.findings/1",
            json!({"findings": findings}),
        );
    }
    if findings.is_empty() {
        return wrote(context.say("Nothing found.\n"));
    }
    let said = findings
        .iter()
        .map(|finding| {
            format!(
                "{}{}: {}\n  {}\n  key {}\n",
                finding.severity.name(),
                if finding.marked { ", expected" } else { "" },
                finding.title,
                finding.detail,
                finding.key
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    wrote(context.say(&said))
}

/// `findings expect <key>`: marks the finding whose key is it, or starts
/// so, expected.
fn expect(context: &Context, config: &Config, key: &str) -> Result<(), Failure> {
    no_own_form(context, "findings")?;
    let history = History::open(&config.history()).map_err(unread)?;
    let marked =
        crate::findings::mark(&history, key, crate::time::now()).map_err(
            |unmarked| match unmarked {
                Unmarked::Unheld(_) => Failure::new(Exit::NotFound, unmarked.to_string())
                    .try_this("spend-watch findings"),
                Unmarked::Several(..) => Failure::new(Exit::Usage, unmarked.to_string())
                    .because("a key's start has to name one")
                    .try_this("spend-watch findings"),
                Unmarked::Broke(why) => {
                    Failure::new(Exit::Failed, "the mark couldn't be kept").because(why)
                }
            },
        )?;
    // The shell shows it at the next pass; ask for one if the daemon runs.
    let _ = crate::daemon::ask(&config.socket(), "refresh");
    if context.format().is_json() {
        return answer(context, "spend-watch.expected/1", json!({"key": marked}));
    }
    wrote(context.say(&format!("{marked} is expected now.\n")))
}

/// `tokens replay <from> <to>`: when the tokens' warning would have said
/// what, every five minutes between two instants.
fn replay(context: &Context, config: &Config, from: &str, to: &str) -> Result<(), Failure> {
    no_own_form(context, "tokens")?;
    let read = |text: &str| {
        crate::time::parse(text).ok_or_else(|| {
            Failure::new(Exit::Usage, format!("{text} isn't an instant"))
                .because("replay takes two RFC 3339 instants")
                .try_this("spend-watch tokens replay 2026-10-31T00:00:00Z 2026-10-31T12:00:00Z")
        })
    };
    let (from, to) = (read(from)?, read(to)?);
    let zone = config.zone().map_err(|why| {
        Failure::new(Exit::Config, "the config's zone can't be read").because(why)
    })?;
    let history = History::open(&config.history()).map_err(unread)?;
    let budgets = crate::budgets::Budgets::of(config);
    let changes = crate::tokens::replay(&history, &budgets, &zone, from, to).map_err(unread)?;
    if context.format().is_json() {
        let changes: Vec<Value> = changes
            .iter()
            .map(|said| {
                json!({
                    "at": crate::time::rfc3339(said.at),
                    "level": said.level,
                    "hint": said.hint,
                    "context": said.context,
                })
            })
            .collect();
        return answer(
            context,
            "spend-watch.tokens-replay/1",
            json!({"changes": changes}),
        );
    }
    let mut out = String::new();
    for said in &changes {
        let clock = zone.clock(said.at, to);
        // Writing to a String can't fail, so what `writeln!` returns is let go.
        _ = match (&said.level, &said.hint, &said.context) {
            (Some(level), Some(hint), Some(context)) => {
                let level = serde_json::to_value(level).unwrap_or_default();
                let level = level.as_str().unwrap_or_default();
                writeln!(out, "{clock} {level} {hint}: {context}")
            }
            _ => writeln!(out, "{clock} nothing to say"),
        };
    }
    wrote(context.say(&out))
}

/// `refresh`: asks the daemon for a pass now, and says what it answered.
fn refresh(context: &Context, config: &Config) -> Result<(), Failure> {
    no_own_form(context, "refresh")?;
    let said = crate::daemon::ask(&config.socket(), "refresh").map_err(|why| {
        Failure::new(Exit::Unavailable, "the daemon didn't answer")
            .because(why)
            .try_this("systemctl --user start spend-watch")
    })?;
    if context.format().is_json() {
        return answer(context, "spend-watch.refresh/1", json!({"answered": said}));
    }
    wrote(context.say(&format!("{said}\n")))
}

/// `go <session>`: brings a session's terminal forward.
fn go(context: &Context, config: &Config, session: &str) -> Result<(), Failure> {
    no_own_form(context, "go")?;
    crate::go::go(&config.registry, session).map_err(|unbrought| match unbrought {
        Unbrought::Gone(why) => {
            Failure::new(Exit::NotFound, format!("no session {session}")).because(why)
        }
        Unbrought::Several(..) => Failure::new(Exit::Usage, unbrought.to_string())
            .because("an id's start has to name one"),
        Unbrought::Broke(why) => Failure::new(
            Exit::Failed,
            format!("session {session} couldn't be brought forward"),
        )
        .because(why),
    })?;
    if context.format().is_json() {
        return answer(context, "spend-watch.go/1", json!({"session": session}));
    }
    Ok(())
}

/// `contract check <file>`: every line of a feed's output against the
/// contract, saying which line fails and why.
fn contract(context: &Context, file: &str) -> Result<(), Failure> {
    no_own_form(context, "contract")?;
    let text = std::fs::read_to_string(file).map_err(|error| {
        let exit = if error.kind() == std::io::ErrorKind::NotFound {
            Exit::NotFound
        } else {
            Exit::Failed
        };
        Failure::new(exit, format!("{file} can't be read")).because(error.to_string())
    })?;
    let mut kinds: std::collections::BTreeMap<&'static str, usize> =
        std::collections::BTreeMap::new();
    let mut lines = 0;
    for (number, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let record = crate::feed::parse(line).map_err(|error| {
            Failure::new(
                Exit::BadInput,
                format!("{file}:{} breaks the contract", number + 1),
            )
            .because(error)
        })?;
        let kind = match record {
            crate::feed::Record::Billed(_) => "billed",
            crate::feed::Record::Settled(_) => "settled",
            crate::feed::Record::Cap(_) => "cap",
            crate::feed::Record::Action(_) => "action",
            crate::feed::Record::Health(_) => "health",
            crate::feed::Record::Read(_) => "read",
            crate::feed::Record::Said(_) => "said",
        };
        *kinds.entry(kind).or_default() += 1;
        lines += 1;
    }
    if context.format().is_json() {
        return answer(
            context,
            "spend-watch.contract/1",
            json!({"file": file, "lines": lines, "kinds": kinds}),
        );
    }
    let counted: Vec<String> = kinds
        .iter()
        .map(|(kind, count)| format!("{count} {kind}"))
        .collect();
    wrote(context.say(&format!(
        "{file}: {lines} lines keep the contract ({}).\n",
        counted.join(", ")
    )))
}

/// `check-config <file>`: whether a config can run, the file asked of
/// being the input rather than spend-watch's own config.
fn check_config(context: &Context, file: &str) -> Result<(), Failure> {
    no_own_form(context, "check-config")?;
    let path = Path::new(file);
    if !path.exists() {
        return Err(Failure::new(Exit::NotFound, format!("no config at {file}")));
    }
    Config::read(path)
        .map_err(|why| Failure::new(Exit::BadInput, format!("{file} can't run")).because(why))?;
    if context.format().is_json() {
        return answer(context, "spend-watch.check-config/1", json!({"file": file}));
    }
    wrote(context.say(&format!("{file} can run.\n")))
}

/// `show` and `show collection`: the status or the collection the daemon
/// would write now, with how long each step took on standard error. Both
/// forms are the file's JSON, since that's what it shows.
fn show(context: &Context, config: &Config, collection: bool) -> Result<(), Failure> {
    no_own_form(context, "show")?;
    let history = History::open(&config.history()).map_err(unread)?;
    let now = crate::time::now();
    let began = std::time::Instant::now();
    let picture = Picture::read(&history, config, now).map_err(unread)?;
    let read = began.elapsed();
    let mut raised = crate::detect::all(&picture, &history, config).map_err(unread)?;
    crate::findings::sort(&mut raised);
    let detected = began.elapsed();
    let status = crate::status::status(&picture, &raised);
    let drawn = crate::collection::collection(&picture, &raised, config);
    let written = began.elapsed();
    // Each later reading of the clock is at least the one before it, so
    // the saturation never comes into it.
    eprintln!(
        "read {:.0} ms, detected {:.0} ms, drawn {:.0} ms",
        read.as_secs_f64() * 1000.0,
        detected.saturating_sub(read).as_secs_f64() * 1000.0,
        written.saturating_sub(detected).as_secs_f64() * 1000.0
    );
    let (schema, shown) = if collection {
        ("spend-watch.collection/1", serde_json::to_value(&drawn))
    } else {
        ("spend-watch.status/1", serde_json::to_value(&status))
    };
    let shown = shown.map_err(|error| {
        Failure::new(Exit::Bug, "what's shown couldn't be put as JSON").because(error.to_string())
    })?;
    if context.format().is_json() {
        return answer(context, schema, shown);
    }
    wrote(context.say(&format!("{shown:#}\n")))
}

/// `import`: reads everything new into the history while the daemon isn't
/// running, and says how much it holds.
fn import(context: &Context, config: &Config) -> Result<(), Failure> {
    no_own_form(context, "import")?;
    let _lock = held(config)?;
    let history = History::open(&config.history()).map_err(unread)?;
    let began = std::time::Instant::now();
    crate::daemon::harness(&history, config);
    crate::daemon::catch_up(&history, config).map_err(unread)?;
    let turns: i64 = history
        .db
        .query_row("SELECT COUNT(*) FROM turns", [], |row| row.get(0))
        .map_err(|error| unread(error.to_string()))?;
    let seconds = began.elapsed().as_secs_f64();
    if context.format().is_json() {
        return answer(
            context,
            "spend-watch.import/1",
            json!({"turns": turns, "read_seconds": seconds}),
        );
    }
    wrote(context.say(&format!("{turns} turns held, read in {seconds:.1} s.\n")))
}

#[cfg(test)]
mod tests;
