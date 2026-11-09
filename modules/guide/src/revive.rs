//! Reviving a Mac with Apple silicon from the stick, on any laptop it
//! boots: the Mac's firmware and macOS put back from Apple's restore image,
//! which the stick carries in a partition of its own (stick.nix), with the
//! Mac waiting in DFU mode on a cable, as Apple's own revive from another
//! Mac does. It updates rather than erases, so everything on the Mac's disk
//! stays. Apple signs every restore for the one Mac it's for, so the stick
//! joins a network first, and asks Apple for that before it changes
//! anything, which is all a practice run does.

use serde::Deserialize;

use crate::observe::disks;
use crate::{
    Answer, Answers, Ask, Fault, Given, Machine, Place, Program, Secret, Standing, Step, Tool,
};

/// Where the restore image's partition is mounted, only for reading.
pub const RESTORE: &str = "/run/guide/restore";

/// Where idevicerestore keeps what it fetches from Apple, and its log.
pub const CACHE: &str = "/run/guide/revive";

/// Where idevicerestore writes everything it does, which it otherwise
/// writes wherever it's run from.
const LOG: &str = "--logfile=/run/guide/revive/restore.log";

/// The stick's usbmuxd, which a revive starts and stops, and where
/// idevicerestore finds it.
const USBMUXD: &str = "guide-usbmuxd.service";
const USBMUXD_SOCKET: &str = "UNIX:/run/guide/usbmuxd.sock";

/// wifid's socket, which the stick's Wi-Fi is asked through.
const WIFI: &str = "/run/wifid/io.jitsusama.Wifi";

/// How a restore image's partition is named: for the file it holds, less
/// `_Restore.ipsw`, as Apple names it.
const PREFIX: &str = "UniversalMac_";

/// A restore image the stick carries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Restore {
    /// Its partition, as lsblk names it.
    pub device: String,
    /// Its partition's name, as `UniversalMac_27.2_26B123`.
    pub label: String,
    /// Whether it's mounted where the guide reads it.
    pub mounted: bool,
}

impl Restore {
    /// The zip, as idevicerestore is given it.
    pub fn file(&self) -> String {
        format!("{RESTORE}/{}_Restore.ipsw", self.label)
    }

    /// Which macOS it holds, as a person reads it: `macOS 27.2 (26B123)`.
    pub fn version(&self) -> String {
        let named = self.label.trim_start_matches(PREFIX);
        match named.split_once('_') {
            Some((version, build)) => format!("macOS {version} ({build})"),
            None => format!("macOS {named}"),
        }
    }
}

/// Every restore image on the disks here, in the order lsblk lists them.
pub fn restores(machine: &mut dyn Machine) -> Result<Vec<Restore>, Fault> {
    Ok(disks::observe(machine)?
        .into_iter()
        .flat_map(|disk| disk.partitions)
        .filter(|partition| partition.filesystem.as_deref() == Some("erofs"))
        .filter_map(|partition| {
            let label = partition.label.filter(|label| label.starts_with(PREFIX))?;
            Some(Restore {
                mounted: partition.mounted.iter().any(|at| at == RESTORE),
                device: partition.path,
                label,
            })
        })
        .collect())
}

/// The restore image mounted for reading, which the steps after this one
/// take.
fn mounted(machine: &mut dyn Machine, step: &'static str) -> Result<Restore, Fault> {
    restores(machine)?
        .into_iter()
        .find(|restore| restore.mounted)
        .ok_or(Fault::Blocked {
            step,
            why: format!("no restore image is mounted at {RESTORE}"),
        })
}

pub struct FindRestore;

impl Step for FindRestore {
    fn id(&self) -> &'static str {
        "find-restore"
    }

    fn place(&self) -> Place {
        Place::Stick
    }

    fn title(&self) -> &'static str {
        "Find the macOS this stick carries"
    }

    fn observe(&self, machine: &mut dyn Machine) -> Result<Standing, Fault> {
        let found = restores(machine)?;
        if let Some(restore) = found.iter().find(|restore| restore.mounted) {
            return Ok(Standing::Done {
                evidence: format!("the stick carries {}", restore.version()),
            });
        }
        Ok(match found.len() {
            0 => Standing::Blocked {
                why: "this stick carries no macOS to put back on a Mac".into(),
                remedy: "write the stick again from a machine whose stick carries one".into(),
            },
            1 => Standing::Needed { asks: vec![] },
            _ => Standing::Needed {
                asks: vec![Ask::Choose {
                    id: "restore",
                    question: "Which macOS should go back on the Mac?".into(),
                    choices: found.iter().map(Restore::version).collect(),
                }],
            },
        })
    }

    fn run(&self, machine: &mut dyn Machine, answers: &Answers) -> Result<(), Fault> {
        let found = restores(machine)?;
        let at = match answers.get("restore") {
            Some(Answer::Chosen(at)) => *at,
            _ => 0,
        };
        let restore = found.get(at).ok_or(Fault::Blocked {
            step: self.id(),
            why: "that restore image isn't on the stick any more".into(),
        })?;
        machine
            .run(
                Tool::Mount,
                &["-o", "ro,X-mount.mkdir", &restore.device, RESTORE],
            )?
            .ok()?;
        Ok(())
    }
}

/// What wifid says of the Wi-Fi and the cable, as much as the guide reads.
#[derive(Debug, Deserialize)]
pub struct Wifi {
    pub cards: Vec<Card>,
    pub saved: Vec<Named>,
    /// The strongest first.
    pub nearby: Vec<Nearby>,
    pub radio: bool,
    pub wired: bool,
}

#[derive(Debug, Deserialize)]
pub struct Card {
    pub name: String,
    pub network: Option<String>,
    pub reach: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Named {
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct Nearby {
    pub name: String,
    /// `open`, `personal` or `enterprise`.
    pub security: String,
    pub strength: u8,
}

impl Wifi {
    /// What reaches the internet, said as evidence, if anything does.
    pub fn online(&self) -> Option<String> {
        if self.wired {
            return Some("a cable reaches the internet".into());
        }
        self.cards
            .iter()
            .find(|card| card.reach.as_deref() == Some("online"))
            .map(|card| match &card.network {
                Some(network) => {
                    format!("{} is on {network}, which reaches the internet", card.name)
                }
                None => format!("{} reaches the internet", card.name),
            })
    }

    /// The networks in reach that need no account, which is all the stick
    /// asks for, strongest first.
    pub fn joinable(&self) -> Vec<&Nearby> {
        self.nearby
            .iter()
            .filter(|nearby| nearby.security != "enterprise")
            .collect()
    }
}

/// What wifid says now, or why it can't be asked.
pub fn wifi(machine: &mut dyn Machine) -> Result<Result<Wifi, Standing>, Fault> {
    let ran = machine.run(
        Tool::Varlinkctl,
        &["call", WIFI, "io.jitsusama.Wifi.Describe", "{}"],
    )?;
    if ran.status != 0 {
        return Ok(Err(Standing::Blocked {
            why: "the stick's Wi-Fi isn't answering".into(),
            remedy: "plug a network cable into this laptop, then start the stick again".into(),
        }));
    }
    let said = ran.ok()?;
    serde_json::from_str(&said)
        .map(Ok)
        .map_err(|why| Fault::unreadable("what wifid said of the Wi-Fi", why))
}

/// How a network in reach reads in a list: its name, how it's joined and
/// how strong it is.
fn described(nearby: &Nearby) -> String {
    let joined = match nearby.security.as_str() {
        "open" => "open",
        _ => "with a password",
    };
    let bars = match nearby.strength {
        1 => "1 bar".to_string(),
        bars => format!("{bars} bars"),
    };
    format!("{}, {joined}, {bars}", nearby.name)
}

pub struct ChooseNetwork;

impl Step for ChooseNetwork {
    fn id(&self) -> &'static str {
        "choose-network"
    }

    fn place(&self) -> Place {
        Place::Stick
    }

    fn title(&self) -> &'static str {
        "Choose a network to ask Apple through"
    }

    fn observe(&self, machine: &mut dyn Machine) -> Result<Standing, Fault> {
        let wifi = match wifi(machine)? {
            Ok(wifi) => wifi,
            Err(standing) => return Ok(standing),
        };
        if let Some(evidence) = wifi.online() {
            return Ok(Standing::Done { evidence });
        }
        let joinable = wifi.joinable();
        // One chosen that's gone out of reach is chosen again.
        if let Some(name) = machine.journal().said(self.id()).last()
            && (joinable.iter().any(|nearby| nearby.name == *name)
                || wifi.saved.iter().any(|saved| saved.name == *name))
        {
            return Ok(Standing::Done {
                evidence: format!("it joins {name}"),
            });
        }
        if !wifi.radio || joinable.is_empty() {
            let why = if !wifi.radio {
                "this laptop's Wi-Fi is switched off"
            } else if wifi.cards.is_empty() {
                "this laptop has no Wi-Fi the stick can use"
            } else {
                "no network in reach can be joined without an account"
            };
            return Ok(Standing::Blocked {
                why: why.into(),
                remedy: "plug a network cable into this laptop".into(),
            });
        }
        Ok(Standing::Needed {
            asks: vec![Ask::Choose {
                id: "network",
                question: "Which network should it join? Apple is asked through it to sign the Mac's restore. The strongest is first.".into(),
                choices: joinable.into_iter().map(described).collect(),
            }],
        })
    }

    fn run(&self, machine: &mut dyn Machine, answers: &Answers) -> Result<(), Fault> {
        let Some(Answer::Chosen(at)) = answers.get("network") else {
            return Err(Fault::Unanswered {
                step: self.id(),
                ask: "network",
            });
        };
        let wifi = wifi(machine)?.map_err(|standing| Fault::Blocked {
            step: self.id(),
            why: standing.to_string(),
        })?;
        let name = wifi
            .joinable()
            .get(*at)
            .map(|nearby| nearby.name.clone())
            .ok_or(Fault::Blocked {
                step: self.id(),
                why: "that network isn't in reach any more".into(),
            })?;
        machine.journal().record(self.id(), &name)
    }
}

pub struct JoinNetwork;

impl JoinNetwork {
    /// The network chosen in the step before.
    fn chosen(machine: &mut dyn Machine) -> Option<String> {
        machine
            .journal()
            .said(ChooseNetwork.id())
            .last()
            .map(ToString::to_string)
    }
}

impl Step for JoinNetwork {
    fn id(&self) -> &'static str {
        "join-network"
    }

    fn place(&self) -> Place {
        Place::Stick
    }

    fn title(&self) -> &'static str {
        "Join the network"
    }

    fn observe(&self, machine: &mut dyn Machine) -> Result<Standing, Fault> {
        let wifi = match wifi(machine)? {
            Ok(wifi) => wifi,
            Err(standing) => return Ok(standing),
        };
        if let Some(evidence) = wifi.online() {
            return Ok(Standing::Done { evidence });
        }
        let Some(name) = Self::chosen(machine) else {
            return Ok(Standing::Blocked {
                why: "no network was chosen".into(),
                remedy: "start the revive again".into(),
            });
        };
        let open = wifi
            .nearby
            .iter()
            .any(|nearby| nearby.name == name && nearby.security == "open");
        let saved = wifi.saved.iter().any(|saved| saved.name == name);
        Ok(Standing::Needed {
            asks: if open || saved {
                vec![]
            } else {
                vec![Ask::Secret {
                    id: "password",
                    name: format!("the password for {name}"),
                }]
            },
        })
    }

    fn run(&self, machine: &mut dyn Machine, answers: &Answers) -> Result<(), Fault> {
        let name = Self::chosen(machine).ok_or(Fault::Blocked {
            step: self.id(),
            why: "no network was chosen".into(),
        })?;
        let mut asked = serde_json::json!({ "name": name });
        if let Some(Answer::Secret(password)) = answers.get("password") {
            asked["password"] = password.expose().into();
        }
        // The password goes on varlinkctl's standard input, which it reads
        // the call's parameters from when none are given.
        let asked = Secret::new(asked.to_string());
        let ran = machine.run_given(
            Program::Tool(Tool::Varlinkctl),
            &["call", WIFI, "io.jitsusama.Wifi.Join"],
            Given {
                stdin: Some(&asked),
                ..Given::default()
            },
        )?;
        if ran.status != 0 {
            let said = String::from_utf8_lossy(&ran.stderr);
            let why = [
                ("WrongPassword", "that password didn't work"),
                ("NotNearby", "the network isn't in reach any more"),
                ("TurnedAway", "the network turned this laptop away"),
                ("TimedOut", "the network didn't let this laptop on in time"),
            ]
            .into_iter()
            .find(|(error, _)| said.contains(&format!("io.jitsusama.Wifi.{error}")));
            return Err(match why {
                Some((_, why)) => Fault::Blocked {
                    step: self.id(),
                    why: format!("{why} for {name}"),
                },
                None => ran.refusal(),
            });
        }
        let said = ran.ok()?;
        if !said.contains("\"online\"") {
            return Err(Fault::Blocked {
                step: self.id(),
                why: format!(
                    "{name} doesn't reach the internet, so Apple can't be asked through it"
                ),
            });
        }
        Ok(())
    }
}

/// A Mac waiting on a cable, as irecovery describes it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mac {
    pub ecid: String,
    /// Apple's name for the model, as `Mac14,2`.
    pub product: String,
    /// What a person calls it, as `MacBook Air (M2, 2022)`.
    pub name: String,
    /// `DFU`, `Recovery` and so on.
    pub mode: String,
}

impl Mac {
    pub fn parse(said: &str) -> Option<Mac> {
        let field = |key: &str| {
            said.lines()
                .find_map(|line| line.strip_prefix(key)?.strip_prefix(": "))
                .map(|value| value.trim().to_string())
        };
        Some(Mac {
            ecid: field("ECID")?,
            product: field("PRODUCT")?,
            name: field("NAME").unwrap_or_default(),
            mode: field("MODE")?,
        })
    }

    /// Whether idevicerestore can put a system on it as it is.
    pub fn waiting(&self) -> bool {
        self.mode == "DFU" || self.mode == "Recovery"
    }

    pub fn described(&self) -> String {
        if self.name.is_empty() {
            format!("a {} waits in {} mode", self.product, self.mode)
        } else {
            format!(
                "a {}, {}, waits in {} mode",
                self.name, self.product, self.mode
            )
        }
    }
}

/// The Mac on a cable to this laptop now, if one waits there.
pub fn mac(machine: &mut dyn Machine) -> Result<Option<Mac>, Fault> {
    let ran = machine.run(Tool::Irecovery, &["-q"])?;
    if ran.status != 0 {
        return Ok(None);
    }
    Ok(Mac::parse(&ran.ok()?).filter(Mac::waiting))
}

/// How a Mac laptop with Apple silicon is put in DFU mode, from Apple's
/// own steps (support.apple.com/108900 and /120694).
const INTO_DFU: &str = "Plug the Mac into power by MagSafe, if it has it, and unplug everything else from it. Then plug a USB cable that carries data, not a Thunderbolt 3 one, from this laptop into the Mac's DFU port: on most Mac laptops, the USB-C port on the left side nearest the back; support.apple.com/120694 names it for each model. Hold the Mac's power button until it turns off. Press and release it, then at once hold left Control, left Option, right Shift and the power button together for about 10 seconds. Let go of all but the power button, and hold it about 10 seconds more. The Mac's screen stays dark.";

pub struct FindMac;

impl FindMac {
    /// The Mac found in this step, by its ECID.
    pub fn found(machine: &mut dyn Machine) -> Option<String> {
        machine
            .journal()
            .said(FindMac.id())
            .last()
            .map(ToString::to_string)
    }
}

impl Step for FindMac {
    fn id(&self) -> &'static str {
        "find-mac"
    }

    fn place(&self) -> Place {
        Place::Stick
    }

    fn title(&self) -> &'static str {
        "Find the Mac in DFU mode"
    }

    fn observe(&self, machine: &mut dyn Machine) -> Result<Standing, Fault> {
        let waiting = mac(machine)?;
        if let Some(ecid) = Self::found(machine) {
            if let Some(mac) = waiting.as_ref().filter(|mac| mac.ecid == ecid) {
                return Ok(Standing::Done {
                    evidence: mac.described(),
                });
            }
            // Revived, it starts up into macOS, so it's no longer waiting,
            // and the steps after this stay done.
            if machine
                .journal()
                .said(Revive.id())
                .iter()
                .any(|said| said.starts_with(&ecid))
            {
                return Ok(Standing::Done {
                    evidence: "the Mac was found in DFU mode".into(),
                });
            }
        }
        Ok(Standing::Needed {
            asks: vec![Ask::Act {
                id: "waiting",
                what: match waiting {
                    Some(mac) => {
                        let said = mac.described();
                        let mut chars = said.chars();
                        let first = chars
                            .next()
                            .map(|first| first.to_uppercase().collect::<String>());
                        format!("{}{}.", first.unwrap_or_default(), chars.as_str())
                    }
                    None => INTO_DFU.into(),
                },
            }],
        })
    }

    fn run(&self, machine: &mut dyn Machine, _answers: &Answers) -> Result<(), Fault> {
        let found = mac(machine)?.ok_or(Fault::Blocked {
            step: self.id(),
            why: "no Mac waits in DFU mode on a cable to this laptop".into(),
        })?;
        machine.journal().record(self.id(), &found.ecid)
    }
}

/// What the journal says of a Mac and a restore: its ECID and the image.
fn fact(ecid: &str, restore: &Restore) -> String {
    format!("{ecid} {}", restore.label)
}

/// The Mac found, waiting still, or why the step can't go on.
fn waiting(machine: &mut dyn Machine) -> Result<Result<String, Standing>, Fault> {
    let Some(ecid) = FindMac::found(machine) else {
        return Ok(Err(Standing::Blocked {
            why: "no Mac was found".into(),
            remedy: "start the revive again".into(),
        }));
    };
    Ok(match mac(machine)? {
        Some(mac) if mac.ecid == ecid => Ok(ecid),
        _ => Err(Standing::Blocked {
            why: "the Mac isn't waiting in DFU mode any more".into(),
            remedy: INTO_DFU.into(),
        }),
    })
}

/// idevicerestore's arguments for this Mac and restore, ahead of its own.
fn restoring<'a>(ecid: &'a str, file: &'a str, own: &[&'a str]) -> Vec<&'a str> {
    let mut args = own.to_vec();
    // No questions on a terminal it doesn't hold: the guide asked them.
    args.extend(["-y", "-i", ecid, "-C", CACHE]);
    args.extend([LOG, file]);
    args
}

pub struct CheckSigning;

impl Step for CheckSigning {
    fn id(&self) -> &'static str {
        "check-signing"
    }

    fn place(&self) -> Place {
        Place::Stick
    }

    fn title(&self) -> &'static str {
        "Ask Apple to sign the restore for this Mac"
    }

    fn observe(&self, machine: &mut dyn Machine) -> Result<Standing, Fault> {
        let restore = mounted(machine, self.id())?;
        if let Some(ecid) = FindMac::found(machine)
            && machine.journal().knows(self.id(), &fact(&ecid, &restore))
        {
            return Ok(Standing::Done {
                evidence: format!("Apple signs {} for this Mac", restore.version()),
            });
        }
        Ok(match waiting(machine)? {
            Ok(_) => Standing::Needed { asks: vec![] },
            Err(standing) => standing,
        })
    }

    fn run(&self, machine: &mut dyn Machine, _answers: &Answers) -> Result<(), Fault> {
        let restore = mounted(machine, self.id())?;
        let ecid = waiting(machine)?.map_err(|standing| Fault::Blocked {
            step: self.id(),
            why: standing.to_string(),
        })?;
        machine.run(Tool::Mkdir, &["-p", CACHE])?.ok()?;
        let file = restore.file();
        // Only asks Apple's signing server, saving its answer, and changes
        // nothing on the Mac.
        let ran = machine.run(Tool::Idevicerestore, &restoring(&ecid, &file, &["-t"]))?;
        if ran.status != 0 {
            let said = [&ran.stdout[..], &ran.stderr[..]].concat();
            if String::from_utf8_lossy(&said).contains("Unable to get SHSH blobs") {
                return Err(Fault::Blocked {
                    step: self.id(),
                    why: format!(
                        "Apple no longer signs {} for this Mac; write the stick again with the newest macOS",
                        restore.version()
                    ),
                });
            }
            return Err(ran.refusal());
        }
        machine.journal().record(self.id(), &fact(&ecid, &restore))
    }
}

pub struct Revive;

impl Step for Revive {
    fn id(&self) -> &'static str {
        "revive-mac"
    }

    fn place(&self) -> Place {
        Place::Stick
    }

    fn title(&self) -> &'static str {
        "Put macOS back on the Mac"
    }

    fn observe(&self, machine: &mut dyn Machine) -> Result<Standing, Fault> {
        let restore = mounted(machine, self.id())?;
        if let Some(ecid) = FindMac::found(machine)
            && machine.journal().knows(self.id(), &fact(&ecid, &restore))
        {
            return Ok(Standing::Done {
                evidence: format!(
                    "{} is back on the Mac, which starts up by itself",
                    restore.version()
                ),
            });
        }
        Ok(match waiting(machine)? {
            Ok(_) => Standing::Needed {
                asks: vec![Ask::Confirm {
                    id: "revive",
                    what: format!(
                        "The Mac's firmware and {} go back on it from the stick, as an update that keeps the data on its disk. It takes about half an hour: keep both plugged in until the Mac starts up again.",
                        restore.version()
                    ),
                    verb: "revive it",
                }],
            },
            Err(standing) => standing,
        })
    }

    fn run(&self, machine: &mut dyn Machine, answers: &Answers) -> Result<(), Fault> {
        if !matches!(answers.get("revive"), Some(Answer::Done)) {
            return Err(Fault::Unanswered {
                step: self.id(),
                ask: "revive",
            });
        }
        let restore = mounted(machine, self.id())?;
        let ecid = waiting(machine)?.map_err(|standing| Fault::Blocked {
            step: self.id(),
            why: standing.to_string(),
        })?;
        machine.run(Tool::Mkdir, &["-p", CACHE])?.ok()?;
        let file = restore.file();
        // What idevicerestore reads of the image is how far it has got.
        machine
            .meter()
            .stage_by_reads("Putting macOS back on the Mac", None);
        // The Mac starts again part way into a system of Apple's, reached
        // through the stick's own usbmuxd, which skips pairing (stick.nix).
        machine.run(Tool::Systemctl, &["start", USBMUXD])?.ok()?;
        // An update, never `-e`, so the Mac's disk is kept.
        let revived = machine.run_given(
            Program::Tool(Tool::Idevicerestore),
            &restoring(&ecid, &file, &[]),
            Given {
                plain: vec![("USBMUXD_SOCKET_ADDRESS", USBMUXD_SOCKET)],
                ..Given::default()
            },
        );
        machine.run(Tool::Systemctl, &["stop", USBMUXD])?.ok()?;
        revived?.ok()?;
        machine.journal().record(self.id(), &fact(&ecid, &restore))
    }
}

/// Every step of a revive, in order.
pub fn steps() -> Vec<Box<dyn Step>> {
    vec![
        Box::new(FindRestore),
        Box::new(ChooseNetwork),
        Box::new(JoinNetwork),
        Box::new(FindMac),
        Box::new(CheckSigning),
        Box::new(Revive),
    ]
}
