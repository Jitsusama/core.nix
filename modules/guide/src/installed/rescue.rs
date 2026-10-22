//! The rescue bundle the install made, handed on: copied into the person's
//! home, where a browser or a file manager finds it, then kept in
//! 1Password and on their other machines. Kept there, a stick for this
//! machine carries its Secure Boot keys even when the machine itself is
//! lost, and that's how someone with one laptop gets it back.

use std::path::PathBuf;

use crate::rescue::{self, Bundle};
use crate::{Answers, Ask, Fault, Machine, Place, Standing, Step};

/// The bundle as the install left it, when there's one.
fn kept(machine: &mut dyn Machine) -> Result<Option<(Vec<u8>, Bundle)>, Fault> {
    let Some(bytes) = machine.read(rescue::KEPT)? else {
        return Ok(None);
    };
    let bundle = Bundle::read(&bytes)?;
    Ok(Some((bytes, bundle)))
}

fn none_here() -> Standing {
    Standing::Done {
        evidence: format!(
            "there's no rescue bundle at {}, so `guide seal-keys` makes one",
            rescue::KEPT
        ),
    }
}

/// Where the person finds their copy, named for the machine.
fn copy(home: &std::path::Path, bundle: &Bundle) -> PathBuf {
    home.join(format!("{}-rescue-bundle.json", bundle.machine()))
}

/// The bundle copied into the person's home, done while the copy there
/// is the one the install made.
pub struct CopyRescueBundle {
    pub home: PathBuf,
}

impl Step for CopyRescueBundle {
    fn id(&self) -> &'static str {
        "copy-rescue-bundle"
    }

    fn place(&self) -> Place {
        Place::Installed
    }

    fn title(&self) -> &'static str {
        "Copy the rescue bundle where you'll find it"
    }

    fn observe(&self, machine: &mut dyn Machine) -> Result<Standing, Fault> {
        let Some((bytes, bundle)) = kept(machine)? else {
            return Ok(none_here());
        };
        let path = copy(&self.home, &bundle);
        Ok(
            if machine.read(&path.display().to_string())?.as_deref() == Some(&bytes[..]) {
                Standing::Done {
                    evidence: format!("{} holds the rescue bundle", path.display()),
                }
            } else {
                Standing::Needed { asks: vec![] }
            },
        )
    }

    fn run(&self, machine: &mut dyn Machine, _answers: &Answers) -> Result<(), Fault> {
        let Some((bytes, bundle)) = kept(machine)? else {
            return Ok(());
        };
        machine.write(&copy(&self.home, &bundle).display().to_string(), &bytes)
    }
}

/// The person keeps the bundle away from the machine. Done once they've
/// said so for this bundle; a reinstall makes another, which asks again.
pub struct KeepRescueBundle {
    pub home: PathBuf,
}

impl Step for KeepRescueBundle {
    fn id(&self) -> &'static str {
        "keep-rescue-bundle"
    }

    fn place(&self) -> Place {
        Place::Installed
    }

    fn title(&self) -> &'static str {
        "Keep the rescue bundle away from the machine"
    }

    fn observe(&self, machine: &mut dyn Machine) -> Result<Standing, Fault> {
        let Some((bytes, bundle)) = kept(machine)? else {
            return Ok(none_here());
        };
        if machine
            .journal()
            .knows(self.id(), &rescue::fingerprint(&bytes))
        {
            return Ok(Standing::Done {
                evidence: "the rescue bundle is kept in 1Password and on your other machines"
                    .into(),
            });
        }
        Ok(Standing::Needed {
            asks: vec![Ask::Act {
                id: "kept",
                what: format!(
                    "Put {} in 1Password, and copy it to your other machines. It's sealed, so it's safe anywhere, and with it a stick for {} carries its Secure Boot keys.",
                    copy(&self.home, &bundle).display(),
                    bundle.machine()
                ),
            }],
        })
    }

    fn run(&self, machine: &mut dyn Machine, _answers: &Answers) -> Result<(), Fault> {
        let Some((bytes, _)) = kept(machine)? else {
            return Ok(());
        };
        machine
            .journal()
            .record(self.id(), &rescue::fingerprint(&bytes))
    }
}
