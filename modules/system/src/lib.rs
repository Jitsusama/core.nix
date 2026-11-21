//! system: how the whole system stands, its four parts at once, each in its
//! own words (docs/glossary.md's Context Map, The System).
//!
//! The parts are the machines of the realm, the mycelium that carries the
//! sessions between them, the home network, and the code. Each already has
//! an owner that reads it: `mycelium machines`, `mycelium health`, `net`
//! and `repos`. This crate reads none of their sources itself. It takes
//! each owner's own answer and judges it, so a fix in an owner shows here
//! at once and the two can never disagree. A part whose answer can't be
//! had is said as not read, with why, and the others still stand, unless
//! its owner answered within the last few minutes, when that answer stands
//! and says when it was read (kept.rs). The one
//! exception is the kernel's word on an error the firmware kept before this
//! machine's boot, which no owner's command says yet, read from this
//! boot's journal with the time since the boot from /proc/uptime
//! (power.rs).
//!
//! What the views read is one record, `system.view/1` (record.rs): the
//! command's screens here, and next the site, the skill and the launcher's
//! home rows. Each thing in it carries the address of its home on the
//! local sites, as the site that is home to it says (homes.rs).

pub mod homes;
pub mod kept;
pub mod parts;
pub mod power;
pub mod record;
pub mod words;
