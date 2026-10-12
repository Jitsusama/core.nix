//! How short of space a filesystem is, and what each level allows.

use serde::{Deserialize, Serialize};

/// A level of free space, least severe first, so `Ord` compares severity.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Band {
    Healthy,
    Pressure,
    Urgent,
    Critical,
}
