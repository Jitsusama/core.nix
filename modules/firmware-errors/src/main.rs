//! Says why each kept boot's last one ended, from what the firmware handed
//! it, for an administrator or an agent, without root.
//!
//! At every boot the keeping unit (../nixos.nix, ../keep.sh) copies the
//! firmware's boot error region and record table (BERT) and its platform
//! health table (PHAT) into /var/lib/firmware-errors/<boot id>/, readable by
//! the wheel group, with when the boot began. This reads them there: the
//! health table's reset source and reason, with Intel's own reset cause
//! beside them (src/phat.rs), and the Crash Log an Intel firmware leaves in
//! the region when it resets the machine for an error, by Intel's own
//! decoder (src/crash.rs). Each boot's tables say how the boot before it
//! ended, so a reset shows on the boot that came after it. Only the boot
//! running now has its watchdog's word, which the driver keeps for as long
//! as it runs. `FIRMWARE_ERRORS_STATE` names another folder to read, and
//! `FIRMWARE_ERRORS_CPUINFO` another processor's description, as the
//! checks' stand-in state gives them, so what it says doesn't hang on the
//! machine the checks run on.
//!
//!   firmware-errors          every kept boot, the newest first, and how the
//!                            boot before each ended
//!   firmware-errors BOOT     one boot, by its id or the start of it, in full
//!
//! It takes the shape every command of ours does (docs/commands.md).

mod boots;
mod crash;
#[cfg(test)]
mod keeping;
mod phat;

use boots::{Kept, Weight};
use crash::Crash;
use phat::Reset;
use serde_json::{Value, json};
use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use terminal_kit::layout;
use terminal_kit::{Context, Exit, Failure, Help, Row};

/// Where the keeping unit keeps each boot's tables.
const STATE: &str = "/var/lib/firmware-errors";
/// The variable naming another folder to read in its place.
const STATE_VARIABLE: &str = "FIRMWARE_ERRORS_STATE";
/// The boot running now, as the kernel names it.
const BOOT_ID: &str = "/proc/sys/kernel/random/boot_id";
/// What the processor is, which says whether Intel's cause can be named.
const CPUINFO: &str = "/proc/cpuinfo";
/// The variable naming another processor's description to read.
const CPUINFO_VARIABLE: &str = "FIRMWARE_ERRORS_CPUINFO";
/// The first watchdog, whose driver says whether it reset the machine.
const WATCHDOG: &str = "/sys/class/watchdog/watchdog0";
/// The watchdog's status bit saying it reset the machine, `WDIOF_CARDRESET`.
const CARD_RESET: u32 = 0x20;

/// What the watchdog says of the boot running now.
struct Watchdog {
    identity: String,
    reset_it: bool,
}

/// What every boot's answer is read against: the boot running now, its
/// watchdog's word, and whether Intel's cause can be named here.
struct Here {
    current: String,
    watchdog: Option<Watchdog>,
    named: bool,
}

impl Here {
    fn read() -> Self {
        Self {
            current: fs::read_to_string(BOOT_ID)
                .ok()
                .and_then(|id| boots::plain_id(id.trim()))
                .unwrap_or_default(),
            watchdog: watchdog(),
            named: fs::read_to_string(env::var_os(CPUINFO_VARIABLE).unwrap_or(CPUINFO.into()))
                .is_ok_and(|info| phat::names_its_causes(&info)),
        }
    }

    fn is_current(&self, boot: &Kept) -> bool {
        boot.boot == self.current
    }

    /// The watchdog's word, for the boot running now alone.
    fn watchdog_of(&self, boot: &Kept) -> Option<&Watchdog> {
        self.watchdog.as_ref().filter(|_| self.is_current(boot))
    }

    fn verdict(&self, boot: &Kept) -> boots::End {
        let reset = self
            .watchdog_of(boot)
            .is_some_and(|watchdog| watchdog.reset_it);
        boot.verdict(self.named, reset)
    }
}

/// The watchdog's word on the boot running now; none where there's no
/// watchdog or it doesn't say.
fn watchdog() -> Option<Watchdog> {
    let read = |name: &str| fs::read_to_string(Path::new(WATCHDOG).join(name)).ok();
    let status = read("bootstatus")?.trim().parse::<u32>().ok()?;
    Some(Watchdog {
        identity: read("identity").map_or_else(String::new, |words| words.trim().to_owned()),
        reset_it: status & CARD_RESET != 0,
    })
}

/// The folder the kept boots are in.
fn state() -> PathBuf {
    env::var_os(STATE_VARIABLE)
        .filter(|state| !state.is_empty())
        .map_or_else(|| PathBuf::from(STATE), PathBuf::from)
}

/// A moment as the machine's clock reads it, to the minute.
fn local(seconds: i64) -> String {
    terminal_kit::local::at(seconds).map_or_else(
        || seconds.to_string(),
        |at| format!("{} {:02}:{:02}", at.date(), at.hour, at.minute),
    )
}

fn weight_name(weight: Weight) -> &'static str {
    match weight {
        Weight::Reset => "reset",
        Weight::Unasked => "unasked",
        Weight::Plain => "plain",
    }
}

/// A boot as its answer gives it: facts and names a program matches,
/// never the sentences the text form says them in.
fn boot_json(boot: &Kept, here: &Here) -> Value {
    let end = here.verdict(boot);
    let reset = boot.reset.as_ref().map(|reset| {
        json!({
            "source": reset.source,
            "source_name": reset.source_name(),
            "reason": reset.reason,
            "reason_name": reset.reason_name(),
            "intel": reset.intel.map(|words| json!({
                "words": words,
                "causes": reset.global_causes(here.named),
            })),
        })
    });
    let crash_log = boot.crash.as_ref().map(|crash| {
        let (state, tags) = match crash {
            Crash::Tags(tags) => ("kept", tags.as_slice()),
            Crash::None => ("none", [].as_slice()),
            Crash::Other => ("other", [].as_slice()),
            Crash::Unreadable(_) => ("unreadable", [].as_slice()),
        };
        json!({"state": state, "tags": tags})
    });
    let watchdog = here
        .watchdog_of(boot)
        .map(|watchdog| json!({"identity": watchdog.identity, "reset_it": watchdog.reset_it}));
    let unread: Vec<&str> = boot.unread.iter().map(|(name, _)| *name).collect();
    json!({
        "boot": boot.boot,
        "began": boot.began.map(terminal_kit::time::rfc3339),
        "current": here.is_current(boot),
        "end": end.name(),
        "weight": weight_name(end.weight()),
        // Whether the health table was kept and read, kept but unreadable, or
        // not kept, so a null reset never reads as a table the keeping lost.
        "health_table": if boot.health {
            "read"
        } else if boot.unread.iter().any(|(name, _)| *name == boots::PHAT_TABLE) {
            "unreadable"
        } else {
            "not-kept"
        },
        "reset": reset,
        "crash_log": crash_log,
        "watchdog": watchdog,
        "unread": unread,
    })
}

/// Every kept boot as a row: when it began, how the boot before it ended,
/// and its id's start.
fn rows(kept: &[Kept], here: &Here) -> Vec<Row> {
    kept.iter()
        .map(|boot| {
            let end = here.verdict(boot);
            let name = boot
                .began
                .map_or_else(|| "began at no known time".to_owned(), local);
            let row = Row::named(name)
                .holding(end.words())
                .side(if here.is_current(boot) {
                    format!("{} now", &boot.boot[..8])
                } else {
                    boot.boot[..8].to_owned()
                });
            match end.weight() {
                Weight::Reset => row.marked("failed", "alert"),
                Weight::Unasked => row.marked("caution", "warning"),
                Weight::Plain => row,
            }
        })
        .collect()
}

/// A number the specification doesn't name, said as one.
fn unnamed(value: u8) -> String {
    format!("{value:#04x}, which the specification doesn't name")
}

fn source_fact(reset: &Reset) -> String {
    reset
        .source_words()
        .map_or_else(|| unnamed(reset.source), str::to_owned)
}

fn reason_fact(reset: &Reset) -> String {
    match reset.reason_words() {
        Some(words) => words.to_owned(),
        None if reset.reason == 0 => "not said".to_owned(),
        None => unnamed(reset.reason),
    }
}

fn crash_fact(crash: &Crash) -> String {
    match crash {
        Crash::Tags(tags) if tags.is_empty() => "kept, naming nothing the decoder knows".to_owned(),
        Crash::Tags(tags) => tags.join(", "),
        Crash::None => "none".to_owned(),
        Crash::Other => "none, but an error of another kind".to_owned(),
        Crash::Unreadable(why) => format!("unreadable: {why}"),
    }
}

/// One boot's facts, as its text answer gives them; a fact with no value
/// wasn't kept. A health table kept that names no reset, or no word of
/// Intel's, says so, so it never reads as a table the keeping lost.
fn facts(boot: &Kept, here: &Here) -> Vec<(String, Option<String>)> {
    let reset = boot.reset.as_ref();
    let intel = reset.and_then(|reset| reset.intel);
    // A table kept that couldn't be read is said so, beside the row saying
    // why, never as one not kept.
    let unreadable = |table: &str| {
        boot.unread
            .iter()
            .any(|(name, _)| *name == table)
            .then(|| "unreadable: see Couldn't read".to_owned())
    };
    let in_the_table = |fact: Option<String>, none: &str| {
        fact.or_else(|| unreadable(boots::PHAT_TABLE))
            .or_else(|| boot.health.then(|| none.to_owned()))
    };
    let no_reset = "none: the health table kept names no reset it can read";
    let no_intel = if reset.is_some() {
        "none in the health table"
    } else {
        no_reset
    };
    let mut facts = vec![
        ("Boot".to_owned(), Some(boot.boot.clone())),
        ("Began".to_owned(), boot.began.map(local)),
        (
            "The boot before".to_owned(),
            Some(here.verdict(boot).words()),
        ),
        (
            "Reset source".to_owned(),
            in_the_table(reset.map(source_fact), no_reset),
        ),
        (
            "Reset reason".to_owned(),
            in_the_table(reset.map(reason_fact), no_reset),
        ),
        (
            "Intel's reset cause".to_owned(),
            in_the_table(
                intel.zip(reset).map(|(words, reset)| {
                    let causes = reset.global_causes(here.named);
                    if causes.is_empty() {
                        format!("0x{:08x}", words[0])
                    } else {
                        format!("0x{:08x}, {}", words[0], causes.join(", "))
                    }
                }),
                no_intel,
            ),
        ),
        (
            "Intel's other words".to_owned(),
            in_the_table(
                intel.map(|words| {
                    format!("0x{:08x} 0x{:08x} 0x{:08x}", words[1], words[2], words[3])
                }),
                no_intel,
            ),
        ),
        (
            "Crash Log".to_owned(),
            boot.crash
                .as_ref()
                .map(crash_fact)
                .or_else(|| unreadable(boots::BERT_REGION)),
        ),
    ];
    if !boot.unread.is_empty() {
        let unread: Vec<String> = boot
            .unread
            .iter()
            .map(|(name, why)| format!("{name}: {why}"))
            .collect();
        facts.push(("Couldn't read".to_owned(), Some(unread.join("; "))));
    }
    if here.is_current(boot) {
        facts.push((
            "Watchdog".to_owned(),
            // Nothing keeps a watchdog's word: it's there to read, or not.
            Some(here.watchdog.as_ref().map_or_else(
                || "none, or it doesn't say".to_owned(),
                |watchdog| {
                    let did = if watchdog.reset_it {
                        "reset it"
                    } else {
                        "didn't reset it"
                    };
                    format!("{} {did}", watchdog.identity).trim().to_owned()
                },
            )),
        ));
    }
    facts
}

/// What it says of itself, set as help and read by the check.
fn help() -> Help {
    Help::new(
        "firmware-errors",
        "says why each kept boot's last one ended, from what the firmware handed it",
    )
    .argument(
        "BOOT",
        "a boot's id, or the start of it, as journalctl --list-boots gives it",
    )
    .example(
        "firmware-errors",
        "every kept boot, and how the boot before each ended",
    )
    .example(
        "firmware-errors 6bccd85d",
        "one boot in full: the reset's reason and Crash Log",
    )
    .example(
        "firmware-errors --format json",
        "every kept boot, for a program",
    )
    .exit(
        Exit::Failed,
        "the kept tables' folder couldn't be read, or the answer couldn't be said",
    )
    .exit(Exit::NotFound, "no kept boot has that id")
    .exit(
        Exit::Usage,
        "it isn't a boot id, it names more than one kept boot, or it was given two",
    )
    .exit(
        Exit::Refused,
        "only the wheel group may read the kept tables",
    )
}

/// Why the kept tables couldn't be read, and what to try.
fn unread(state: &Path, error: &io::Error) -> Failure {
    if error.kind() == io::ErrorKind::PermissionDenied {
        // Never run0: what it parses the firmware wrote, so it runs as the
        // account that asks, never as root. The try line lists the
        // account's groups, which shows whether wheel is among them.
        return Failure::new(Exit::Refused, "it may not read the kept tables")
            .because(format!(
                "{} is the wheel group's to read: {error}",
                state.display()
            ))
            .try_this("id -nG");
    }
    let state = state.to_string_lossy();
    Failure::new(Exit::Failed, "it couldn't read the kept tables")
        .because(error.to_string())
        .try_this(format!(
            "ls -l {}",
            terminal_kit::failure::shell_word(&state)
        ))
}

fn said(result: io::Result<()>) -> Result<(), Failure> {
    result.map_err(|error| {
        Failure::new(Exit::Failed, "the answer couldn't be said").because(error.to_string())
    })
}

/// Every kept boot.
fn list(context: &Context, kept: &[Kept], here: &Here) -> Result<(), Failure> {
    if context.format().is_json() {
        let boots: Vec<Value> = kept.iter().map(|boot| boot_json(boot, here)).collect();
        return said(context.answer(&json!({"schema": "firmware-errors.boots/1", "boots": boots})));
    }
    if kept.is_empty() {
        return said(context.say(
            "No boot's tables are kept: the firmware gives none, or none was found to keep.\n",
        ));
    }
    let rows = rows(kept, here);
    said(context.say(&layout::joined(vec![layout::list(
        &rows,
        &context.out,
        context.room,
    )])))
}

/// The one kept boot an id names, or why there isn't one.
fn named<'a>(kept: &'a [Kept], id: &str) -> Result<&'a Kept, Failure> {
    let Some(plain) = boots::plain_id(id) else {
        return Err(Failure::new(Exit::Usage, format!("{id:?} isn't a boot id"))
            .because(format!(
                "a boot id is hex digits, with hyphens or without but never opening with \
                 one, and at least its first {}",
                boots::SHORTEST_ID
            ))
            .try_this("firmware-errors"));
    };
    match boots::matching(kept, &plain).as_slice() {
        [boot] => Ok(*boot),
        [] => Err(
            Failure::new(Exit::NotFound, format!("no kept boot has the id {id}"))
                .because("a boot's tables are kept 90 days, and only where the firmware gives them")
                .try_this("firmware-errors"),
        ),
        many => Err(Failure::new(
            Exit::Usage,
            format!("{id} starts the ids of {} kept boots", many.len()),
        )
        .try_this(format!("firmware-errors {}", many[0].boot))),
    }
}

/// One boot, by its id or the start of it.
fn one(context: &Context, kept: &[Kept], here: &Here, id: &str) -> Result<(), Failure> {
    let boot = named(kept, id)?;
    if context.format().is_json() {
        let mut answer = boot_json(boot, here);
        if let Some(fields) = answer.as_object_mut() {
            fields.insert("schema".to_owned(), json!("firmware-errors.boot/1"));
        }
        return said(context.answer(&answer));
    }
    let facts = facts(boot, here);
    let facts: Vec<(&str, Option<&str>)> = facts
        .iter()
        .map(|(label, value)| (label.as_str(), value.as_deref()))
        .collect();
    said(context.say(&layout::joined(vec![layout::facts(
        &facts,
        "not kept",
        &context.out,
        context.room,
    )])))
}

fn main() -> ExitCode {
    terminal_kit::main(&help(), env!("CARGO_PKG_VERSION"), |context| {
        let state = state();
        let kept = boots::kept_in(&state).map_err(|error| unread(&state, &error))?;
        let here = Here::read();
        match context.args.as_slice() {
            [] => list(context, &kept, &here),
            [id] => one(context, &kept, &here, id),
            _ => Err(Failure::new(Exit::Usage, "it takes one boot at most")
                .try_this("firmware-errors 6bccd85d")),
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn here(current: &str) -> Here {
        Here {
            current: current.to_owned(),
            watchdog: Some(Watchdog {
                identity: "iTCO_wdt".to_owned(),
                reset_it: true,
            }),
            named: true,
        }
    }

    fn boot(reset: Option<Reset>, crash: Option<Crash>) -> Kept {
        Kept {
            boot: "6bccd85d80714583afd805c687f0cdb6".to_owned(),
            began: Some(0),
            written: Some(0),
            health: reset.is_some(),
            reset,
            crash,
            unread: Vec::new(),
        }
    }

    /// The value one of a boot's facts is given, by its label.
    fn fact(facts: &[(String, Option<String>)], label: &str) -> Option<String> {
        facts
            .iter()
            .find(|(named, _)| named == label)
            .and_then(|(_, value)| value.clone())
    }

    #[test]
    fn a_health_table_kept_naming_no_reset_never_reads_as_lost() {
        let lost = facts(&boot(None, None), &here(""));
        let named_none = Kept {
            health: true,
            ..boot(None, None)
        };
        let without_intel = Kept {
            health: true,
            ..boot(
                Some(Reset {
                    source: 1,
                    reason: 0,
                    intel: None,
                }),
                None,
            )
        };
        assert_eq!(
            boot_json(&named_none, &here(""))["health_table"],
            "read",
            "a kept table naming no reset was said lost in the JSON"
        );
        assert_eq!(
            boot_json(&boot(None, None), &here(""))["health_table"],
            "not-kept",
            "a table never kept was said kept in the JSON"
        );
        let named_none = facts(&named_none, &here(""));
        let without_intel = facts(&without_intel, &here(""));
        for label in ["Reset source", "Reset reason", "Intel's reset cause"] {
            assert_eq!(
                fact(&lost, label),
                None,
                "{label} was said of a table not kept"
            );
            let said = fact(&named_none, label);
            assert!(
                said.as_deref()
                    .is_some_and(|said| said.starts_with("none: the health table")),
                "{label} of a kept table naming no reset read {said:?}"
            );
        }
        assert_eq!(
            fact(&without_intel, "Intel's reset cause").as_deref(),
            Some("none in the health table"),
            "a kept table without Intel's words read as lost"
        );
    }

    #[test]
    fn a_table_kept_but_unreadable_never_reads_as_not_kept() {
        let closed = Kept {
            unread: vec![
                (boots::PHAT_TABLE, "Permission denied".to_owned()),
                (boots::BERT_REGION, "Permission denied".to_owned()),
            ],
            ..boot(None, None)
        };
        let said = facts(&closed, &here(""));
        for label in [
            "Reset source",
            "Reset reason",
            "Intel's reset cause",
            "Crash Log",
        ] {
            assert_eq!(
                fact(&said, label).as_deref(),
                Some("unreadable: see Couldn't read"),
                "{label} of a table kept but unreadable read as not kept"
            );
        }
        assert_eq!(
            boot_json(&closed, &here(""))["health_table"],
            "unreadable",
            "a table that couldn't be read was said read or lost"
        );
    }

    #[test]
    fn a_machine_without_a_watchdog_says_so_rather_than_not_kept() {
        let current = boot(None, None);
        let without = Here {
            watchdog: None,
            ..here(&current.boot)
        };
        assert_eq!(
            fact(&facts(&current, &without), "Watchdog").as_deref(),
            Some("none, or it doesn't say"),
            "a missing watchdog read as one not kept"
        );
    }

    #[test]
    fn a_boot_answers_facts_and_names_never_sentences() {
        let reset = Reset {
            source: 1,
            reason: 0x26,
            intel: Some([0x400, 0, 1, 0x1039_0000]),
        };
        let answer = boot_json(&boot(Some(reset), Some(Crash::Other)), &here(""));
        assert_eq!(
            answer,
            json!({
                "boot": "6bccd85d80714583afd805c687f0cdb6",
                "began": "1970-01-01T00:00:00Z",
                "current": false,
                "end": "global-reset",
                "weight": "reset",
                "health_table": "read",
                "reset": {
                    "source": 1,
                    "source_name": "unknown",
                    "reason": 0x26,
                    "reason_name": null,
                    "intel": {"words": [0x400, 0, 1, 0x1039_0000], "causes": ["PMC_FW"]},
                },
                "crash_log": {"state": "other", "tags": []},
                "watchdog": null,
                "unread": [],
            })
        );
    }

    #[test]
    fn every_boot_has_every_key_whatever_was_kept() {
        let bare = boot_json(&boot(None, None), &here("6bccd85d80714583afd805c687f0cdb6"));
        let full = boot_json(
            &boot(
                Some(Reset {
                    source: 2,
                    reason: 1,
                    intel: None,
                }),
                Some(Crash::Unreadable("cut".to_owned())),
            ),
            &here(""),
        );
        let keys = |answer: &Value| {
            answer
                .as_object()
                .map(|fields| fields.keys().cloned().collect::<Vec<_>>())
        };
        assert_eq!(keys(&bare), keys(&full));
        assert_eq!(
            bare["end"], "watchdog-reset",
            "the watchdog's word wasn't the current boot's"
        );
        assert_eq!(bare["watchdog"]["identity"], "iTCO_wdt");
        assert_eq!(
            full["crash_log"],
            json!({"state": "unreadable", "tags": []})
        );
        assert_eq!(full["reset"]["intel"], Value::Null);
    }

    #[test]
    fn an_id_names_one_boot_or_says_why_not() {
        let mut other = boot(None, None);
        other.boot = "6bccd85e00000000000000000000000a".to_owned();
        let kept = [boot(None, None), other];
        let exit = |id: &str| named(&kept, id).err().map(|failure| failure.exit);
        assert_eq!(exit("6bccd85d"), None);
        assert_eq!(exit("6BCC-D85D"), None);
        assert_eq!(
            exit("6bccd85"),
            Some(Exit::Usage),
            "two boots start 6bccd85"
        );
        assert_eq!(exit("949a"), Some(Exit::NotFound));
        assert_eq!(exit(""), Some(Exit::Usage));
        assert_eq!(exit("-"), Some(Exit::Usage));
    }

    #[test]
    fn a_number_the_specification_doesnt_name_is_kept_in_the_facts() {
        let reset = Reset {
            source: 0b110,
            reason: 0x26,
            intel: None,
        };
        let facts = facts(&boot(Some(reset), None), &here(""));
        let fact = |label: &str| {
            facts
                .iter()
                .find(|(named, _)| named == label)
                .and_then(|(_, value)| value.clone())
        };
        assert_eq!(
            fact("Reset source").as_deref(),
            Some("0x06, which the specification doesn't name")
        );
        assert_eq!(
            fact("Reset reason").as_deref(),
            Some("0x26, which the specification doesn't name")
        );
        assert_eq!(fact("Crash Log"), None);
    }
}
