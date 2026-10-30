//! What a writer writes a stick from, kept from Nix's garbage collector for
//! as long as the writer runs. `nix run` roots nothing it builds, so on a
//! machine whose store is kept small, as a disk guard keeps it, the stick's
//! image or its payload can go while the person is still answering the
//! writer's questions, and the write then finds it gone.

use std::path::{Path, PathBuf};

use crate::{Fault, Given, Machine, Program, Tool};

/// Roots held until this drops, each a link Nix counts as one.
#[derive(Debug, Default)]
pub struct Held {
    links: Vec<PathBuf>,
}

impl Held {
    /// The links held, for a test to see where they are.
    pub fn links(&self) -> &[PathBuf] {
        &self.links
    }
}

impl Drop for Held {
    fn drop(&mut self) {
        for link in &self.links {
            // A link already gone holds nothing, and one that can't be
            // removed only keeps its path a little longer, so neither
            // stops the writer from finishing.
            let _ = std::fs::remove_file(link);
        }
    }
}

/// The store path a path is inside, or none when it isn't in the store.
fn store_path(path: &str) -> Option<String> {
    let inside = path.strip_prefix("/nix/store/")?;
    let name = inside.split('/').next().filter(|name| !name.is_empty())?;
    Some(format!("/nix/store/{name}"))
}

/// Each of these paths that's in the Nix store rooted by a link in
/// `folder`, which it makes, until what this gives back drops. A path
/// already gone from the store is refused, saying to run the command
/// again, which builds it again; a path outside the store is left alone.
pub fn hold(machine: &mut dyn Machine, folder: &Path, paths: &[&str]) -> Result<Held, Fault> {
    let mut held = Held::default();
    let home = std::env::var("HOME").unwrap_or_default();
    for path in paths {
        let Some(store) = store_path(path) else {
            continue;
        };
        if machine.resolve(&store)?.is_none() {
            return Err(Fault::Blocked {
                step: "write-stick",
                why: format!(
                    "{store} has gone from the Nix store since it was built: run the command again, which builds it again"
                ),
            });
        }
        let link = folder.join(format!("writing-{}", held.links.len()));
        let link_text = link.to_string_lossy().into_owned();
        // Already in the store, it builds nothing: Nix only makes the link
        // and counts it.
        machine
            .run_given(
                Program::Tool(Tool::Nix),
                &[
                    "--extra-experimental-features",
                    "nix-command",
                    "build",
                    "--out-link",
                    &link_text,
                    &store,
                ],
                Given {
                    plain: vec![("HOME", home.as_str())],
                    ..Given::default()
                },
            )?
            .ok()?;
        held.links.push(link);
    }
    Ok(held)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_file_inside_a_store_path_roots_the_whole_path() {
        assert_eq!(
            store_path("/nix/store/abc-stick-1/stick_1.raw").as_deref(),
            Some("/nix/store/abc-stick-1")
        );
        assert_eq!(
            store_path("/nix/store/abc-payload").as_deref(),
            Some("/nix/store/abc-payload")
        );
        assert_eq!(store_path("/tmp/stick.raw"), None);
        assert_eq!(store_path("/nix/store/"), None);
    }
}
