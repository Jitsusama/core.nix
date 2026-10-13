//! What the daemon freezes a runaway writer through, whatever does it on
//! this platform.

/// What a writer is frozen as, named for it.
pub fn unit_for(pid: u32) -> String {
    format!("disk-guard-freeze-{pid}.scope")
}

/// What the daemon freezes and thaws through: a service manager, or a
/// stand-in for one.
pub trait Freeze {
    fn freeze(&mut self, unit: &str, pids: &[u32], description: &str) -> Result<(), String>;
    fn thaw(&mut self, unit: &str) -> Result<(), String>;
    fn frozen(&mut self, unit: &str) -> Result<bool, String>;
}
