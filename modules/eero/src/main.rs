//! eero: the home network's eero, read from this machine at home or away.
//!
//! The eero has no local API, so everything goes through eero's cloud
//! (cloud.rs), under a session Joel makes once by typing the code eero sends
//! him (session.rs). It reads, and changes three things: "Optimize for
//! conferencing and gaming", eero's speed test, and the address kept for a
//! device, which goes only through the daemon (asking.rs).
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
//! eero devices <MAC> reserve <ip> keep an address for a device; unreserve to stop
//! eero sign-in <email or phone>   sign in once, typing the code eero sends
//! eero daemon                     keep the home network for the desktop
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
mod asking;
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
        ["devices" | "device", mac, "reserve", address] => reserve(context, mac, Some(address)),
        ["devices" | "device", mac, "unreserve"] => reserve(context, mac, None),
        ["devices" | "device", _, "reserve" | "unreserve", ..] => Err(Failure::new(
            Exit::Usage,
            "reserve takes the address to keep, and unreserve nothing more",
        )
        .try_this("eero devices 02:00:00:00:00:01 reserve 192.168.4.20")),
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
    let page = live::show(&view, json, &views::theme::ink(&context.out), room)?;
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
        ink: &views::theme::ink(&context.out),
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
        cloud::Failure::Unknown(why) => {
            Failure::new(Exit::TryAgain, "whether eero took it isn't known").because(why)
        }
    }
}

fn sign_in(context: &Context, login: &str) -> Result<(), Failure> {
    if !context.can_ask() {
        return Err(
            Failure::new(Exit::Refused, "sign-in asks for the code eero sends")
                .because("stdin isn't a terminal to type it in, so run it in one")
                .try_this(format!(
                    "eero sign-in {}",
                    terminal_kit::failure::shell_word(login)
                )),
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
        acts::done(&mut cloud, url, acts::Act::SpeedTest)
            .done
            .map_err(from_cloud)?;
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
        acts::done(&mut cloud, &url, acts::Act::Conferencing(on))
            .done
            .map_err(from_cloud)?;
        networks.push(api::network(&url, &cloud.get(&url).map_err(from_cloud)?));
    }
    if context.format().is_json() {
        let networks: Vec<Value> = networks
            .iter()
            .map(|network| json!({"name": network.name, "url": network.url, "sqm": network.sqm}))
            .collect();
        return said(context.answer(&json!({"schema": "eero.sqm/1", "networks": networks})));
    }
    said(context.say(&each_said(&networks)))
}

/// Keeps `address` for the device `mac`, or with none stops keeping one,
/// by asking the daemon (asking.rs). It says what it changes before
/// asking, since the change is to the whole house, and its undo once it's
/// known to have changed anything.
fn reserve(context: &Context, mac: &str, address: Option<&str>) -> Result<(), Failure> {
    let (mac, address) = reservation(mac, address)?;
    let (asked, doing) = match address {
        Some(address) => (
            json!({"act": "reserve", "mac": mac.to_string(), "address": address.to_string()}),
            format!("Keeping {address} for {mac}"),
        ),
        None => (
            json!({"act": "unreserve", "mac": mac.to_string()}),
            format!("No longer keeping an address for {mac}"),
        ),
    };
    eprintln!("{doing}.");
    let answered = asking::act(&asked).map_err(|undone| undone_said(undone, mac))?;
    let report = reported(mac, address, &answered);
    if context.format().is_json() {
        return said(context.answer(&json!({
            "schema": "eero.reservation/1",
            "mac": mac.to_string(),
            "address": address.map(|address| address.to_string()),
            "unchanged": answered["unchanged"] == json!(true),
            "undo": report.undo,
        })));
    }
    said(context.say(&report.said))
}

/// What a reservation the daemon did comes to, for a person and an agent.
#[derive(Debug, PartialEq, Eq)]
struct Report {
    /// The command that undoes it, if it changed anything.
    undo: Option<String>,
    /// Its line.
    said: String,
}

/// The report of keeping `address` for `mac`, or with none no longer
/// keeping one, as the daemon `answered` it done: one that already stood
/// changed nothing, so it has no undo, since undoing it would undo
/// someone else's reservation. An unreservation's undo keeps again the
/// address the daemon says it held.
fn reported(mac: acts::Mac, address: Option<std::net::Ipv4Addr>, answered: &Value) -> Report {
    let unchanged = answered["unchanged"] == json!(true);
    let undone = |undo: String, said: String| Report {
        said: format!("{said}; undo with {undo}.\n"),
        undo: Some(undo),
    };
    let unchanged_said = |said: String| Report {
        undo: None,
        said: format!("{said}; nothing changed.\n"),
    };
    match (address, unchanged, answered["held"].as_str()) {
        (Some(address), false, _) => undone(
            format!("eero devices {mac} unreserve"),
            format!("eero keeps {address} for {mac}"),
        ),
        (Some(address), true, _) => {
            unchanged_said(format!("eero already kept {address} for {mac}"))
        }
        (None, false, Some(held)) => undone(
            format!("eero devices {mac} reserve {held}"),
            format!("eero keeps no address for {mac}"),
        ),
        (None, true, _) => unchanged_said(format!("eero kept no address for {mac}")),
        // Done and changed, yet no address said: it did change something,
        // so it's never said as nothing, though its undo can't be given.
        (None, false, None) => Report {
            undo: None,
            said: format!(
                "eero keeps no address for {mac}; the daemon didn't say which it kept, so \
                 see the eero app to undo it.\n"
            ),
        },
    }
}

/// How an act asked of the daemon that wasn't done is said, each by what
/// mends it: an outcome no one knows is said to be read again, never as
/// done or refused.
fn undone_said(undone: asking::Undone, mac: acts::Mac) -> Failure {
    match undone {
        asking::Undone::Absent(why) => {
            Failure::new(Exit::Unavailable, "eero-daemon isn't answering")
                .because(why)
                .try_this("systemctl --user status eero-daemon")
        }
        // Eero's own refusal and the daemon's guards, which send nothing,
        // both come here; nothing changed either way, and the reason says
        // which held it back.
        asking::Undone::Refused(why) => {
            Failure::new(Exit::Failed, "refused, so nothing changed").because(why)
        }
        asking::Undone::Unreachable(why) => {
            Failure::new(Exit::TryAgain, "eero couldn't be reached").because(why)
        }
        asking::Undone::SignedOut(why) => Failure::new(Exit::Refused, "not signed in to eero")
            .because(why)
            .try_this("eero sign-in <email or phone>"),
        asking::Undone::Unknown(why) => {
            // The device's record is rewritten only once the daemon's next
            // read ends, which it starts as it answers, so read now it
            // still shows the reservation as it was before the act.
            Failure::new(
                Exit::TryAgain,
                "whether eero took it isn't known until the daemon's next read, a few seconds on",
            )
            .because(why)
            .try_this(format!("eero devices {mac}"))
        }
        asking::Undone::Unlogged(why) => Failure::new(
            Exit::Unavailable,
            "eero-daemon couldn't log the ask, so nothing was done",
        )
        .because(why)
        .try_this("journalctl --user -u eero-daemon"),
        asking::Undone::NoSuchAct(why) => Failure::new(
            Exit::Unavailable,
            "eero-daemon doesn't know this act, as one older than this command wouldn't",
        )
        .because(why)
        .try_this("systemctl --user status eero-daemon"),
        asking::Undone::Misasked(why) => Failure::new(
            Exit::Bug,
            "eero-daemon couldn't read this command's ask, so nothing was done",
        )
        .because(why),
        asking::Undone::NotOurs(why) => Failure::new(
            Exit::Refused,
            "eero-daemon answers only its own account, so nothing was asked",
        )
        .because(why),
    }
}

/// The device and the address, if any, a reservation names, or why it
/// names none.
fn reservation(
    mac: &str,
    address: Option<&str>,
) -> Result<(acts::Mac, Option<std::net::Ipv4Addr>), Failure> {
    let mac = acts::Mac::parse(mac).ok_or_else(|| {
        Failure::new(Exit::Usage, "a reservation names a device by its MAC")
            .try_this("eero devices 02:00:00:00:00:01 reserve 192.168.4.20")
    })?;
    let address: Option<std::net::Ipv4Addr> = address
        .map(|address| {
            address.parse().map_err(|_| {
                Failure::new(Exit::Usage, "a reservation keeps an IPv4 address")
                    .try_this(format!("eero devices {mac} reserve 192.168.4.20"))
            })
        })
        .transpose()?;
    Ok((mac, address))
}

/// Each network's status a line, for a person. A network's name is
/// anyone's to set, so each line is drawn without what the terminal kit
/// keeps from the terminal, as a page is.
fn each_said(networks: &[api::Network]) -> String {
    let mut lines: Vec<String> = networks
        .iter()
        .map(|network| terminal_kit::layout::one_line(&status::line(network, None)).into_owned())
        .collect();
    lines.push(String::new());
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::{
        Report, acts, api, asking, commands, each_said, reported, reservation, undone_said,
        view_words,
    };
    use serde_json::json;
    use terminal_kit::{Exit, tree};

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
            (
                "devices 02:00:00:00:00:01 reserve --help",
                "eero devices <name> reserve",
            ),
            (
                "devices 02:00:00:00:00:01 unreserve --help",
                "eero devices <name> unreserve",
            ),
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

    #[test]
    fn a_networks_name_says_one_status_line_and_no_control() {
        let named =
            json!({"name": "Home\n\u{1b}[2Jeero  online", "status": "connected", "sqm": true});
        let networks = [api::network("/2.2/networks/1", &named)];
        assert_eq!(
            each_said(&networks),
            "Home [2Jeero  online  online  conferencing on\n",
            "a network's name drew a line of its own, or a control reached the terminal"
        );
    }

    #[test]
    fn a_reservation_names_a_mac_and_an_ipv4_address_or_is_usage() {
        assert!(
            reservation("02:00:00:00:00:01", Some("10.0.0.31")).is_ok(),
            "a good reservation was refused"
        );
        assert!(
            reservation("02:00:00:00:00:01", None).is_ok(),
            "a good unreservation was refused"
        );
        for (mac, address) in [
            ("Pixel", Some("10.0.0.31")),
            ("Pixel", None),
            ("02:00:00:00:00:01", Some("10.0.0")),
            ("02:00:00:00:00:01", Some("Pixel")),
        ] {
            assert_eq!(
                reservation(mac, address).err().map(|failure| failure.exit),
                Some(terminal_kit::Exit::Usage),
                "eero devices {mac} reserve {address:?} should be refused with 64"
            );
        }
    }

    fn mac() -> acts::Mac {
        acts::Mac::parse("02:00:00:00:00:38").expect("a MAC")
    }

    #[test]
    fn each_act_undone_exits_by_what_mends_it() {
        use asking::Undone;
        let why = || "why".to_owned();
        let exits = [
            Undone::Absent(why()),
            Undone::Refused(why()),
            Undone::Unreachable(why()),
            Undone::SignedOut(why()),
            Undone::Unknown(why()),
            Undone::Unlogged(why()),
            Undone::NoSuchAct(why()),
            Undone::Misasked(why()),
            Undone::NotOurs(why()),
        ]
        .map(|undone| undone_said(undone, mac()).exit);
        assert_eq!(
            exits,
            [
                Exit::Unavailable,
                Exit::Failed,
                Exit::TryAgain,
                Exit::Refused,
                Exit::TryAgain,
                Exit::Unavailable,
                Exit::Unavailable,
                Exit::Bug,
                Exit::Refused,
            ],
            "an act undone exits as something other mends it, which an agent would act on"
        );
        assert_eq!(
            undone_said(Undone::Unknown(why()), mac()).then.as_deref(),
            Some("eero devices 02:00:00:00:00:38"),
            "an outcome no one knows doesn't say to read the device again"
        );
    }

    #[test]
    fn a_reservation_has_an_undo_only_where_it_changed_something() {
        let kept = Some("10.0.0.208".parse().expect("an address"));
        let report = |undo: Option<&str>, said: &str| Report {
            undo: undo.map(str::to_owned),
            said: said.to_owned(),
        };
        assert_eq!(
            [
                reported(mac(), kept, &json!({"done": true})),
                reported(mac(), kept, &json!({"done": true, "unchanged": true})),
                reported(mac(), None, &json!({"done": true, "held": "10.0.0.208"})),
                reported(mac(), None, &json!({"done": true, "unchanged": true})),
                reported(mac(), None, &json!({"done": true})),
            ],
            [
                report(
                    Some("eero devices 02:00:00:00:00:38 unreserve"),
                    "eero keeps 10.0.0.208 for 02:00:00:00:00:38; undo with eero devices \
                     02:00:00:00:00:38 unreserve.\n"
                ),
                report(
                    None,
                    "eero already kept 10.0.0.208 for 02:00:00:00:00:38; nothing changed.\n"
                ),
                report(
                    Some("eero devices 02:00:00:00:00:38 reserve 10.0.0.208"),
                    "eero keeps no address for 02:00:00:00:00:38; undo with eero devices \
                     02:00:00:00:00:38 reserve 10.0.0.208.\n"
                ),
                report(
                    None,
                    "eero kept no address for 02:00:00:00:00:38; nothing changed.\n"
                ),
                report(
                    None,
                    "eero keeps no address for 02:00:00:00:00:38; the daemon didn't say which \
                     it kept, so see the eero app to undo it.\n"
                ),
            ],
            "a reservation that changed nothing gave an undo, or one that did gave none"
        );
    }
}
