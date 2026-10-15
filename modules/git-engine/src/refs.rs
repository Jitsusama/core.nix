//! A repository's refs, from whichever store it keeps them in: loose files
//! with `packed-refs` behind them, or reftable. A linked worktree keeps
//! HEAD and its own few refs apart from the shared ones.

use crate::hash::Oid;
use crate::reftable::{Stack, Value};
use crate::stamp::{Stamp, Stamps};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub enum Refs {
    Files {
        own: PathBuf,
        shared: PathBuf,
        packed: HashMap<String, Oid>,
        stamps: Stamps,
    },
    Reftable {
        own: Option<Stack>,
        shared: Stack,
    },
}

/// Refs a worktree keeps for itself rather than sharing.
fn is_own(name: &str) -> bool {
    !name.starts_with("refs/")
        || name.starts_with("refs/bisect/")
        || name.starts_with("refs/worktree/")
        || name.starts_with("refs/rewritten/")
}

impl Refs {
    /// `own` is the worktree's git folder and `shared` the one every worktree
    /// shares; they're the same for the main worktree.
    pub fn open(own: &Path, shared: &Path, reftable: bool) -> std::io::Result<Refs> {
        if reftable {
            let shared_stack = Stack::read(&shared.join("reftable"))?;
            let own_stack = if own == shared {
                None
            } else {
                Some(Stack::read(&own.join("reftable"))?)
            };
            return Ok(Refs::Reftable {
                own: own_stack,
                shared: shared_stack,
            });
        }
        let path = shared.join("packed-refs");
        let mut packed = HashMap::new();
        if let Ok(text) = std::fs::read(&path) {
            for line in text.split(|&b| b == b'\n') {
                if line.first().is_none_or(|&c| c == b'#' || c == b'^') || line.len() < 42 {
                    continue;
                }
                if let Some(oid) = Oid::from_hex(&line[..40]) {
                    packed.insert(String::from_utf8_lossy(&line[41..]).into_owned(), oid);
                }
            }
        }
        let stamps = vec![(path.clone(), Stamp::of(&path))];
        Ok(Refs::Files {
            own: own.to_path_buf(),
            shared: shared.to_path_buf(),
            packed,
            stamps,
        })
    }

    /// Whether what this was read from is as it was.
    pub fn current(&self) -> bool {
        match self {
            Refs::Files { stamps, .. } => crate::stamp::still(stamps),
            Refs::Reftable { own, shared } => {
                crate::stamp::still(&shared.stamps)
                    && own
                        .as_ref()
                        .is_none_or(|own| crate::stamp::still(&own.stamps))
            }
        }
    }

    pub fn read(&self, name: &str) -> Option<Value> {
        match self {
            Refs::Files {
                own,
                shared,
                packed,
                ..
            } => {
                let dir = if is_own(name) { own } else { shared };
                if let Ok(text) = std::fs::read(dir.join(name)) {
                    let text = text.trim_ascii_end();
                    if let Some(target) = text.strip_prefix(b"ref:") {
                        return Some(Value::Symbolic(
                            String::from_utf8_lossy(target.trim_ascii()).into_owned(),
                        ));
                    }
                    return Oid::from_hex(text).map(Value::Oid);
                }
                packed.get(name).copied().map(Value::Oid)
            }
            Refs::Reftable { own, shared } => {
                let stack = match own {
                    Some(own) if is_own(name) => own,
                    _ => shared,
                };
                stack.refs.get(name).cloned()
            }
        }
    }

    /// What a ref finally points at, following symbolic refs.
    pub fn resolve(&self, name: &str) -> Option<Oid> {
        let mut name = name.to_string();
        for _ in 0..5 {
            match self.read(&name)? {
                Value::Oid(oid) => return Some(oid),
                Value::Symbolic(target) => name = target,
            }
        }
        None
    }

    pub fn stashes(&self) -> usize {
        if self.read("refs/stash").is_none() {
            return 0;
        }
        match self {
            Refs::Files { shared, .. } => std::fs::read(shared.join("logs/refs/stash"))
                .map(|log| {
                    log.split(|&b| b == b'\n')
                        .filter(|line| !line.is_empty())
                        .count()
                })
                .unwrap_or(0),
            Refs::Reftable { shared, .. } => shared.stashes,
        }
    }
}
