//! wifid-sign-in: a window onto the sign-in page of the network that wants
//! one, which the shell opens only when Joel asks.
//!
//! The page is reached the way wifid's check reaches it: through the card,
//! with names looked up by the network's own DNS servers, so a VPN routing
//! and resolving everything else, as WARP does, can't stand in the way. The
//! web view's traffic goes through a SOCKS5 proxy of the window's own on a
//! loopback port (socks.rs), which asks a username and password made up for
//! this window alone. Its session keeps nothing once it closes.
//!
//! After each page loads, wifid checks the network again, and the window
//! closes once the network reaches beyond itself, or the card leaves it.
//!
//! It takes the shape every command of ours does (docs/commands.md),
//! through the terminal kit, and asks wifid and starts its proxy before
//! any window: with no network wanting a sign-in it says so and exits 0,
//! where wifid can't say it exits 69, and where the proxy won't start, or
//! GTK ends before its window closes, it exits 1. The window runs once
//! the kit's body has ended, since a panic in one of GTK's callbacks ends
//! the process at GTK's edge, where the kit's catch never sees it and
//! would only keep its words back.
//!
//! A second run while a window is open hands itself to the first, which
//! brings its window forward.

mod socks;
mod wifi;

use std::cell::Cell;
use std::fmt::Write as _;
use std::fs::File;
use std::io::{self, Read};
use std::net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream};
use std::path::Path;
use std::process::ExitCode;
use std::rc::Rc;
use std::thread;
use std::time::Duration;

use gtk4 as gtk;
use gtk4::glib;
use gtk4::prelude::*;
use serde_json::{Value, json};
use socket2::{Domain, Socket, Type};
use terminal_kit::layout::Room;
use terminal_kit::{Exit, Failure, Help, Ink, Pen};
use webkit6::prelude::*;

use socks::{Credentials, Target};

/// How long a connection has to be made before it's given up.
const PATIENCE: Duration = Duration::from_secs(10);

/// How many times a page that didn't load is tried again.
const RETRIES: u32 = 5;

fn help() -> Help {
    Help::new(
        "wifid-sign-in",
        "opens the sign-in page of the network that wants one",
    )
    .example("wifid-sign-in", "what the shell runs when Joel asks")
    .exit(
        Exit::Done,
        "the window closed, or no network wants a sign-in",
    )
    .exit(
        Exit::Unavailable,
        "wifid didn't say which network wants one",
    )
    .exit(
        Exit::Failed,
        "the window's proxy wouldn't start, or GTK ended before the window closed",
    )
}

fn main() -> ExitCode {
    // bubblewrap refuses to start WebKit's sandbox for a process holding
    // capabilities, and WebKit then gives up. A session can hand its
    // programs some, as a compositor run from a system service does, and
    // the window needs none. The ambient set is each thread's own, so it's
    // cleared before any thread starts, the proxy's among them.
    // SAFETY: PR_CAP_AMBIENT_CLEAR_ALL takes no pointers, needs no
    // privilege, and touches only this thread's ambient set.
    unsafe {
        libc::prctl(
            libc::PR_CAP_AMBIENT,
            libc::PR_CAP_AMBIENT_CLEAR_ALL,
            0,
            0,
            0,
        )
    };
    let mut window = None;
    let ended = terminal_kit::main(&help(), env!("CARGO_PKG_VERSION"), |context| {
        takes_no_words(&context.args)?;
        match prepared(Path::new(wifi::SOCKET))? {
            Some(prepared) => window = Some(prepared),
            None => {
                let _ = context.tell("done", "muted", "no network wants a sign-in");
            }
        }
        Ok(())
    });
    match window {
        Some((portal, proxy)) => show(portal, proxy),
        None => ended,
    }
}

/// The portal of the network that wants a sign-in, as wifid at `socket`
/// says, and the proxy its window's traffic goes through; nothing when no
/// network wants one.
fn prepared(socket: &Path) -> Result<Option<(Portal, String)>, Failure> {
    let described = wifi::call_within(socket, "Describe", PATIENCE).map_err(|error| {
        Failure::new(
            Exit::Unavailable,
            "wifid didn't say which network wants a sign-in",
        )
        .because(error.to_string())
    })?;
    let Some(portal) = wanting(&described) else {
        return Ok(None);
    };
    let proxy = proxy(&portal.card).map_err(|error| {
        Failure::new(Exit::Failed, "the window's proxy wouldn't start").because(error.to_string())
    })?;
    Ok(Some((portal, proxy)))
}

/// Refuses any word, since the command takes none, so a typo never opens
/// the window onto a page; the kit has answered --help before this.
fn takes_no_words(words: &[String]) -> Result<(), Failure> {
    match words.first() {
        None => Ok(()),
        Some(word) => Err(Failure::new(
            Exit::Usage,
            format!("wifid-sign-in takes no words, not {word}"),
        )
        .try_this("wifid-sign-in --help")),
    }
}

/// Shows the window onto the portal's page until it closes. Another
/// activation, a second run's, brings the open window forward.
fn show(portal: Portal, proxy: String) -> ExitCode {
    let app = gtk::Application::builder()
        .application_id("io.jitsusama.WifiSignIn")
        .build();
    app.connect_activate(move |app| match app.active_window() {
        Some(window) => window.present(),
        None => open(app, &portal, &proxy),
    });
    if app.run_with_args::<&str>(&[]) == glib::ExitCode::SUCCESS {
        return Exit::Done.into();
    }
    // GTK ended other than by its window closing, perhaps before one
    // opened, having said why; GTK that can't reach a display ends the
    // process itself as it starts. Past the kit's body, so said as the
    // kit says a failure, plainly, and ended with the kit's own code.
    let failure = Failure::new(
        Exit::Failed,
        "GTK ended before the sign-in window closed",
    );
    eprint!(
        "{}",
        failure.text(&Pen::new(Ink::plain(), None), Room::Unbounded, false)
    );
    Exit::Failed.into()
}

/// The card whose network wants a sign-in, the network, and where.
#[derive(Clone, Debug, PartialEq)]
struct Portal {
    card: String,
    network: String,
    page: String,
}

/// The first card wifid says wants a sign-in.
fn wanting(described: &Value) -> Option<Portal> {
    let cards = described["cards"].as_array()?;
    let card = cards.iter().find(|card| card["reach"] == "signIn")?;
    Some(Portal {
        card: card["name"].as_str()?.to_string(),
        network: card["network"].as_str()?.to_string(),
        page: card["signIn"].as_str()?.to_string(),
    })
}

/// Whether the window is still wanted, as wifid describes things now: the
/// card still on its network, which doesn't reach beyond itself yet. One
/// that's limited may still need its page, so the window stays for it.
fn still_wanted(described: &Value, portal: &Portal) -> bool {
    let cards = described["cards"].as_array().into_iter().flatten();
    let mut card = cards.filter(|card| card["name"] == portal.card.as_str());
    card.next()
        .is_some_and(|card| card["network"] == portal.network.as_str() && card["reach"] != "online")
}

/// Opens the window onto the portal's page, its traffic through the proxy.
fn open(app: &gtk::Application, portal: &Portal, proxy: &str) {
    let session = webkit6::NetworkSession::new_ephemeral();
    let settings = webkit6::NetworkProxySettings::new(Some(proxy), &[]);
    session.set_proxy_settings(webkit6::NetworkProxyMode::Custom, Some(&settings));
    let view = webkit6::WebView::builder()
        .network_session(&session)
        .build();

    let title = gtk::Label::new(Some(&format!("Sign in to {}", portal.network)));
    title.add_css_class("title");
    let safety = gtk::Label::new(None);
    safety.add_css_class("subtitle");
    let heading = gtk::Box::new(gtk::Orientation::Vertical, 0);
    heading.append(&title);
    heading.append(&safety);
    let header = gtk::HeaderBar::new();
    header.set_title_widget(Some(&heading));

    let window = gtk::ApplicationWindow::builder()
        .application(app)
        .title(format!("Sign in to {}", portal.network))
        .default_width(960)
        .default_height(720)
        .child(&view)
        .build();
    window.set_titlebar(Some(&header));

    // Each page loaded may be the one that signs in, so wifid checks again
    // and the window learns of it through Describe below.
    view.connect_load_changed(move |view, event| {
        if event != webkit6::LoadEvent::Finished {
            return;
        }
        safety.set_text(&secured(view));
        thread::spawn(|| {
            let _ = wifi::call(Path::new(wifi::SOCKET), "Check", json!({}));
        });
    });

    // A page that doesn't load is tried again a few times, a moment apart,
    // since a network that's just been joined can drop the first try.
    let tries = Rc::new(Cell::new(0));
    view.connect_load_failed(move |view, _, address, error| {
        eprintln!("can't load {address}: {error}");
        if tries.get() < RETRIES {
            tries.set(tries.get() + 1);
            let view = view.clone();
            glib::timeout_add_local_once(Duration::from_secs(2), move || view.reload());
        }
        false
    });

    let (signed_in, done) = async_channel::bounded(1);
    let following = portal.clone();
    thread::spawn(move || {
        let Ok(described) = wifi::follow(Path::new(wifi::SOCKET)) else {
            return;
        };
        for described in described {
            if !still_wanted(&described, &following) {
                let _ = signed_in.send_blocking(());
                return;
            }
        }
    });
    let closing = window.clone();
    glib::spawn_future_local(async move {
        if done.recv().await.is_ok() {
            closing.close();
        }
    });

    view.load_uri(&portal.page);
    window.present();
}

/// Whether the page came over HTTPS, in a word for the heading.
fn secured(view: &webkit6::WebView) -> String {
    let address = view.uri().map(|uri| uri.to_string()).unwrap_or_default();
    if address.starts_with("https:") && view.tls_info().is_some() {
        "over HTTPS".into()
    } else {
        "not over HTTPS".into()
    }
}

/// Starts the window's proxy on a loopback port, connecting through the
/// card, and gives its address for the web view, its credentials in it.
fn proxy(card: &str) -> io::Result<String> {
    let credentials = Credentials {
        username: token()?,
        password: token()?,
    };
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))?;
    let port = listener.local_addr()?.port();
    let address = format!(
        "socks5://{}:{}@127.0.0.1:{port}",
        credentials.username, credentials.password
    );
    let card = card.to_string();
    thread::spawn(move || {
        socks::serve(&listener, &credentials, move |target| {
            connect(&card, target)
                .inspect_err(|error| eprintln!("can't reach {target:?} through {card}: {error}"))
        });
    });
    Ok(address)
}

/// Sixteen random bytes, in hex, for a username or password nobody else
/// can know.
fn token() -> io::Result<String> {
    let mut bytes = [0; 16];
    File::open("/dev/urandom")?.read_exact(&mut bytes)?;
    Ok(bytes.iter().fold(String::new(), |mut hex, byte| {
        // Writing to a String can't fail.
        _ = write!(hex, "{byte:02x}");
        hex
    }))
}

/// Connects where the page asks through the card alone, a name looked up
/// by the card's network.
fn connect(card: &str, target: &Target) -> io::Result<TcpStream> {
    let addresses: Vec<SocketAddr> = match target {
        Target::Address(address) => vec![*address],
        Target::Name(name, port) => wifi::lookup(Path::new(wifi::SOCKET), card, name)?
            .into_iter()
            .map(|address| SocketAddr::new(address, *port))
            .collect(),
    };
    let mut reason = io::Error::from(io::ErrorKind::NotFound);
    for address in addresses {
        let socket = Socket::new(Domain::for_address(address), Type::STREAM, None)?;
        socket.bind_device(Some(card.as_bytes()))?;
        match socket.connect_timeout(&address.into(), PATIENCE) {
            Ok(()) => return Ok(socket.into()),
            Err(error) => reason = error,
        }
    }
    Err(reason)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_no_words_opens_the_window() {
        let words = |said: &[&str]| {
            said.iter()
                .map(|word| (*word).to_owned())
                .collect::<Vec<_>>()
        };
        assert!(takes_no_words(&words(&[])).is_ok(), "no words was refused");
        for wrong in [&["flsh"][..], &["--halp"], &["x", "y"]] {
            let refused = takes_no_words(&words(wrong));
            assert_eq!(
                refused.map_err(|failure| failure.code()),
                Err(64),
                "{wrong:?} wasn't refused as asked wrongly"
            );
        }
    }

    fn inn(reach: &str) -> Value {
        json!({"cards": [{
            "name": "wlan2",
            "network": "Roadside Inn",
            "reach": reach,
            "signIn": "http://front-desk.inn/sign-in",
        }]})
    }

    fn portal() -> Portal {
        Portal {
            card: "wlan2".into(),
            network: "Roadside Inn".into(),
            page: "http://front-desk.inn/sign-in".into(),
        }
    }

    #[test]
    fn a_wifid_that_cant_be_reached_ends_it_as_unavailable() {
        let dir = tempfile::TempDir::new().unwrap();

        let refused = prepared(&dir.path().join("wifi")).map(|_| ());

        assert_eq!(refused.map_err(|failure| failure.code()), Err(69));
    }

    #[test]
    fn a_wifid_that_refuses_ends_it_as_unavailable() {
        let (_dir, socket, _wifid) =
            wifi::tests::wifid(vec![json!({"error": "io.jitsusama.Wifi.Failed"})]);

        let refused = prepared(&socket).map(|_| ());

        assert_eq!(refused.map_err(|failure| failure.code()), Err(69));
    }

    #[test]
    fn no_window_is_wanted_when_no_network_wants_a_sign_in() {
        let (_dir, socket, _wifid) = wifi::tests::wifid(vec![json!({"parameters": inn("online")})]);

        let prepared = prepared(&socket).map_err(|failure| failure.code());

        assert_eq!(prepared.map(|wanted| wanted.is_none()), Ok(true));
    }

    #[test]
    fn a_network_wanting_a_sign_in_gets_its_window_and_its_proxy() {
        let (_dir, socket, _wifid) = wifi::tests::wifid(vec![json!({"parameters": inn("signIn")})]);

        let (wanted, proxy) = prepared(&socket)
            .map_err(|failure| failure.code())
            .expect("a sign-in was wanted")
            .expect("a window was wanted");

        assert_eq!(wanted, portal());
        assert!(
            proxy.starts_with("socks5://") && proxy.contains("@127.0.0.1:"),
            "the proxy isn't the window's own on loopback: {proxy}"
        );
    }

    #[test]
    fn the_card_wanting_a_sign_in_is_the_one_signed_in_to() {
        let wanted = wanting(&inn("signIn"));

        assert_eq!(wanted, Some(portal()));
    }

    #[test]
    fn nothing_is_signed_in_to_when_nothing_wants_it() {
        assert_eq!(wanting(&inn("online")), None);
    }

    #[test]
    fn a_sign_in_is_still_wanted_while_wifid_checks_again() {
        assert!(still_wanted(&inn("signIn"), &portal()));
        assert!(still_wanted(&inn("checking"), &portal()));
    }

    #[test]
    fn a_network_reaching_beyond_itself_wants_no_more() {
        assert!(!still_wanted(&inn("online"), &portal()));
    }

    #[test]
    fn a_network_reaching_nowhere_sensible_may_still_need_its_page() {
        assert!(still_wanted(&inn("limited"), &portal()));
    }

    #[test]
    fn a_card_that_left_the_network_wants_no_more() {
        let moved = json!({"cards": [{"name": "wlan2", "network": "Lantern", "reach": "signIn"}]});

        assert!(!still_wanted(&moved, &portal()));
        assert!(!still_wanted(&json!({"cards": []}), &portal()));
    }
}
