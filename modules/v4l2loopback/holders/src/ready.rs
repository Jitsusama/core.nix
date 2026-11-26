//! How long a run took to be ready: from its start to its file first
//! saying who holds the camera, said once in its log as `ready in N ms`.
//! A switch restarts each watch, and health times a restart to the line a
//! daemon says once it's ready (docs/design.md, under Latency); with none,
//! the restart reads as one that never said it served again.

use std::time::Instant;

/// When the run began, until it has said it's ready.
pub struct Ready(Option<Instant>);

impl Ready {
    /// A run beginning now.
    pub fn begun() -> Self {
        Self(Some(Instant::now()))
    }

    /// The line saying it's ready, the first time its file is written,
    /// and nothing after.
    pub fn line(&mut self) -> Option<String> {
        self.0
            .take()
            .map(|began| format!("ready in {} ms", began.elapsed().as_millis()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_run_says_its_ready_once() {
        let mut ready = Ready::begun();
        let line = ready.line().expect("a run's first write said nothing");
        assert!(
            line.starts_with("ready in ") && line.ends_with(" ms"),
            "the ready line isn't one health reads: {line}"
        );
        assert_eq!(ready.line(), None, "a second write said ready again");
    }
}
