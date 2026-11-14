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
//! them before the machines line, and empties both once nothing is
//! lent: the instance in its folder, as `instance`, and the token in
//! [`TOKENS`], named by its folder's own name. Even a lender only for
//! `there` does, since its builder stands in for the first lender's in
//! the lines root reads, and its own folder vouches for it. The token is
//! readable by the service's account alone, which an ACL on it names.
//!
//! Every token is in the one folder, made at boot, rather than in each
//! lender's, so a sandbox hides every lender's token by one name, even
//! a lender's made after the sandbox was: the token can do anything the
//! cloud account can, and nothing a sandbox runs needs it.

use std::fs;
use std::io::Write;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};

/// The account that may read the token besides the lender's: the
/// service's. It's named in an ACL rather than given the file through
/// its group, since only root may give a file to a group its process
/// isn't in, and the lender's process joins a group only when the
/// account next logs in: on 10 November the group would have come with
/// the switch that set the service up, and every lender would have
/// failed to write either file until then, so root had no lent builder.
pub const ACCOUNT: &str = "nix-offload-lines";

/// Where the machine's accounts are named.
pub const ACCOUNTS: &str = "/etc/passwd";

/// Where every lender keeps its token, as nixos.nix makes it.
pub const TOKENS: &str = "/run/nix-offload-tokens";

/// What a lender's token was called in its own folder, before tokens
/// moved to [`TOKENS`], removed as a lender starts so none is left there.
const LEFT_BEHIND: &str = "cloud-token";

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
    /// Where [`ACCOUNT`] is looked for, each time the lender says where
    /// its builder is, since the account may come with a switch while the
    /// lender runs.
    accounts: PathBuf,
    /// What was last written, and the id of [`ACCOUNT`] it was written
    /// for, so an account that comes has the token written again for it
    /// at once rather than at the next new token.
    written: Option<(Place, Option<u32>)>,
}

impl Cloud {
    /// The instance in `dir` and the token in `tokens`, named by `dir`'s
    /// own name, readable by [`ACCOUNT`] as `accounts` names it. A token
    /// left in `dir` from before is removed.
    pub fn in_dir(dir: &Path, tokens: &Path, accounts: &Path) -> Self {
        let left = dir.join(LEFT_BEHIND);
        if let Err(error) = fs::remove_file(&left)
            && error.kind() != std::io::ErrorKind::NotFound
        {
            eprintln!("nix-offload: removing {}: {error}", left.display());
        }
        let name = dir.file_name().unwrap_or(dir.as_os_str());
        Self {
            instance: dir.join("instance"),
            token: tokens.join(name),
            accounts: accounts.to_owned(),
            written: None,
        }
    }

    /// Say where the builder lent is, or empty both files while none is.
    pub fn say(&mut self, place: Option<&Place>) {
        let place = place.cloned().unwrap_or_default();
        let reader = fs::read_to_string(&self.accounts)
            .ok()
            .and_then(|accounts| id_of(&accounts, ACCOUNT));
        if self.written.as_ref() == Some(&(place.clone(), reader)) {
            return;
        }
        let line = |said: &Option<String>| {
            said.as_ref()
                .map_or_else(String::new, |said| format!("{said}\n"))
        };
        let token = written(&self.token, &line(&place.token), 0o600, reader);
        let instance = written(&self.instance, &line(&place.instance), 0o644, None);
        for (path, result) in [(&self.token, token), (&self.instance, instance)] {
            if let Err(error) = result {
                eprintln!("nix-offload: writing {}: {error}", path.display());
                return;
            }
        }
        self.written = Some((place, reader));
    }
}

/// Write `text` to `path` whole, through a draft beside it, in `mode`,
/// and readable too by the account whose id is `reader`, where there's
/// one. A file the ACL can't be put on is written all the same, in
/// `mode` alone, and said: a lender that wrote nothing would name no
/// builder to root at all.
fn written(path: &Path, text: &str, mode: u32, reader: Option<u32>) -> std::io::Result<()> {
    let draft = path.with_extension("writing");
    let _ = fs::remove_file(&draft);
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&draft)?;
    file.write_all(text.as_bytes())?;
    file.set_permissions(fs::Permissions::from_mode(mode))?;
    if let Some(reader) = reader
        && let Err(error) = readable_by(&file, reader)
    {
        eprintln!(
            "nix-offload: letting {ACCOUNT} read {}: {error}, so it's the lender's alone",
            path.display()
        );
    }
    fs::rename(&draft, path)
}

/// The ACL that lets the account `reader` read a file besides its owner,
/// and no one else, as Linux keeps one in `system.posix_acl_access`:
/// a version, then each entry's tag, permissions and id, in tag order.
/// Its mask is its group bits, so the file reads as mode 0640.
fn access_list(reader: u32) -> Vec<u8> {
    const VERSION: u32 = 2;
    const NO_ID: u32 = u32::MAX;
    const READ: u16 = 4;
    const WRITE: u16 = 2;
    let entries = [
        (0x01_u16, READ | WRITE, NO_ID), // the owner
        (0x02, READ, reader),            // the account named
        (0x04, 0, NO_ID),                // the owning group
        (0x10, READ, NO_ID),             // the mask
        (0x20, 0, NO_ID),                // everyone else
    ];
    let mut bytes = VERSION.to_le_bytes().to_vec();
    for (tag, permissions, id) in entries {
        bytes.extend(tag.to_le_bytes());
        bytes.extend(permissions.to_le_bytes());
        bytes.extend(id.to_le_bytes());
    }
    bytes
}

/// Let `reader` read `file` too, through its ACL, which its owner may
/// set in any group.
#[cfg(target_os = "linux")]
fn readable_by(file: &fs::File, reader: u32) -> std::io::Result<()> {
    use std::os::fd::AsRawFd;
    let list = access_list(reader);
    // SAFETY: the name is a C string and the value a live buffer of the
    // length given; fsetxattr reads both and keeps neither.
    let set = unsafe {
        libc::fsetxattr(
            file.as_raw_fd(),
            c"system.posix_acl_access".as_ptr(),
            list.as_ptr().cast(),
            list.len(),
            0,
        )
    };
    if set == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}

/// Elsewhere, macOS among them, a file has no POSIX ACL to set, and the
/// service runs only on NixOS.
#[cfg(not(target_os = "linux"))]
fn readable_by(_: &fs::File, _: u32) -> std::io::Result<()> {
    Err(std::io::ErrorKind::Unsupported.into())
}

/// The id of the account or group `name` in `listed`, as /etc/passwd and
/// /etc/group list them, each its third field.
fn id_of(listed: &str, name: &str) -> Option<u32> {
    listed.lines().find_map(|line| {
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
    fn an_account_is_found_by_its_name() {
        let accounts = "root:x:0:0::/root:/bin/sh\nnix-offload-lines-old:x:7:7::/:/bin/false\n\
                        nix-offload-lines:x:991:991::/var/empty:/bin/false\n";
        assert_eq!(id_of(accounts, ACCOUNT), Some(991));
        assert_eq!(id_of(accounts, "absent"), None);
    }

    /// Who else a file's ACL lets do what, by id: each named account's
    /// entry, as Linux gives the list back.
    #[cfg(target_os = "linux")]
    fn named_in(path: &Path) -> Vec<(u32, u16)> {
        use std::os::unix::ffi::OsStrExt;
        let path = std::ffi::CString::new(path.as_os_str().as_bytes()).expect("a path");
        let mut list = vec![0_u8; 256];
        // SAFETY: both names are C strings, and the buffer is live and as
        // long as it's said to be.
        let read = unsafe {
            libc::getxattr(
                path.as_ptr(),
                c"system.posix_acl_access".as_ptr(),
                list.as_mut_ptr().cast(),
                list.len(),
            )
        };
        let Ok(read) = usize::try_from(read) else {
            return Vec::new();
        };
        list[4..read]
            .as_chunks::<8>()
            .0
            .iter()
            .filter(|entry| u16::from_le_bytes([entry[0], entry[1]]) == 0x02)
            .map(|entry| {
                let permissions = u16::from_le_bytes([entry[2], entry[3]]);
                let id = u32::from_le_bytes([entry[4], entry[5], entry[6], entry[7]]);
                (id, permissions)
            })
            .collect()
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn the_token_is_the_services_alone_and_both_empty_once_none_is_lent() {
        let folder = tempfile::tempdir().expect("a folder");
        let (dir, tokens) = folders(folder.path());
        let accounts = folder.path().join("passwd");
        fs::write(
            &accounts,
            format!("{ACCOUNT}:x:65534:65534::/var/empty:/bin/false\n"),
        )
        .expect("written");
        let mut cloud = Cloud::in_dir(&dir, &tokens, &accounts);
        let place = Place {
            instance: Some("lend-5 us-central1-a".to_owned()),
            token: Some("ya29.x".to_owned()),
        };
        cloud.say(Some(&place));
        let (instance, token) = (dir.join("instance"), tokens.join(LENDER));
        let read = |path: &Path| fs::read_to_string(path).expect("a file");
        let mode = |path: &Path| fs::metadata(path).expect("a file").mode() & 0o777;
        assert_eq!(read(&instance), "lend-5 us-central1-a\n");
        assert_eq!(
            read(&token),
            "ya29.x\n",
            "the token is in the tokens folder, by its lender's name"
        );
        assert!(
            !dir.join(LEFT_BEHIND).exists(),
            "no token is in the lender's own folder"
        );
        assert_eq!(
            (named_in(&token), mode(&token)),
            handed_over(&tokens),
            "the token is read by the service's account and named for no other, its group \
             bits the ACL's mask; or, where no ACL may be set, the lender's alone"
        );
        assert_eq!(mode(&instance), 0o644, "the instance is anyone's");
        cloud.say(None);
        assert_eq!(
            (read(&instance), read(&token)),
            (String::new(), String::new())
        );
    }

    /// The name the lender's folder has in these tests.
    const LENDER: &str = "nix-offload-vm-5";

    /// A lender's folder and a tokens folder beside it, in `folder`.
    fn folders(folder: &Path) -> (PathBuf, PathBuf) {
        let (dir, tokens) = (folder.join(LENDER), folder.join("tokens"));
        for made in [&dir, &tokens] {
            fs::create_dir(made).expect("a folder");
        }
        (dir, tokens)
    }

    #[test]
    fn a_token_left_in_the_lenders_folder_is_removed() {
        let folder = tempfile::tempdir().expect("a folder");
        let (dir, tokens) = folders(folder.path());
        fs::write(dir.join(LEFT_BEHIND), "ya29.old\n").expect("written");
        let _cloud = Cloud::in_dir(&dir, &tokens, &folder.path().join("passwd"));
        assert!(
            !dir.join(LEFT_BEHIND).exists(),
            "the token left from before the move is gone as the lender starts"
        );
    }

    #[test]
    fn without_the_account_the_token_is_still_written_the_lenders_alone() {
        let folder = tempfile::tempdir().expect("a folder");
        let (dir, tokens) = folders(folder.path());
        let accounts = folder.path().join("passwd");
        fs::write(&accounts, "root:x:0:0::/root:/bin/sh\n").expect("written");
        let mut cloud = Cloud::in_dir(&dir, &tokens, &accounts);
        cloud.say(Some(&Place {
            instance: Some("lend-5 us-central1-a".to_owned()),
            token: Some("ya29.x".to_owned()),
        }));
        let (instance, token) = (dir.join("instance"), tokens.join(LENDER));
        let read = |path: &Path| fs::read_to_string(path).expect("a file");
        assert_eq!(
            (read(&instance), read(&token)),
            ("lend-5 us-central1-a\n".to_owned(), "ya29.x\n".to_owned())
        );
        assert_eq!(
            fs::metadata(&token).expect("a file").mode() & 0o777,
            0o600,
            "a token nobody else may read"
        );
        #[cfg(target_os = "linux")]
        assert_eq!(named_in(&token), Vec::new());
    }

    #[test]
    fn an_account_that_comes_has_the_same_token_written_again_for_it() {
        let folder = tempfile::tempdir().expect("a folder");
        let (dir, tokens) = folders(folder.path());
        let token = tokens.join(LENDER);
        let accounts = folder.path().join("passwd");
        fs::write(&accounts, "root:x:0:0::/root:/bin/sh\n").expect("written");
        let mut cloud = Cloud::in_dir(&dir, &tokens, &accounts);
        let place = Place {
            instance: Some("lend-5 us-central1-a".to_owned()),
            token: Some("ya29.x".to_owned()),
        };
        // Each write renames a new draft into place, so a file written
        // again is another inode.
        let inode = || fs::metadata(&token).expect("a file").ino();
        cloud.say(Some(&place));
        let first = inode();
        cloud.say(Some(&place));
        assert_eq!(
            inode(),
            first,
            "the same place, for the same account, isn't written again"
        );
        // The account comes with a switch while the lender runs, its token
        // unchanged: the token is written again at once, for the account.
        fs::write(
            &accounts,
            format!("{ACCOUNT}:x:65534:65534::/var/empty:/bin/false\n"),
        )
        .expect("written");
        cloud.say(Some(&place));
        assert_ne!(
            inode(),
            first,
            "the token is written again once the account comes"
        );
        assert_eq!(fs::read_to_string(&token).expect("a file"), "ya29.x\n");
        #[cfg(target_os = "linux")]
        {
            let mode = fs::metadata(&token).expect("a file").mode() & 0o777;
            assert_eq!(
                (named_in(&token), mode),
                handed_over(&tokens),
                "the token written again is the account's to read"
            );
        }
    }

    /// What a token handed to the service's account (uid 65534) in `dir`
    /// is left as: named in its ACL, mode 0640, where the file system lets
    /// an ACL be set; and where it doesn't, as in Nix's build sandbox,
    /// which refuses every extended attribute, the lender's alone, 0600.
    #[cfg(target_os = "linux")]
    fn handed_over(dir: &Path) -> (Vec<(u32, u16)>, u32) {
        let probe = fs::File::create(dir.join("probe")).expect("a file");
        match readable_by(&probe, 65534) {
            Ok(()) => (vec![(65534, 4)], 0o640),
            Err(error) if error.kind() == std::io::ErrorKind::Unsupported => {
                eprintln!("no ACL may be set here ({error}): proving the token is kept instead");
                (Vec::new(), 0o600)
            }
            Err(error) => panic!("setting an ACL failed other than unsupported: {error}"),
        }
    }

    #[test]
    fn the_access_list_names_the_owner_the_reader_and_a_read_mask() {
        let list = access_list(991);
        assert_eq!(
            list[..4],
            2_u32.to_le_bytes(),
            "version 2, as Linux reads it"
        );
        let entries: Vec<(u16, u16, u32)> = list[4..]
            .as_chunks::<8>()
            .0
            .iter()
            .map(|entry| {
                (
                    u16::from_le_bytes([entry[0], entry[1]]),
                    u16::from_le_bytes([entry[2], entry[3]]),
                    u32::from_le_bytes([entry[4], entry[5], entry[6], entry[7]]),
                )
            })
            .collect();
        assert_eq!(
            entries,
            vec![
                (0x01, 6, u32::MAX),
                (0x02, 4, 991),
                (0x04, 0, u32::MAX),
                (0x10, 4, u32::MAX),
                (0x20, 0, u32::MAX),
            ]
        );
    }
}
