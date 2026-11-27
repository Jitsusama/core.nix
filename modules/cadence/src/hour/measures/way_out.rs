//! The way the sessions' calls go out: pi's calls as the sessions met
//! them, the path to each origin, and the model relay between them. They
//! change together, since each says the same outage from its own side,
//! and every count of them is asked of the way's keeper apart, never
//! taking the hour's lead from what else drags.

pub mod calls;
pub mod path;
pub mod relay;

/// Whether `key` is a count of the way the sessions' calls go out.
#[must_use]
pub fn on_the_way(key: &str) -> bool {
    [relay::PREFIX, calls::PREFIX, path::PREFIX]
        .iter()
        .any(|prefix| key.starts_with(prefix))
}
