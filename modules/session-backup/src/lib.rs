//! session-backup: each settled pi session log, copied once into a Google
//! Drive folder, packed with zstd and sealed with age.
//!
//! The logs are where an account's sessions can be mined from later, and
//! they grow by gigabytes a month, past what a git backup takes. So each one
//! is copied as an object of its own once it has settled, which is when it
//! has had no writes for a while (`settledHours`, two days by default):
//!
//!   <host>/<folder>/<log>.<last write>.jsonl.zst.age
//!
//! named for the machine, the folder pi keeps it in, the log's own name and
//! the time of its last write, in UTC. A session resumed after it was copied
//! is copied again under its new last write, and a restore takes the newest.
//! Nothing on Drive is ever replaced or removed, so the run only ever creates
//! files in its one folder and asks Drive whether a name is there already;
//! the tests fail on any other call (drive.rs).
//!
//! A log stays on the machine for `keepDays` after its last write, 90 by
//! default. Past that, a run removes it once its copy under that last write
//! is in the ledger and Drive still finds it by name; a log Drive no longer
//! finds is kept as the only copy (backup.rs).
//!
//! What was sent is kept in a ledger, one JSON line per object
//! (ledger.rs), written as each lands. A run stopped part way carries on
//! from the ledger, and one stopped between Drive taking an object and the
//! ledger saying so finds it by name rather than sending it twice. When
//! Drive answers busy (429 or 5xx) or can't be reached, a call is tried
//! again after a few waits; if it stays busy, the run ends without failing
//! and leaves the rest for the next one (backup.rs). One run goes at a
//! time, under a lock beside the ledger, so a timer's run and a hand-run
//! one never share the scratch a seal is written to.
//!
//! It signs in as the account's gcloud application-default login, refreshing
//! its token straight from Google's endpoint (token.rs) rather than running
//! gcloud. Sealing is age's, as a library, to every recipient named
//! (seal.rs); none of their keys is needed to seal.

pub mod backup;
pub mod config;
pub mod drive;
pub mod ledger;
pub mod names;
pub mod seal;
pub mod token;
