//! A repository's key from its remote's address, so every clone of one
//! history, cloned over HTTPS or SSH, in either of ~/src's layouts, is
//! the same repository. The key reads as ghq lays clones out: where it's
//! kept, then its path there, `github:Jitsusama/core.nix`. GitHub and
//! GitLab are said by their names; any other host by its own address.

/// A repository's identity, and the name and owner a person sees.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Key {
    pub key: String,
    pub name: String,
    pub owner: Option<String>,
}

/// The key a remote's address gives, or none for an address on the
/// machine itself (a path, `file://`, or `.`), which is no remote at all.
pub fn of_remote(url: &str) -> Option<Key> {
    let (host, path) = host_and_path(url.trim())?;
    let path = path.trim_matches('/');
    let path = path.strip_suffix(".git").unwrap_or(path);
    let path = path.trim_end_matches('/');
    if path.is_empty() {
        return None;
    }
    let host = host.to_lowercase();
    let host = host.strip_prefix("www.").unwrap_or(&host);
    let place = match host {
        "github.com" => "github",
        "gitlab.com" => "gitlab",
        other => other,
    };
    let (owner, name) = match path.rsplit_once('/') {
        Some((owner, name)) => (Some(owner.to_owned()), name.to_owned()),
        None => (None, path.to_owned()),
    };
    Some(Key {
        key: format!("{place}:{path}"),
        name,
        owner,
    })
}

/// The key of a repository with no remote: its checkout's folder, said
/// from home, since nothing else names it.
pub fn of_folder(shown: &str, folder_name: &str) -> Key {
    Key {
        key: format!("local:{shown}"),
        name: folder_name.to_owned(),
        owner: None,
    }
}

/// The host and path of an address git takes: `scheme://[user@]host[:port]/path`
/// or scp's `[user@]host:path`. None for a path on the machine.
fn host_and_path(url: &str) -> Option<(&str, &str)> {
    if let Some((scheme, rest)) = url.split_once("://") {
        if scheme.eq_ignore_ascii_case("file") {
            return None;
        }
        let (authority, path) = rest.split_once('/').unwrap_or((rest, ""));
        let host = authority
            .rsplit_once('@')
            .map_or(authority, |(_, host)| host);
        let host = host.split_once(':').map_or(host, |(host, _)| host);
        return (!host.is_empty()).then_some((host, path));
    }
    // scp's form: a colon before any slash, and something before it that
    // isn't a Windows drive's letter.
    let colon = url.find(':')?;
    if url[..colon].contains('/') || colon < 2 {
        return None;
    }
    let authority = &url[..colon];
    let host = authority
        .rsplit_once('@')
        .map_or(authority, |(_, host)| host);
    Some((host, &url[colon + 1..]))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(url: &str) -> Option<String> {
        of_remote(url).map(|key| key.key)
    }

    #[test]
    fn https_and_ssh_clones_of_one_repository_share_its_key() {
        for url in [
            "https://github.com/Jitsusama/core.nix.git",
            "https://github.com/Jitsusama/core.nix",
            "https://github.com/Jitsusama/core.nix/",
            "git@github.com:Jitsusama/core.nix.git",
            "ssh://git@github.com/Jitsusama/core.nix.git",
            "ssh://git@github.com:22/Jitsusama/core.nix",
            "https://user@www.github.com/Jitsusama/core.nix.git",
        ] {
            assert_eq!(
                key(url).as_deref(),
                Some("github:Jitsusama/core.nix"),
                "{url} gave another key"
            );
        }
    }

    #[test]
    fn its_name_is_the_last_part_and_its_owner_the_rest() {
        let key = of_remote("https://gitlab.com/jitsusama/group/reef.git").expect("a remote");
        assert_eq!(key.key, "gitlab:jitsusama/group/reef", "{key:?}");
        assert_eq!(key.name, "reef", "{key:?}");
        assert_eq!(key.owner.as_deref(), Some("jitsusama/group"), "{key:?}");
    }

    #[test]
    fn any_other_host_is_said_by_its_own_address() {
        assert_eq!(
            key("https://git.example.org/team/tool.git").as_deref(),
            Some("git.example.org:team/tool"),
            "another host's key changed"
        );
    }

    #[test]
    fn an_address_on_the_machine_is_no_remote() {
        for url in [
            ".",
            "/home/someone/src/thing",
            "../thing",
            "file:///srv/thing.git",
        ] {
            assert_eq!(key(url), None, "{url} was taken for a remote");
        }
    }
}
