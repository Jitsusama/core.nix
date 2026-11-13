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
//! ```text
//! eero [view] [--json]            a view of the home network, status bare
//! eero watch <view> [--json]      the view, drawn again as it changes
//! eero sign-in <email or phone>   sign in once, typing the code eero sends
//! eero speedtest                  start the eero's own speed test
//! eero sqm on|off                 optimize for conferencing and gaming, or not
//! eero daemon                     keep the home network for the shell
//! ```
//!
//! The views: status, nodes, node <location|id>, mesh, devices,
//! device <name|mac>, internet, speeds, firmware, radio, traffic, profiles,
//! profile <name|id> and events.
//!
//! For the shell: `eero completion zsh` writes zsh's completion, and
//! `eero names views|nodes|devices|profiles` what it offers of each. Both
//! are read by zsh, so their text is exactly what it reads.
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
use terminal_kit::{Context, Exit, Failure, Help, flags};

use cloud::Cloud;

/// The kinds completion asks the names of.
const KINDS: [&str; 4] = ["views", "nodes", "devices", "profiles"];

fn help() -> Help {
    let mut help = Help::new(
        "eero",
        "the home network's eero, read from its cloud at home or away",
    )
    .example("eero", "the home network at a glance")
    .example(
        "eero node Study",
        "one node: backhaul, radios, ports, devices",
    )
    .example(
        "eero watch devices",
        "every device on now, drawn as it changes",
    )
    .example(
        "eero devices --json",
        "the same for a program, as eero.devices/1",
    );
    for (view, words) in views::VIEWS {
        help = help.command(view, words);
    }
    help.command("watch", "a view, drawn again each time it changes")
        .command("sign-in", "sign in once, typing the code eero sends")
        .command("speedtest", "start the eero's own speed test")
        .command("sqm", "optimize for conferencing and gaming: on or off")
        .command("daemon", "keep the home network for the shell")
        .command("completion", "zsh's completion, as zsh reads it")
        .command(
            "names",
            "what completion offers of views, nodes and the rest",
        )
        .exit(
            Exit::Failed,
            "eero refused, or the daemon or a watch stopped",
        )
        .exit(Exit::BadInput, "the record isn't one this eero reads")
        .exit(
            Exit::NotFound,
            "no node, device or profile goes by that name",
        )
        .exit(
            Exit::Unavailable,
            "eero-daemon isn't running, or eero's cloud is down",
        )
        .exit(
            Exit::Refused,
            "not signed in, or no terminal to type the code in",
        )
        .exit(Exit::Config, "XDG_RUNTIME_DIR isn't set")
}

fn main() -> ExitCode {
    let help = help();
    terminal_kit::main(&help, env!("CARGO_PKG_VERSION"), |context| {
        run(context, &help)
    })
}

fn run(context: &Context, help: &Help) -> Result<(), Failure> {
    let words: Vec<&str> = context.args.iter().map(String::as_str).collect();
    match words.as_slice() {
        [] => view(context, &["status"]),
        [word, ..] if views::View::named(word) => view(context, &words),
        ["watch"] => {
            Err(Failure::new(Exit::Usage, "name a view to watch").try_this("eero watch status"))
        }
        ["watch", rest @ ..] => watch(context, rest),
        ["completion", "zsh"] => said(context.say(include_str!("completion.zsh"))),
        ["completion", ..] => Err(
            Failure::new(Exit::Usage, "completion is written for zsh alone")
                .try_this("eero completion zsh"),
        ),
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
        ["speedtest"] => speedtest(context),
        ["sqm", "on"] => conferencing(context, true),
        ["sqm", "off"] => conferencing(context, false),
        ["sqm", ..] => {
            Err(Failure::new(Exit::Usage, "sqm takes on or off").try_this("eero sqm on"))
        }
        ["daemon"] => daemon::run().map_err(|why| Failure::new(Exit::Failed, why)),
        [word, ..] if help.command_names().contains(word) => Err(Failure::new(
            Exit::Usage,
            format!("{word} takes no more words"),
        )
        .try_this("eero --help")),
        [word, ..] => Err(flags::unknown("command", word, &help.command_names())),
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
    let page = live::show(&view, json, &views::theme::ink(context.out.inks()))?;
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
        .map_err(|why| Failure::new(Exit::Usage, why).try_this("eero watch status"))?;
    let how = watch::Watch {
        view: &view,
        json: context.format().is_json(),
        ink: &views::theme::ink(context.out.inks()),
        terminal: context.environment.stdout,
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
