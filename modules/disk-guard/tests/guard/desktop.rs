use disk_guard::desktop::{Notifier, patient, saver};
use disk_guard::notices::{Change, Notice, Urgency};
use std::collections::HashMap;
use std::os::unix::net::UnixStream;
use std::sync::{Arc, Mutex};
use zbus::blocking::Connection;
use zbus::blocking::connection::Builder;
use zbus::zvariant::OwnedValue;

/// A connection to a peer serving `serve`'s objects, kept alive with it.
fn peer(serve: impl FnOnce(Builder<'static>) -> Builder<'static> + Send + 'static) -> Connection {
    let (ours, theirs) = UnixStream::pair().unwrap();
    let server = std::thread::spawn(move || {
        let builder = Builder::async_io_unix_stream(theirs)
            .server(zbus::Guid::generate())
            .unwrap()
            .p2p();
        serve(builder).build().unwrap()
    });
    let client = patient(Builder::async_io_unix_stream(ours).p2p()).unwrap();
    let server = server.join().unwrap();
    // The peer lives as long as the test, answering on its own threads.
    std::mem::forget(server);
    client
}

struct Profiles(&'static str);

#[zbus::interface(name = "org.freedesktop.UPower.PowerProfiles")]
impl Profiles {
    #[zbus(property)]
    fn active_profile(&self) -> String {
        self.0.to_owned()
    }
}

struct OldProfiles(&'static str);

#[zbus::interface(name = "net.hadess.PowerProfiles")]
impl OldProfiles {
    #[zbus(property)]
    fn active_profile(&self) -> String {
        self.0.to_owned()
    }
}

fn upower(profile: &'static str) -> Connection {
    peer(move |builder| {
        builder
            .serve_at("/org/freedesktop/UPower/PowerProfiles", Profiles(profile))
            .unwrap()
    })
}

#[test]
fn saving_power_is_read_from_the_power_profile() {
    assert!(saver(&upower("power-saver")));
    assert!(!saver(&upower("balanced")));
    let old = peer(|builder| {
        builder
            .serve_at("/net/hadess/PowerProfiles", OldProfiles("power-saver"))
            .unwrap()
    });
    assert!(saver(&old), "the older name is asked too");
}

#[test]
fn a_desktop_that_never_answers_is_given_up_on() {
    // Serving nothing, this peer never replies, as a wedged service wouldn't.
    let silent = peer(|builder| builder);
    let started = std::time::Instant::now();
    assert!(!saver(&silent), "no answer saves nothing");
    let mut notifier = Notifier::new(silent);
    assert!(notifier.tell(&show("pressure", Urgency::Normal)).is_err());
    assert!(
        started.elapsed() < std::time::Duration::from_secs(10),
        "{:?}",
        started.elapsed()
    );
}

#[derive(Clone, Debug, PartialEq)]
enum Heard {
    Notify {
        replaces: u32,
        summary: String,
        body: String,
        urgency: Option<u8>,
    },
    Close(u32),
}

#[derive(Clone, Default)]
struct Notifications {
    heard: Arc<Mutex<Vec<Heard>>>,
}

#[zbus::interface(name = "org.freedesktop.Notifications")]
impl Notifications {
    #[expect(
        clippy::too_many_arguments,
        reason = "the specification's Notify takes eight arguments"
    )]
    fn notify(
        &self,
        app_name: &str,
        replaces_id: u32,
        app_icon: &str,
        summary: &str,
        body: &str,
        actions: Vec<String>,
        hints: HashMap<String, OwnedValue>,
        expire_timeout: i32,
    ) -> u32 {
        // What the guard's notices never vary in is heard and let go.
        let _ = (app_name, app_icon, actions, expire_timeout);
        let mut heard = self.heard.lock().unwrap();
        heard.push(Heard::Notify {
            replaces: replaces_id,
            summary: summary.to_owned(),
            body: body.to_owned(),
            urgency: hints
                .into_iter()
                .find(|(name, _)| name == "urgency")
                .and_then(|(_, value)| u8::try_from(&value).ok()),
        });
        if replaces_id == 0 {
            40 + u32::try_from(heard.len()).unwrap()
        } else {
            replaces_id
        }
    }

    fn close_notification(&self, id: u32) {
        self.heard.lock().unwrap().push(Heard::Close(id));
    }
}

fn show(summary: &str, urgency: Urgency) -> Change {
    Change::Show(Notice {
        summary: summary.to_owned(),
        body: "body".to_owned(),
        urgency,
    })
}

#[test]
fn one_notice_is_replaced_in_place_and_withdrawn() {
    let server = Notifications::default();
    let heard = server.heard.clone();
    let mut notifier = Notifier::new(peer(move |builder| {
        builder
            .serve_at("/org/freedesktop/Notifications", server)
            .unwrap()
    }));
    notifier.tell(&Change::Withdraw).unwrap();
    notifier.tell(&show("pressure", Urgency::Normal)).unwrap();
    notifier.tell(&show("critical", Urgency::Critical)).unwrap();
    notifier.tell(&Change::Withdraw).unwrap();
    notifier.tell(&Change::Withdraw).unwrap();
    notifier.tell(&show("again", Urgency::Normal)).unwrap();
    let notify = |replaces, summary: &str, urgency| Heard::Notify {
        replaces,
        summary: summary.to_owned(),
        body: "body".to_owned(),
        urgency: Some(urgency),
    };
    assert_eq!(
        *heard.lock().unwrap(),
        [
            notify(0, "pressure", 1),
            notify(41, "critical", 2),
            Heard::Close(41),
            notify(0, "again", 1),
        ]
    );
}
