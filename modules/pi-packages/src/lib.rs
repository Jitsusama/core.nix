//! pi-packages: each of pi's packages installed into a release folder of
//! its own, checked whole, and swapped in by one rename, so no file a
//! running pi has loaded ever changes under it.
//!
//! pi's own update of a git package resets the checkout, cleans it with
//! `git clean -fdx` and installs its dependencies again, all in the folder
//! every running session loaded from. On 26 October that took a native
//! module's binding away from live sessions, and rewriting it in place
//! crashed them. pi never installs or changes a package named by a local
//! path, so pi is given each package as a link instead:
//!
//!   <root>/<host>/<path>/current -> releases/<commit>
//!
//! An update fetches the package's mirror (`mirror.git`), and when its
//! branch has moved, builds the new commit into a hidden folder beside
//! the releases: a clone sharing the mirror's objects, and its production
//! dependencies, installed with this machine's pnpm and node in an
//! environment of their own, so no shell's pnpm or settings leak in. The
//! build is checked whole (check.rs) and only then renamed into
//! `releases/<commit>`, and `current` is pointed at it by renaming a new
//! link over the old one, which a reader sees as the old release or the new,
//! never half of either (release.rs).
//!
//! A release `current` has left stays as it was, for the sessions still
//! running it, until it has been out for `keepDays` and no process on the
//! machine holds a file in it, maps one or stands in it (inuse.rs). A
//! build that fails its check is removed and `current` is left alone; the
//! next run tries again. One update runs at a time for each package, under
//! a lock beside its releases.

pub mod check;
pub mod config;
pub mod inuse;
pub mod release;
pub mod run;
