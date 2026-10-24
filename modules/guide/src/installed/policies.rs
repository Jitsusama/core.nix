//! The policies earlier installs left in the TPM. systemd-pcrlock keeps
//! each disk's policy in an NV index of its own, picked at random when the
//! machine has none, and a disk wiped for a reinstall takes the file naming
//! it along, so the index stays, holding a little of the TPM's NV space for
//! good. Every reinstall leaves one more, until the TPM has no room for the
//! next install's.
//!
//! Only an index systemd-pcrlock would have defined, by its range and
//! shape (`tpm::NvPublic::is_pcrlocks`), and only once this install's own
//! is the one the TPM holds, is taken for an earlier install's. Another
//! system on another disk of the same machine, sealed with systemd-pcrlock
//! too, would lose its policy, and open with its recovery key; core
//! installs a machine whole, so that's none.

use super::{as_root, policy_held, policy_index};
use crate::observe::tpm::{self, NvPublic};
use crate::{Answers, Fault, Machine, Place, Program, Standing, Step, Tool};

pub struct FreeOldPolicies;

/// The policy indices the TPM holds other than this install's, or none
/// to say when the TPM doesn't hold this install's, since then which is
/// whose can't be told.
fn left_behind(machine: &mut dyn Machine) -> Result<Option<Vec<u32>>, Fault> {
    let Some(own) = policy_index(machine)? else {
        return Ok(None);
    };
    if !policy_held(machine)? {
        return Ok(None);
    }
    let listed = machine
        .run_given(Program::Tool(Tool::Tpm2Getcap), tpm::NV_INDICES, as_root())?
        .ok()?;
    let mut left = Vec::new();
    for index in tpm::nv_indices(&listed)? {
        if index == own || !tpm::UNASSIGNED.contains(&index) {
            continue;
        }
        let args = NvPublic::args(index);
        let args: Vec<&str> = args.iter().map(String::as_str).collect();
        let public = machine
            .run_given(Program::Tool(Tool::Tpm2Nvreadpublic), &args, as_root())?
            .ok()?;
        if NvPublic::parse(&public)?.is_pcrlocks() {
            left.push(index);
        }
    }
    Ok(Some(left))
}

impl Step for FreeOldPolicies {
    fn id(&self) -> &'static str {
        "free-old-policies"
    }

    fn place(&self) -> Place {
        Place::Installed
    }

    fn title(&self) -> &'static str {
        "Free the TPM of earlier installs' policies"
    }

    fn observe(&self, machine: &mut dyn Machine) -> Result<Standing, Fault> {
        Ok(match left_behind(machine)? {
            None => Standing::Done {
                evidence: "the TPM doesn't hold this install's policy, so none is taken for an earlier install's".into(),
            },
            Some(left) if left.is_empty() => Standing::Done {
                evidence: "the TPM holds this install's systemd-pcrlock policy and no earlier one".into(),
            },
            Some(_) => Standing::Needed { asks: vec![] },
        })
    }

    fn run(&self, machine: &mut dyn Machine, _answers: &Answers) -> Result<(), Fault> {
        for index in left_behind(machine)?.unwrap_or_default() {
            let index = format!("{index:#x}");
            let freed = machine.run_given(
                Program::Tool(Tool::Tpm2Nvundefine),
                &["-T", "device:/dev/tpmrm0", "-C", "o", &index],
                as_root(),
            )?;
            if freed.status != 0 {
                return Err(freed.refusal());
            }
        }
        Ok(())
    }
}
