//! What the server's integration tests share, each taking it as a module of
//! its own (`mod common;`), so a fix to it reaches every test at once.

use std::path::Path;
use std::process::Child;
use std::thread;

/// Stops the server `child` and takes its `folder` away, failing the test
/// when the server had ended under it: a server that died serves nothing,
/// and must not pass for one that refused.
pub fn stop(child: &mut Child, folder: &Path) {
    let ended = child.try_wait().ok().flatten();
    let _ = child.kill();
    let _ = child.wait();
    let _ = std::fs::remove_dir_all(folder);
    if let Some(status) = ended
        && !thread::panicking()
    {
        panic!("the server ended under the test: {status}");
    }
}
