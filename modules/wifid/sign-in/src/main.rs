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

mod socks;
mod wifi;

use std::fs::File;
use std::io::{self, Read};
use std::net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream};
use std::path::Path;
use std::thread;
use std::time::Duration;

use gtk4 as gtk;
use gtk4::glib;
use gtk4::prelude::*;
use serde_json::{Value, json};
use socket2::{Domain, Socket, Type};
use webkit6::prelude::*;

use socks::{Credentials, Target};

/// How long a connection has to be made before it's given up.
const PATIENCE: Duration = Duration::from_secs(10);

fn main() -> glib::ExitCode {
    let app = gtk::Application::builder()
        .application_id("io.jitsusama.WifiSignIn")
        .build();
    app.connect_activate(open);
    app.run_with_args::<&str>(&[])
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

fn open(app: &gtk::Application) {
    let socket = Path::new(wifi::SOCKET);
    let described = wifi::call(socket, "Describe", json!({}));
    let Some(portal) = described.ok().as_ref().and_then(wanting) else {
        eprintln!("no network wants a sign-in");
        return;
    };
    let proxy = match proxy(&portal.card) {
        Ok(proxy) => proxy,
        Err(error) => return eprintln!("can't open the window's proxy: {error}"),
    };

    let session = webkit6::NetworkSession::new_ephemeral();
    let settings = webkit6::NetworkProxySettings::new(Some(&proxy), &[]);
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
        socks::serve(listener, credentials, move |target| connect(&card, target))
    });
    Ok(address)
}

/// Sixteen random bytes, in hex, for a username or password nobody else
/// can know.
fn token() -> io::Result<String> {
    let mut bytes = [0; 16];
    File::open("/dev/urandom")?.read_exact(&mut bytes)?;
    Ok(bytes.iter().map(|byte| format!("{byte:02x}")).collect())
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
