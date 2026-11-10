//! Where the lent builder is in the cloud, for the service that writes
//! the builder lines root's Nix reads (`nix-offload-lines`, run by an
//! account of its own). A line names the host key root believes, and
//! this account mustn't be the one to say which, or any program running
//! as it could list a builder of its own. So the lender says only which
//! instance stands, and the service asks the cloud for that instance's
//! host key itself, which only the instance can publish.
//!
//! The helper says, before its machines line, `instance NAME ZONE` and
//! `cloud-token TOKEN`, an access token the service asks the cloud with,
//! and says the token again while it holds the builder, since one lasts
//! an hour and the service asks whenever it starts. Each lender writes
//! them in its folder, `instance` and `cloud-token`, before the machines
//! line, and empties both once nothing is lent. Even a lender only for
//! `there` does, since its builder stands in for the first lender's in
//! the lines root reads, and its own folder vouches for it. The token is
//! readable by the service's group alone.

use std::fs;
use std::io::Write;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};

/// The group whose members may read the token: the service's own, which
/// the lender's account is in so it can hand the file over.
pub const GROUP: &str = "nix-offload-lines";

/// Where the machine's groups are named.
pub const GROUPS: &str = "/etc/group";

/// What the helper said of where its builder is: its instance, `NAME
/// ZONE`, and the token the cloud is asked about it with, each once said.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Place {
    pub instance: Option<String>,
    pub token: Option<String>,
}

impl Place {
    /// Take `line` if it says where the builder is, saying whether it did.
    pub fn heard(&mut self, line: &str) -> bool {
        if let Some(instance) = line.strip_prefix("instance ") {
            self.instance = Some(instance.trim().to_owned());
        } else if let Some(token) = line.strip_prefix("cloud-token ") {
            self.token = Some(token.trim().to_owned());
        } else {
            return false;
        }
        true
    }
}

/// The two files the service reads in a lender's folder, as last written.
pub struct Cloud {
    instance: PathBuf,
    token: PathBuf,
    /// The id of [`GROUP`], where the machine has it.
    group: Option<u32>,
    written: Option<Place>,
}

impl Cloud {
    /// The files in `dir`, the token handed to [`GROUP`] as `groups`
    /// names it.
    pub fn in_dir(dir: &Path, groups: &Path) -> Self {
        Self {
            instance: dir.join("instance"),
            token: dir.join("cloud-token"),
            group: fs::read_to_string(groups)
                .ok()
                .and_then(|groups| id_of(&groups, GROUP)),
            written: None,
        }
    }

    /// Say where the builder lent is, or empty both files while none is.
    pub fn say(&mut self, place: Option<&Place>) {
        let place = place.cloned().unwrap_or_default();
        if self.written.as_ref() == Some(&place) {
            return;
        }
        let line = |said: &Option<String>| {
            said.as_ref()
                .map_or_else(String::new, |said| format!("{said}\n"))
        };
        let token = written(&self.token, &line(&place.token), self.group);
        let instance = written(&self.instance, &line(&place.instance), None);
        for (path, result) in [(&self.token, token), (&self.instance, instance)] {
            if let Err(error) = result {
                eprintln!("nix-offload: writing {}: {error}", path.display());
                return;
            }
        }
        self.written = Some(place);
    }
}

/// Write `text` to `path` whole, through a draft beside it: readable by
/// everyone without a `group`, and with one by its members alone.
fn written(path: &Path, text: &str, group: Option<u32>) -> std::io::Result<()> {
    let draft = path.with_extension("writing");
    let _ = fs::remove_file(&draft);
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&draft)?;
    file.write_all(text.as_bytes())?;
    let mode = match group {
        Some(group) => {
            std::os::unix::fs::fchown(&file, None, Some(group))?;
            0o640
        }
        None if path.ends_with("cloud-token") => 0o600,
        None => 0o644,
    };
    file.set_permissions(fs::Permissions::from_mode(mode))?;
    fs::rename(&draft, path)
}

/// The id of the group `name` in `groups`, as /etc/group lists them.
fn id_of(groups: &str, name: &str) -> Option<u32> {
    groups.lines().find_map(|line| {
        let mut fields = line.split(':');
        (fields.next() == Some(name))
            .then(|| fields.nth(1)?.parse().ok())
            .flatten()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::MetadataExt;

    #[test]
    fn the_helper_says_where_its_builder_is() {
        let mut place = Place::default();
        assert!(
            place.heard("instance lend-5 us-central1-a"),
            "its instance is taken"
        );
        assert!(place.heard("cloud-token ya29.x"), "its token is taken");
        assert!(!place.heard("price 0.68"), "its price is the lender's");
        assert_eq!(
            place,
            Place {
                instance: Some("lend-5 us-central1-a".to_owned()),
                token: Some("ya29.x".to_owned()),
            }
        );
    }

    #[test]
    fn a_group_is_found_by_its_name() {
        let groups = "root:x:0:\nnix-offload-lines-old:x:7:\nnix-offload-lines:x:991:someone\n";
        assert_eq!(id_of(groups, GROUP), Some(991));
        assert_eq!(id_of(groups, "absent"), None);
    }

    #[test]
    fn the_token_is_its_groups_alone_and_both_empty_once_none_is_lent() {
        let folder = tempfile::tempdir().expect("a folder");
        let dir = folder.path();
        let mine = fs::metadata(dir).expect("the folder").gid();
        let groups = dir.join("group");
        fs::write(&groups, format!("{GROUP}:x:{mine}:\n")).expect("written");
        let mut cloud = Cloud::in_dir(dir, &groups);
        let place = Place {
            instance: Some("lend-5 us-central1-a".to_owned()),
            token: Some("ya29.x".to_owned()),
        };
        cloud.say(Some(&place));
        let read = |name: &str| fs::read_to_string(dir.join(name)).expect("a file");
        let mode = |name: &str| fs::metadata(dir.join(name)).expect("a file").mode() & 0o777;
        assert_eq!(read("instance"), "lend-5 us-central1-a\n");
        assert_eq!(read("cloud-token"), "ya29.x\n");
        assert_eq!(
            (mode("cloud-token"), mode("instance")),
            (0o640, 0o644),
            "the token is its group's alone, and the instance anyone's"
        );
        cloud.say(None);
        assert_eq!(
            (read("instance"), read("cloud-token")),
            (String::new(), String::new())
        );
    }

    #[test]
    fn without_the_group_the_token_is_the_lenders_alone() {
        let folder = tempfile::tempdir().expect("a folder");
        let dir = folder.path();
        let mut cloud = Cloud::in_dir(dir, &dir.join("no-groups"));
        cloud.say(Some(&Place {
            instance: None,
            token: Some("ya29.x".to_owned()),
        }));
        let mode = fs::metadata(dir.join("cloud-token"))
            .expect("a file")
            .mode()
            & 0o777;
        assert_eq!(mode, 0o600, "a token nobody else may read");
    }
}
