//! A process a test starts, stopped when the test ends, passed or failed:
//! one left running holds the build's output open, and the build never
//! ends. Shared by every test that starts one.

use std::process::Child;

/// A started process, killed and reaped when dropped.
pub struct Running(pub Child);

impl Drop for Running {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
