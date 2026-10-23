use disk_guard::desktop::patient;
use disk_guard::freezer::{Freezer, thaw_all};
use std::collections::BTreeMap;
use std::os::unix::net::UnixStream;
use std::sync::{Arc, Mutex};
use zbus::blocking::connection::Builder;
use zbus::zvariant::{OwnedObjectPath, OwnedValue};

/// What the fake manager was asked, and each unit's freezer state.
#[derive(Clone, Default)]
struct Told {
    calls: Arc<Mutex<Vec<String>>>,
    units: Arc<Mutex<BTreeMap<String, &'static str>>>,
}

impl Told {
    fn calls(&self) -> Vec<String> {
        self.calls.lock().unwrap().clone()
    }
}

fn unit_path(name: &str) -> OwnedObjectPath {
    let escaped: String = name
        .bytes()
        .map(|byte| match byte {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' => (byte as char).to_string(),
            other => format!("_{other:02x}"),
        })
        .collect();
    OwnedObjectPath::try_from(format!("/org/freedesktop/systemd1/unit/{escaped}")).unwrap()
}

struct Manager(Told);

type Listed = (
    String,
    String,
    String,
    String,
    String,
    String,
    OwnedObjectPath,
    u32,
    String,
    OwnedObjectPath,
);

#[zbus::interface(name = "org.freedesktop.systemd1.Manager")]
impl Manager {
    fn start_transient_unit(
        &self,
        name: String,
        mode: &str,
        properties: Vec<(String, OwnedValue)>,
        aux: Vec<(String, Vec<(String, OwnedValue)>)>,
    ) -> OwnedObjectPath {
        // The guard asks for no auxiliary units.
        drop(aux);
        let pids = properties
            .into_iter()
            .find(|(key, _)| key == "PIDs")
            .map(|(_, value)| Vec::<u32>::try_from(value).unwrap());
        self.0
            .calls
            .lock()
            .unwrap()
            .push(format!("start {name} {mode} {pids:?}"));
        self.0.units.lock().unwrap().insert(name, "running");
        OwnedObjectPath::try_from("/org/freedesktop/systemd1/job/1").unwrap()
    }

    fn freeze_unit(&self, name: String) {
        self.0.calls.lock().unwrap().push(format!("freeze {name}"));
        self.0.units.lock().unwrap().insert(name, "frozen");
    }

    fn thaw_unit(&self, name: String) {
        self.0.calls.lock().unwrap().push(format!("thaw {name}"));
        self.0.units.lock().unwrap().insert(name, "running");
    }

    #[expect(
        clippy::unused_self,
        reason = "zbus serves a method on its object, which a unit's path needs nothing of"
    )]
    fn get_unit(&self, name: &str) -> OwnedObjectPath {
        unit_path(name)
    }

    #[expect(
        clippy::needless_pass_by_value,
        reason = "zbus hands each argument over by value, and a list can't borrow from a message"
    )]
    fn list_units_by_patterns(&self, states: Vec<String>, patterns: Vec<&str>) -> Vec<Listed> {
        // Every unit is listed, whatever its state.
        drop(states);
        assert_eq!(patterns, ["disk-guard-freeze-*.scope"]);
        let units = self.0.units.lock().unwrap();
        units
            .keys()
            .map(|name| {
                let path = unit_path(name);
                let job = OwnedObjectPath::try_from("/").unwrap();
                let none = String::new;
                (
                    name.clone(),
                    none(),
                    none(),
                    none(),
                    none(),
                    none(),
                    path,
                    0,
                    none(),
                    job,
                )
            })
            .collect()
    }
}

struct Unit {
    name: &'static str,
    told: Told,
}

#[zbus::interface(name = "org.freedesktop.systemd1.Unit")]
impl Unit {
    #[zbus(property)]
    fn freezer_state(&self) -> String {
        self.told
            .units
            .lock()
            .unwrap()
            .get(self.name)
            .copied()
            .unwrap_or("running")
            .to_owned()
    }

    #[zbus(property)]
    #[expect(
        clippy::unused_self,
        reason = "zbus serves a property on its object, and every unit here is active"
    )]
    fn active_state(&self) -> String {
        "active".to_owned()
    }
}

/// A freezer talking to a fake manager that knows `units` by name.
fn freezer(units: &[&'static str]) -> (Freezer, Told) {
    let told = Told::default();
    let (ours, theirs) = UnixStream::pair().unwrap();
    let serving = told.clone();
    let units: Vec<&'static str> = units.to_vec();
    let server = std::thread::spawn(move || {
        let mut builder = Builder::async_io_unix_stream(theirs)
            .server(zbus::Guid::generate())
            .unwrap()
            .p2p()
            .serve_at("/org/freedesktop/systemd1", Manager(serving.clone()))
            .unwrap();
        for name in units {
            let unit = Unit {
                name,
                told: serving.clone(),
            };
            builder = builder.serve_at(unit_path(name), unit).unwrap();
        }
        builder.build().unwrap()
    });
    let client = patient(Builder::async_io_unix_stream(ours).p2p()).unwrap();
    std::mem::forget(server.join().unwrap());
    (Freezer::new(client), told)
}

const UNIT: &str = "disk-guard-freeze-10.scope";

#[test]
fn a_writer_and_its_descendants_are_frozen_in_a_scope_of_their_own() {
    let (freezer, told) = freezer(&[UNIT]);
    freezer
        .freeze(UNIT, &[10, 11, 12], "dd, filling /")
        .unwrap();
    assert_eq!(
        told.calls(),
        [
            format!("start {UNIT} fail Some([10, 11, 12])"),
            format!("freeze {UNIT}")
        ]
    );
    assert!(freezer.frozen(UNIT).unwrap());
    assert_eq!(freezer.ours().unwrap(), [UNIT]);
}

#[test]
fn thawing_all_thaws_every_scope_the_guard_made_and_says_which() {
    let other = "disk-guard-freeze-20.scope";
    let (freezer, told) = freezer(&[UNIT, other]);
    freezer.freeze(UNIT, &[10], "dd, filling /").unwrap();
    freezer.freeze(other, &[20], "cp, filling /").unwrap();
    assert_eq!(thaw_all(&freezer).unwrap(), [UNIT, other]);
    assert!(told.calls().contains(&format!("thaw {UNIT}")));
    assert!(told.calls().contains(&format!("thaw {other}")));
}

#[test]
fn a_thawed_scope_runs_again() {
    let (freezer, told) = freezer(&[UNIT]);
    freezer.freeze(UNIT, &[10], "dd, filling /").unwrap();
    freezer.thaw(UNIT).unwrap();
    assert_eq!(told.calls().last().unwrap(), &format!("thaw {UNIT}"));
    assert!(!freezer.frozen(UNIT).unwrap());
}
