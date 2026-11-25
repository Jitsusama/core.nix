//! Which build the train goes as: the merge-net the watch follows, which
//! the watch names under the net's state folder as it starts, rather than
//! the asker's, and between candidates the build a switch installed since
//! the ride began.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

/// The file under the net's state folder `root` naming where the watch
/// follows the merge-net installed.
fn installed_file(root: &Path) -> PathBuf {
    root.join("installed")
}

/// Say, under the net's state folder `root`, that the merge-net installed
/// is at `path`, for the train to start as and follow. The watch says so
/// as it starts. The path is made absolute but its links are kept, so the
/// file goes on naming the build a later switch puts there.
///
/// # Errors
///
/// When the path can't be made absolute or the file can't be written.
pub fn install(root: &Path, path: &Path) -> io::Result<()> {
    let path = std::path::absolute(path)?;
    fs::write(installed_file(root), format!("{}\n", path.display()))
}

/// The build installed where the watch follows, as it is now, when the
/// watch named one that can be read.
fn installed(root: &Path) -> Option<PathBuf> {
    let named = fs::read_to_string(installed_file(root)).ok()?;
    fs::canonicalize(named.trim()).ok()
}

/// The build the train starts as: the one installed, rather than the
/// asker's own, which may be a tree's or a profile's from before the last
/// switch; the asker's own only when the watch named none.
pub(super) fn build(root: &Path, running: io::Result<PathBuf>) -> Option<PathBuf> {
    installed(root).or_else(|| running.ok())
}

/// The build installed when it is another than the `running` one, for the
/// train to go on as; never the build this ride was `handed` to, so a ride
/// that can't see itself as installed (its own path unreadable) doesn't
/// hand over to itself without end.
pub(super) fn newer(
    root: &Path,
    running: io::Result<PathBuf>,
    handed: Option<&Path>,
) -> Option<PathBuf> {
    let installed = installed(root)?;
    if handed == Some(installed.as_path()) {
        return None;
    }
    crate::world::replaced(Ok(installed.clone()), running).then_some(installed)
}

/// What a ride sets, to the path of the build it hands over to, which then
/// keeps the right of way the ride held.
const HANDED_OVER: &str = "MERGE_NET_HANDED_OVER";

/// The build a ride handed this one over to, when one did.
pub(super) fn handed() -> Option<PathBuf> {
    std::env::var_os(HANDED_OVER).map(PathBuf::from)
}

/// Go on as the build installed under `root`, when it is a newer one.
///
/// A ride started before a switch went on as the build from before it:
/// on 18 November one pushed #5426, #5460 and #5321 to main itself,
/// under Joel's key, after the switch that made the train merge through
/// GitHub. Between candidates it goes on as the build installed instead;
/// when that build won't run, it goes on as it is rather than leave its
/// riders waiting on an asker.
pub(super) fn go_on_as_installed(root: &Path) {
    if let Some(newer) = newer(root, std::env::current_exe(), handed().as_deref()) {
        println!(
            "merge-net: another merge-net is installed, so the train goes on as {}",
            newer.display()
        );
        println!("merge-net: {}, so it goes on as it is", hand_over(&newer));
    }
}

/// Go on riding as the build at `newer`, in this process and so this
/// unit: the riders and the right of way are on disk, but the search a
/// split was in starts afresh. Only says why, when it couldn't.
pub(super) fn hand_over(newer: &Path) -> String {
    use std::os::unix::process::CommandExt;
    let why = Command::new(newer)
        .arg("ride")
        .env(HANDED_OVER, newer)
        .exec();
    format!("the train couldn't go on as {}: {why}", newer.display())
}
