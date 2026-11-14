//! Whether each adapter's controller still answers. A controller can stop
//! answering while the kernel still says its adapter is up, as one wedged
//! in its firmware does, and then nothing it carries works. One that's up
//! and answers nothing it's asked for three looks in a row is said to have
//! stopped, in `stopped.json` beside the file, written only when what it
//! says changes, with what it sits on so the shell can give the command
//! that starts it again:
//!
//! ```json
//! {"adapters": [{"name": "hci0", "device": "0000:00:14.7",
//!   "driver": "/sys/bus/pci/drivers/btintel_pcie"}]}
//! ```
//!
//! Once stopped it's asked only every thirty seconds, since each command
//! it can't answer is a second waited and a line in the kernel's log, and
//! it's taken off the list the first time it answers again.

use crate::Controller;
use crate::hci::Unanswered;
use crate::reply::{self, Adapter, Version};
use serde::Serialize;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// How many looks in a row an adapter that's up is asked something and
/// answers nothing before it's said to have stopped: one could fall in a
/// busy moment, three in a row can't.
pub const SILENT: u32 = 3;
/// How often, in looks, an adapter that's asked nothing else is asked its
/// version to hear that it still answers: every thirty seconds, so an idle
/// controller isn't woken each look. One that's stopped is asked as often,
/// to hear it come back.
pub const HEARD_EVERY: u64 = 6;

/// Whether an adapter that's up answers: how many looks in a row it was
/// asked something and answered nothing, and the look it was last asked
/// anything at.
#[derive(Clone, Copy)]
struct Hearing {
    unanswered: u32,
    asked: u64,
}

impl Hearing {
    const fn stopped(self) -> bool {
        self.unanswered >= SILENT
    }

    /// Whether it's been long enough since it was last asked anything to
    /// ask it again.
    const fn due(self, look: u64) -> bool {
        look.saturating_sub(self.asked) >= HEARD_EVERY
    }

    /// Whether it's left unasked this look: one that's stopped is asked
    /// only now and then, to hear it come back, since each command it
    /// can't answer is a second waited and a line in the kernel's log.
    const fn hushed(self, look: u64) -> bool {
        self.stopped() && !self.due(look)
    }
}

/// The controller as asked within one look, noting of each adapter
/// whether it answered anything it was asked: true once it has, false
/// while all it was asked went unanswered. A socket that failed says
/// nothing of the controller, so it notes nothing.
pub struct Heeding<'a, C> {
    pub controller: &'a mut C,
    pub heard: HashMap<u16, bool>,
}

impl<C: Controller> Heeding<'_, C> {
    pub fn ask(&mut self, adapter: u16, opcode: u16, handle: u16) -> Option<Vec<u8>> {
        let said = self.controller.ask(adapter, opcode, handle);
        match said {
            Ok(_) | Err(Unanswered::Refused(_)) => {
                self.heard.insert(adapter, true);
            }
            Err(Unanswered::Silent) => {
                self.heard.entry(adapter).or_insert(false);
            }
            Err(Unanswered::Broken(_)) => {}
        }
        said.ok()
    }
}

/// Whether each adapter that's up answers, and the names of those that
/// stopped, as of the last look.
#[derive(Default)]
pub struct Hearings {
    by: HashMap<u16, Hearing>,
    stopped: Vec<String>,
}

impl Hearings {
    /// The adapters that are up but have stopped answering, by name.
    pub fn stopped(&self) -> Vec<String> {
        self.stopped.clone()
    }

    /// Whether an adapter is left unasked at `look`.
    pub fn hushed(&self, adapter: u16, look: u64) -> bool {
        self.by
            .get(&adapter)
            .is_some_and(|hearing| hearing.hushed(look))
    }

    /// Hears from what each adapter that's up was asked at `look` whether
    /// it answers, first asking its version of one that was asked nothing
    /// while it's due, or while it's yet to be heard to stop or answer
    /// again, and keeping what it says in `versions`. An adapter down or
    /// gone starts afresh, since bringing it up again starts its
    /// controller again.
    pub fn hear(
        &mut self,
        heeding: &mut Heeding<'_, impl Controller>,
        adapters: &[Adapter],
        look: u64,
        versions: &mut HashMap<u16, Version>,
    ) {
        self.by.retain(|id, _| {
            adapters
                .iter()
                .any(|adapter| adapter.id == *id && adapter.up)
        });
        for adapter in adapters.iter().filter(|adapter| adapter.up) {
            let hearing = *self.by.entry(adapter.id).or_insert(Hearing {
                unanswered: 0,
                asked: look,
            });
            let doubted = hearing.unanswered > 0 && !hearing.stopped();
            if !heeding.heard.contains_key(&adapter.id) && (doubted || hearing.due(look)) {
                let said = heeding.ask(adapter.id, reply::READ_LOCAL_VERSION, 0);
                if let Some(version) = said.as_deref().and_then(reply::version) {
                    versions.insert(adapter.id, version);
                }
            }
            let unanswered = match heeding.heard.get(&adapter.id) {
                Some(true) => 0,
                Some(false) => hearing.unanswered.saturating_add(1),
                None => continue,
            };
            self.by.insert(
                adapter.id,
                Hearing {
                    unanswered,
                    asked: look,
                },
            );
        }
        self.stopped = adapters
            .iter()
            .filter(|adapter| {
                self.by
                    .get(&adapter.id)
                    .is_some_and(|hearing| hearing.stopped())
            })
            .map(|adapter| adapter.name.clone())
            .collect();
    }
}

/// The adapters that have stopped answering, as `stopped.json` says them.
#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct Stopped {
    pub adapters: Vec<Halted>,
}

/// An adapter that stopped answering: its name, and the kernel's names
/// for the device it sits on and for that device's driver's folder in
/// sysfs, by which the driver is let go of the device and given it back,
/// starting the controller again; null where sysfs doesn't say, as of a
/// virtual adapter.
#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct Halted {
    name: String,
    device: Option<String>,
    driver: Option<String>,
}

/// What an adapter sits on, from sysfs under `sysfs`: its `device` links
/// to the device by its name, as `0000:00:14.7` for a PCI chip or
/// `1-10:1.0` for a USB one's interface, and that device's `driver` to
/// its driver's folder, as `/sys/bus/pci/drivers/btintel_pcie`.
pub fn sits_on(sysfs: &Path, name: &str) -> Halted {
    let device = sysfs.join("class/bluetooth").join(name).join("device");
    let named = |path: PathBuf| path.to_str().map(String::from);
    Halted {
        name: name.into(),
        device: fs::read_link(&device)
            .ok()
            .and_then(|link| link.file_name().map(PathBuf::from))
            .and_then(named),
        driver: fs::canonicalize(device.join("driver")).ok().and_then(named),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Watch;
    use crate::tests::{StandIn, mouse, radio, seconds};

    /// Ticks the watch over `looks` more looks, five seconds apart from
    /// look `from`, saying after each which adapters have stopped.
    fn watched(
        watch: &mut Watch,
        controller: &mut StandIn,
        from: u64,
        looks: u64,
    ) -> Vec<Vec<String>> {
        (from..from + looks)
            .map(|step| {
                watch.tick(controller, step * 5, seconds(step * 5));
                watch.hearings.stopped()
            })
            .collect()
    }

    #[test]
    fn a_controller_answering_nothing_has_stopped_after_three_looks() {
        let mut watch = Watch::default();
        let mut controller = StandIn {
            adapters: vec![radio(0, 0)],
            listed: vec![(0, mouse())],
            ..StandIn::default()
        };
        watched(&mut watch, &mut controller, 0, 4);
        controller.silent = true;
        let said = watched(&mut watch, &mut controller, 4, 3);
        assert_eq!(
            said,
            vec![vec![], vec![], vec!["hci0".to_string()]],
            "three looks unanswered weren't said, or fewer were"
        );
        controller.asked.clear();
        watched(&mut watch, &mut controller, 7, HEARD_EVERY - 1);
        assert!(
            controller.asked.is_empty(),
            "one that stopped was asked its link's commands each look"
        );
        controller.silent = false;
        let back = watched(&mut watch, &mut controller, 7 + HEARD_EVERY - 1, 1);
        assert_eq!(
            back,
            vec![Vec::<String>::new()],
            "its answering again went unheard"
        );
    }

    #[test]
    fn an_idle_controller_is_asked_now_and_then() {
        let mut watch = Watch::default();
        let version = vec![0x00, 0x0e, 0x00, 0x01, 0x0e, 0x02, 0x00, 0x00, 0x01];
        let mut controller = StandIn {
            adapters: vec![radio(0, 0)],
            replies: HashMap::from([((reply::READ_LOCAL_VERSION, 0), version)]),
            ..StandIn::default()
        };
        watched(&mut watch, &mut controller, 0, 1 + 6 * HEARD_EVERY);
        assert_eq!(
            controller.asked.len(),
            7,
            "an idle controller wasn't asked its version, then once every thirty seconds"
        );
        controller.silent = true;
        let said = watched(&mut watch, &mut controller, 1 + 6 * HEARD_EVERY, 9);
        assert_eq!(
            said.iter().position(|stopped| !stopped.is_empty()),
            Some(7),
            "an idle controller that stopped answering wasn't found within 45 s"
        );
    }

    #[test]
    fn an_adapter_down_or_gone_has_not_stopped() {
        let mut watch = Watch::default();
        let mut controller = StandIn {
            adapters: vec![radio(0, 0)],
            listed: vec![(0, mouse())],
            silent: true,
            ..StandIn::default()
        };
        watched(&mut watch, &mut controller, 0, 3);
        assert_eq!(watch.hearings.stopped(), vec!["hci0".to_string()]);
        controller.adapters[0].up = false;
        controller.listed.clear();
        assert_eq!(
            watched(&mut watch, &mut controller, 3, 1),
            vec![Vec::<String>::new()],
            "an adapter taken down was still said to have stopped"
        );
        controller.adapters.clear();
        assert_eq!(
            watched(&mut watch, &mut controller, 4, 1),
            vec![Vec::<String>::new()]
        );
    }

    #[test]
    fn what_an_adapter_sits_on_is_read_from_sysfs() {
        use std::os::unix::fs::symlink;
        let sysfs = std::env::temp_dir().join(format!("bluetooth-sysfs-{}", std::process::id()));
        let chip = sysfs.join("devices/pci0000:00/0000:00:14.7");
        fs::create_dir_all(chip.join("bluetooth/hci0")).unwrap();
        fs::create_dir_all(sysfs.join("bus/pci/drivers/btintel_pcie")).unwrap();
        fs::create_dir_all(sysfs.join("class/bluetooth")).unwrap();
        symlink(
            "../../devices/pci0000:00/0000:00:14.7/bluetooth/hci0",
            sysfs.join("class/bluetooth/hci0"),
        )
        .unwrap();
        symlink("../../../0000:00:14.7", chip.join("bluetooth/hci0/device")).unwrap();
        symlink("../../../bus/pci/drivers/btintel_pcie", chip.join("driver")).unwrap();
        let driver = fs::canonicalize(sysfs.join("bus/pci/drivers/btintel_pcie")).unwrap();
        assert_eq!(
            sits_on(&sysfs, "hci0"),
            Halted {
                name: "hci0".into(),
                device: Some("0000:00:14.7".into()),
                driver: driver.to_str().map(String::from),
            }
        );
        assert_eq!(
            sits_on(&sysfs, "hci1"),
            Halted {
                name: "hci1".into(),
                device: None,
                driver: None,
            },
            "an adapter sysfs doesn't know sat on something"
        );
        fs::remove_dir_all(&sysfs).unwrap();
    }
}
