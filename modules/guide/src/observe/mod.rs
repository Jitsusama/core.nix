//! What the guide reads off a machine, each from the tool or file that says
//! it, parsed into what steps decide by. Every parser is tested against
//! what the tools printed on real machines (`fixtures/`), never against
//! what they're expected to print.

pub mod disks;
pub mod efi;
pub mod entries;
pub mod fido2;
pub mod sbctl;
pub mod tpm;
