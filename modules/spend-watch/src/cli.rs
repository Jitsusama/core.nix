//! The command line. Everything but `run` reads the history the daemon
//! keeps, so it answers while the daemon runs, and while it doesn't.

use crate::config::Config;
use crate::history::History;
use crate::picture::Picture;
use crate::report::{self, Cut, Format, Period};
use std::path::PathBuf;

const USAGE: &str = "\
spend-watch: what AI use costs

  spend-watch report [period] [by <cut>] [--format text|markdown|json|csv]
      period: today, yesterday, this week, last week, this month,
              last month, 30 days; with no cut, the week at a glance
      cut: quest, model, agent, repo, session, tool, day, source
  spend-watch report waste|caps|quests|outliers|forecast
  spend-watch findings              what holds now
  spend-watch findings expect <key> mark one expected
  spend-watch backtest              the forecast and the notices, replayed
  spend-watch refresh               ask the daemon for a pass now
  spend-watch go <session>          bring a session's terminal forward
  spend-watch contract check <file> check a feed's lines against the contract
  spend-watch check-config <file>   whether a config can run
  spend-watch show                  the status the shell would draw now, and
                                    what working it out cost
  spend-watch import                read what's new from pi and the harness once
  spend-watch run                   the daemon
  spend-watch version

Every command takes --config <file>; the default is the one the module
installs.";

/// The config the home module renders, found the way the module puts it.
fn default_config() -> PathBuf {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
        .unwrap_or_else(|| PathBuf::from("."));
    base.join("spend-watch/config.json")
}

struct Args {
    config: PathBuf,
    format: Format,
    words: Vec<String>,
}

fn parse(args: Vec<String>) -> Result<Args, String> {
    let mut config = default_config();
    let mut format = Format::Text;
    let mut words = Vec::new();
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--config" => config = PathBuf::from(args.next().ok_or("--config wants a file")?),
            "--format" => {
                let name = args
                    .next()
                    .ok_or("--format wants text, markdown, json or csv")?;
                format = Format::parse(&name)
                    .ok_or(format!("no format {name}: text, markdown, json or csv"))?;
            }
            "--json" => format = Format::Json,
            "--help" | "-h" => words.insert(0, "help".into()),
            _ => words.push(arg),
        }
    }
    Ok(Args {
        config,
        format,
        words,
    })
}

pub fn main(args: Vec<String>) -> i32 {
    match command(args) {
        Ok(out) => {
            print!("{out}");
            0
        }
        Err(error) => {
            eprintln!("spend-watch: {error}");
            1
        }
    }
}

fn picture(config: &Config) -> Result<(History, Picture), String> {
    let history = History::open(&config.history())?;
    let picture = Picture::read(&history, config, crate::time::now())?;
    Ok((history, picture))
}

fn command(args: Vec<String>) -> Result<String, String> {
    let Args {
        config: path,
        format,
        words,
    } = parse(args)?;
    let words: Vec<&str> = words.iter().map(String::as_str).collect();
    match words.as_slice() {
        [] | ["help"] | ["help", ..] => Ok(format!("{USAGE}\n")),
        ["version"] => Ok(format!("spend-watch {}\n", env!("CARGO_PKG_VERSION"))),
        ["contract", "check", file] => contract(file),
        ["check-config", file] => {
            Config::read(std::path::Path::new(file)).map(|_| format!("{file} can run.\n"))
        }
        ["run"] => {
            let config = Config::read(&path)?;
            crate::daemon::run(config).map(|()| String::new())
        }
        ["import"] => {
            let config = Config::read(&path)?;
            let _lock = crate::daemon::lock(&config.state)
                .map_err(|_| "the daemon is running, and reads what's new itself".to_owned())?;
            let history = History::open(&config.history())?;
            let began = std::time::Instant::now();
            crate::daemon::harness(&history, &config);
            crate::daemon::catch_up(&history, &config)?;
            let turns: i64 = history
                .db
                .query_row("SELECT COUNT(*) FROM turns", [], |row| row.get(0))
                .map_err(|error| error.to_string())?;
            Ok(format!(
                "{turns} turns held, read in {:.1} s.\n",
                began.elapsed().as_secs_f64()
            ))
        }
        ["show"] | ["show", "collection"] => {
            let config = Config::read(&path)?;
            let history = History::open(&config.history())?;
            let now = crate::time::now();
            let began = std::time::Instant::now();
            let picture = Picture::read(&history, &config, now)?;
            let read = began.elapsed();
            let mut raised = crate::detect::all(&picture, &history, &config)?;
            crate::findings::sort(&mut raised);
            let detected = began.elapsed();
            let status = crate::status::status(&picture, &raised);
            let collection = crate::collection::collection(&picture, &raised, &config);
            let written = began.elapsed();
            eprintln!(
                "read {:.0} ms, detected {:.0} ms, drawn {:.0} ms",
                read.as_secs_f64() * 1000.0,
                (detected - read).as_secs_f64() * 1000.0,
                (written - detected).as_secs_f64() * 1000.0
            );
            let text = if words.len() == 2 {
                serde_json::to_string_pretty(&collection)
            } else {
                serde_json::to_string_pretty(&status)
            };
            text.map(|text| text + "\n")
                .map_err(|error| error.to_string())
        }
        ["refresh"] => {
            let config = Config::read(&path)?;
            crate::daemon::ask(&config.socket(), "refresh").map(|said| format!("{said}\n"))
        }
        ["go", session] => {
            let config = Config::read(&path)?;
            crate::go::go(&config.registry, session).map(|()| String::new())
        }
        ["findings"] => {
            let config = Config::read(&path)?;
            let history = History::open(&config.history())?;
            let findings = crate::findings::standing(&history)?;
            if format == Format::Json {
                return serde_json::to_string_pretty(&findings)
                    .map(|text| text + "\n")
                    .map_err(|e| e.to_string());
            }
            if findings.is_empty() {
                return Ok("Nothing found.\n".into());
            }
            Ok(findings
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
                .join("\n"))
        }
        ["findings", "expect", key] => {
            let config = Config::read(&path)?;
            let history = History::open(&config.history())?;
            let marked = crate::findings::mark(&history, key, crate::time::now())?;
            // The shell shows it at the next pass; ask for one if the daemon runs.
            let _ = crate::daemon::ask(&config.socket(), "refresh");
            Ok(format!("{marked} is expected now.\n"))
        }
        ["backtest"] => {
            let config = Config::read(&path)?;
            let (history, picture) = picture(&config)?;
            Ok(report::render(
                &crate::backtest::backtest(&history, &picture, &config)?,
                format,
            ))
        }
        ["report", rest @ ..] => {
            let config = Config::read(&path)?;
            report_command(&config, rest, format)
        }
        _ => Err(format!("not a command: {}\n\n{USAGE}", words.join(" "))),
    }
}

fn report_command(config: &Config, rest: &[&str], format: Format) -> Result<String, String> {
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
            &crate::findings::standing(&history)?,
        )),
        _ => None,
    };
    if let Some(report) = focused {
        let mut out = report::render(&report, format);
        if joined.is_empty() && format == Format::Text {
            out.push_str(
                "\nAsk for more: spend-watch report last week by quest; report today by model;\n\
                 report waste; report caps; report outliers; report forecast.\n",
            );
        }
        return Ok(out);
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
        Period::parse(&period_words).ok_or(format!(
            "no period {period_words}: today, yesterday, this week, last week, this month, \
             last month or 30 days"
        ))?
    };
    let cut = match cut_words {
        Some(words) => Cut::parse(&words).ok_or(format!(
            "no cut {words}: quest, model, agent, repo, session, tool, day or source"
        ))?,
        None => Cut::Quest,
    };
    Ok(report::render(
        &report::cut(&picture, period, cut, 25),
        format,
    ))
}

/// Checks every line of a feed's output against the contract, saying
/// which line fails and why.
fn contract(file: &str) -> Result<String, String> {
    let text = std::fs::read_to_string(file).map_err(|error| format!("reading {file}: {error}"))?;
    let mut kinds: std::collections::BTreeMap<&'static str, usize> =
        std::collections::BTreeMap::new();
    let mut lines = 0;
    for (number, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let record =
            crate::feed::parse(line).map_err(|error| format!("{file}:{}: {error}", number + 1))?;
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
    let counted: Vec<String> = kinds
        .iter()
        .map(|(kind, count)| format!("{count} {kind}"))
        .collect();
    Ok(format!(
        "{file}: {lines} lines keep the contract ({}).\n",
        counted.join(", ")
    ))
}
