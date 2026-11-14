//! eero: the home network's eero, read from this machine at home or away.
//!
//! The eero has no local API, so everything goes through eero's cloud
//! (cloud.rs), under a session Joel makes once by typing the code eero sends
//! him (session.rs). It reads, and changes one setting: "Optimize for
//! conferencing and gaming".
//!
//! The views read the record the daemon keeps of the home network
//! (live.rs, views/), and never eero's cloud themselves, so a view is
//! instant and says how old what it shows is.
//!
//! Its words read as a tree of resources, as kubectl's do (commands.rs):
//!
//! ```text
//! eero [view] [--watch] [--json]  a view of the home network, status bare
//! eero devices [name]             a collection, or one of it by name
//! eero nodes <name> devices       what one holds, as radios, and a profile's devices
//! eero internet sqm [on|off]      optimize for conferencing and gaming, or say it
//! eero speeds run                 start the eero's own speed test
//! eero sign-in <email or phone>   sign in once, typing the code eero sends
//! eero daemon                     keep the home network for the shell
//! ```
//!
//! The views: status, nodes [name], mesh, devices [name], profiles [name],
//! internet, speeds, firmware, radios, traffic and events. Each collection
//! answers to its singular too, and the old `eero watch <view>`,
//! `eero sqm on|off` and `eero speedtest` still work, listed nowhere.
//!
//! For the shell: `eero completion zsh` writes zsh's completion,
//! `eero names next <words>` what may follow the words typed, down the
//! tree, and `eero names views|nodes|devices|profiles` the names of each.
//! All are read by zsh, so their text is exactly what it reads.
//!
//! It takes the shape every command of ours does (docs/commands.md),
//! through the terminal kit: its help, a view as text or as JSON naming
//! its schema, `eero.<view>/1`, and a failure that says what to try. It
//! exits 69 when eero-daemon isn't keeping the record or eero's cloud
//! doesn't answer, 77 when it isn't signed in, and 66 when a view names a
//! node, device or profile the home network doesn't have.

mod acts;
mod api;
mod called;
mod cloud;
mod commands;
mod completing;
mod daemon;
mod day;
mod desktop;
mod drawers;
#[cfg(test)]
mod golden;
mod history;
mod listeners;
mod live;
mod model;
mod notices;
mod place;
mod probe;
mod read;
mod rows;
mod schedule;
mod served;
mod session;
mod status;
mod time;
mod views;
mod waiting;
mod watch;
mod words;

use std::io::{self, BufRead, Write};
use std::process::ExitCode;

use serde_json::{Value, json};
use terminal_kit::{Context, Exit, Failure, Room, flags};

use cloud::Cloud;

/// The kinds completion asks the names of.
const KINDS: [&str; 4] = ["views", "nodes", "devices", "profiles"];

fn main() -> ExitCode {
    terminal_kit::main(&commands::help(), env!("CARGO_PKG_VERSION"), run)
}

/// Runs what the words ask, which the kit has walked down eero's tree
/// (commands.rs), each old spelling put as the name it stands for.
fn run(context: &Context) -> Result<(), Failure> {
    let mut words: Vec<&str> = context.args.iter().map(String::as_str).collect();
    let watching = words.contains(&"--watch");
    words.retain(|word| *word != "--watch");
    match words.as_slice() {
        ["watch"] => {
            Err(Failure::new(Exit::Usage, "name a view to watch").try_this("eero devices --watch"))
        }
        ["watch", rest @ ..] => watch(context, &view_words(rest)),
        ["completion", "zsh"] => said(context.say(include_str!("completion.zsh"))),
        ["completion", ..] => Err(
            Failure::new(Exit::Usage, "completion is written for zsh alone")
                .try_this("eero completion zsh"),
        ),
        ["names", "next", typed @ ..] => next(context, typed),
        ["names", kind] => names(context, kind),
        ["names", ..] => Err(Failure::new(Exit::Usage, "names takes one kind")
            .because(format!("the kinds are {}", flags::and(&KINDS)))
            .try_this("eero names nodes")),
        ["sign-in", login] => sign_in(context, login),
        ["sign-in", ..] => Err(Failure::new(
            Exit::Usage,
            "sign-in takes the email address or phone number eero knows you by",
        )
        .try_this("eero sign-in you@example.com")),
        ["speeds", "run"] | ["speedtest"] => speedtest(context),
        ["sqm", "on"] | ["internet", "sqm", "on"] => conferencing(context, true),
        ["sqm", "off"] | ["internet", "sqm", "off"] => conferencing(context, false),
        ["internet", "sqm"] if watching => watch(context, &words),
        ["internet", "sqm"] => view(context, &words),
        ["sqm", ..] | ["internet", "sqm", ..] => {
            Err(Failure::new(Exit::Usage, "sqm takes on or off").try_this("eero internet sqm on"))
        }
        ["daemon"] => daemon::run().map_err(|why| Failure::new(Exit::Failed, why)),
        _ if watching => watch(context, &view_words(&words)),
        _ => view(context, &view_words(&words)),
    }
}

/// A view's words as `views::View` reads them: one of a collection named
/// by its singular, as `eero devices Pixel` is `device Pixel`, and the
/// status bare.
fn view_words<'a>(words: &[&'a str]) -> Vec<&'a str> {
    match words {
        [] => vec!["status"],
        ["nodes" | "node", name] => vec!["node", name],
        ["nodes" | "node", name, held @ ("devices" | "radios")] => vec!["node", name, held],
        ["profiles" | "profile", name, "devices"] => vec!["profile", name, "devices"],
        ["devices" | "device", name] => vec!["device", name],
        ["profiles" | "profile", name] => vec!["profile", name],
        ["node"] => vec!["nodes"],
        ["device"] => vec!["devices"],
        ["profile"] => vec!["profiles"],
        ["radios" | "radio"] => vec!["radio"],
        _ => words.to_vec(),
    }
}

/// What completion offers after the words typed, as zsh reads it
/// (completing.rs): a collection's names, read from the record, or the
/// words the tree has next.
fn next(context: &Context, typed: &[&str]) -> Result<(), Failure> {
    match completing::next(&commands::help(), typed) {
        completing::Next::Names(kind) => names(context, kind),
        completing::Next::Words(words) => said(context.say(&views::names::for_zsh(&words))),
    }
}

/// What a write to stdout came to, as the command's ending.
fn said(written: io::Result<()>) -> Result<(), Failure> {
    written.map_err(|error| {
        Failure::new(Exit::Failed, "can't write the answer").because(error.to_string())
    })
}

/// A view of the record the daemon keeps: text for a person, or the
/// record's slice of it as JSON, a line of its own under `jsonl`.
fn view(context: &Context, words: &[&str]) -> Result<(), Failure> {
    let view = views::View::parse(words)
        .map_err(|why| Failure::new(Exit::Usage, why).try_this("eero --help"))?;
    let json = context.format().is_json();
    let room = Room::of_stdout(&context.environment);
    let page = live::show(&view, json, &views::theme::ink(context.out.inks()), room)?;
    if context.format() == &terminal_kit::Format::Jsonl {
        let slice: Value = serde_json::from_str(&page).map_err(|error| {
            Failure::new(Exit::Bug, "a view's JSON didn't read back").because(error.to_string())
        })?;
        return said(context.answer(&slice));
    }
    said(context.say(&page))
}

fn watch(context: &Context, words: &[&str]) -> Result<(), Failure> {
    let view = views::View::parse(words)
        .map_err(|why| Failure::new(Exit::Usage, why).try_this("eero status --watch"))?;
    let how = watch::Watch {
        view: &view,
        json: context.format().is_json(),
        ink: &views::theme::ink(context.out.inks()),
        room: &|| Room::of_stdout(&context.environment),
    };
    watch::watch(&how, &mut io::stdout().lock())
        .map_err(|why| Failure::new(Exit::Failed, "the watch stopped").because(why))
}

/// What completion offers of a kind, as zsh reads it. The views need no
/// record; the rest are what the record holds now.
fn names(context: &Context, kind: &str) -> Result<(), Failure> {
    use views::names;
    let record = || live::read(&live::file()?);
    let named = match kind {
        "views" => names::views(),
        "nodes" => names::nodes(&record()?),
        "devices" => names::devices(&record()?),
        "profiles" => names::profiles(&record()?),
        _ => return Err(flags::unknown("kind", kind, &KINDS)),
    };
    said(context.say(&names::for_zsh(&named)))
}

/// The kit's failure for one of eero's cloud: signing in again is the only
/// mend for a session it won't take, and a cloud that isn't answering may
/// answer later.
fn from_cloud(failure: cloud::Failure) -> Failure {
    match failure {
        cloud::Failure::SignedOut(why) => Failure::new(Exit::Refused, "not signed in to eero")
            .because(why)
            .try_this("eero sign-in <email or phone>"),
        cloud::Failure::Unreachable(why) => {
            Failure::new(Exit::Unavailable, "eero's cloud isn't answering").because(why)
        }
        cloud::Failure::Refused(why) => Failure::new(Exit::Failed, "eero refused").because(why),
    }
}

fn sign_in(context: &Context, login: &str) -> Result<(), Failure> {
    if !context.can_ask() {
        return Err(
            Failure::new(Exit::Refused, "sign-in asks for the code eero sends")
                .because("stdin isn't a terminal to type it in, so run it in one")
                .try_this(format!("eero sign-in {login}")),
        );
    }
    let refused = |why: String| Failure::new(Exit::Failed, "eero didn't sign you in").because(why);
    let mut cloud = Cloud::new();
    cloud.sign_in(login).map_err(refused)?;
    eprint!("eero sent a code to {login}. Type it here: ");
    let mut code = String::new();
    io::stderr()
        .flush()
        .and_then(|()| io::stdin().lock().read_line(&mut code))
        .map_err(|error| {
            Failure::new(Exit::Failed, "can't read the code").because(error.to_string())
        })?;
    cloud.verify(code.trim()).map_err(refused)?;
    eprintln!("Signed in; the session is sealed for this account on this machine.");
    Ok(())
}

fn speedtest(context: &Context) -> Result<(), Failure> {
    let mut cloud = Cloud::signed_in().map_err(from_cloud)?;
    let networks = api::networks(&cloud.get(api::ACCOUNT).map_err(from_cloud)?);
    for url in &networks {
        acts::done(&mut cloud, url, acts::Act::SpeedTest).map_err(from_cloud)?;
    }
    if context.format().is_json() {
        return said(context.answer(&json!({
            "schema": "eero.speedtest/1",
            "networks": networks,
        })));
    }
    said(context.say(
        "The eero is testing its internet; `eero speeds` shows the result in about a minute.\n",
    ))
}

/// Turns "Optimize for conferencing and gaming" on or off on every
/// network, and says each one's status once eero reads the change back.
fn conferencing(context: &Context, on: bool) -> Result<(), Failure> {
    let mut cloud = Cloud::signed_in().map_err(from_cloud)?;
    let mut networks = Vec::new();
    for url in api::networks(&cloud.get(api::ACCOUNT).map_err(from_cloud)?) {
        acts::done(&mut cloud, &url, acts::Act::Conferencing(on)).map_err(from_cloud)?;
        networks.push(api::network(&url, &cloud.get(&url).map_err(from_cloud)?));
    }
    if context.format().is_json() {
        let networks: Vec<Value> = networks
            .iter()
            .map(|network| json!({"name": network.name, "url": network.url, "sqm": network.sqm}))
            .collect();
        return said(context.answer(&json!({"schema": "eero.sqm/1", "networks": networks})));
    }
    let mut lines: Vec<String> = networks
        .iter()
        .map(|network| status::line(network, None))
        .collect();
    lines.push(String::new());
    said(context.say(&lines.join("\n")))
}

#[cfg(test)]
mod tests {
    use super::{commands, view_words};
    use terminal_kit::tree;

    /// The words a body is given for a command line, as the kit walks it.
    fn walked(line: &str) -> Vec<String> {
        let args: Vec<String> = line.split_whitespace().map(str::to_owned).collect();
        let help = commands::help();
        match tree::walk(&help, &args) {
            Ok(walked) => walked.words,
            Err(failure) => panic!("eero {line} was refused: {}", failure.what),
        }
    }

    /// What a command line comes to as `views::View` reads it, `--watch`
    /// taken out first as `run` takes it.
    fn reads_as(line: &str) -> Vec<String> {
        let words = walked(line);
        let words: Vec<&str> = words
            .iter()
            .map(String::as_str)
            .filter(|word| *word != "--watch")
            .collect();
        view_words(&words).into_iter().map(str::to_owned).collect()
    }

    #[test]
    fn every_old_spelling_of_a_view_reads_as_it_always_did() {
        // An old spelling reaching View::parse with the very words it
        // always did is what keeps its JSON byte for byte the same.
        for old in [
            "status",
            "nodes",
            "node Study",
            "mesh",
            "devices",
            "device Pixel",
            "profiles",
            "profile Kids",
            "internet",
            "speeds",
            "firmware",
            "radio",
            "traffic",
            "events",
        ] {
            let words: Vec<String> = old.split(' ').map(str::to_owned).collect();
            assert_eq!(reads_as(old), words, "eero {old} no longer reads as it did");
        }
    }

    #[test]
    fn each_new_spelling_reads_as_the_old_one_it_replaces() {
        for (new, old) in [
            ("", "status"),
            ("nodes Study", "node Study"),
            ("devices Pixel", "device Pixel"),
            ("profiles Kids", "profile Kids"),
            ("radios", "radio"),
            ("nodes Study --watch", "node Study"),
        ] {
            let words: Vec<String> = old.split(' ').map(str::to_owned).collect();
            assert_eq!(
                reads_as(new),
                words,
                "eero {new} doesn't read as eero {old}"
            );
        }
    }

    #[test]
    fn each_eero_subcommands_help_is_titled_with_its_path() {
        let help = commands::help();
        for (line, path) in [
            ("devices --help", "eero devices"),
            ("device --help", "eero devices"),
            ("nodes Study --help", "eero nodes"),
            ("internet sqm --help", "eero internet sqm"),
            ("speeds run --help", "eero speeds run"),
            ("sqm --help", "eero sqm"),
        ] {
            let args: Vec<String> = line.split(' ').map(str::to_owned).collect();
            let shown = tree::walk(&help, &args).map(|walked| walked.shown().name);
            assert_eq!(
                shown.ok().as_deref(),
                Some(path),
                "eero {line} shows the wrong help"
            );
        }
    }

    #[test]
    fn a_word_eeros_tree_doesnt_hold_is_refused_as_usage() {
        let help = commands::help();
        for line in ["devises", "devices Pixel Study", "internet sqm maybe extra"] {
            let args: Vec<String> = line.split(' ').map(str::to_owned).collect();
            let exit = tree::walk(&help, &args).err().map(|failure| failure.exit);
            assert_eq!(
                exit,
                Some(terminal_kit::Exit::Usage),
                "eero {line} should be refused with 64"
            );
        }
    }
}
