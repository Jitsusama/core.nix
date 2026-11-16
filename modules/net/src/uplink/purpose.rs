//! What a socket's upload is for, and whose it is, from where it goes and
//! the process holding it. Pure, so every rule is a case below.

use std::net::IpAddr;

use serde::Serialize;

use super::owners::Owner;

/// What the bytes are for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Purpose {
    /// An agent session's requests to its model: the whole conversation,
    /// every turn.
    Sessions,
    /// Tunnels to remote builders, carrying what they build from.
    Builders,
    /// Container images pushed to a registry.
    Images,
    Backups,
    Git,
    /// Anything else a known process sends.
    Other,
    /// A socket whose process can't be read: another user's, root's
    /// included, or one that ended.
    Unknown,
}

/// Whose traffic it is: Joel's own programs, or the work running for him.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Side {
    Joel,
    Agents,
    Unknown,
}

/// What a socket to this peer, held by this owner, is for; nothing for a
/// peer on this network, whose bytes never reach the uplink.
pub fn purpose(peer: IpAddr, owner: Option<&Owner>) -> Option<Purpose> {
    if !reaches_the_uplink(peer) {
        return None;
    }
    let Some(owner) = owner else {
        return Some(Purpose::Unknown);
    };
    let name = owner.name.as_str();
    let command = owner.command.as_str();
    let pushes = command.split_whitespace().any(|word| word == "push");
    Some(
        if command.contains("start-iap-tunnel") || owner.cgroup.contains("/offload.slice/") {
            Purpose::Builders
        } else if pushes
            && ["podman", "docker", "skopeo", "buildah"]
                .iter()
                .any(|tool| name.contains(tool) || command.contains(tool))
        {
            Purpose::Images
        } else if ["backup", "restic", "rclone", "borg"]
            .iter()
            .any(|tool| name.contains(tool))
        {
            Purpose::Backups
        } else if name.starts_with("git") || name == "gh" || name.starts_with("gh.") {
            Purpose::Git
        } else if name == "pi" || command.contains("pi-host") || owner.cgroup.contains("/pi-host-")
        {
            Purpose::Sessions
        } else {
            Purpose::Other
        },
    )
}

/// Whose a socket is, by where systemd placed its process: Joel's apps
/// and his desktop session are his, every slice the work runs in is the
/// agents'.
pub fn side(owner: Option<&Owner>) -> Side {
    let Some(owner) = owner else {
        return Side::Unknown;
    };
    let cgroup = owner.cgroup.as_str();
    if cgroup.contains("/pi-host-") || owner.name == "pi" {
        Side::Agents
    } else if cgroup.contains("/app.slice/") || cgroup.contains("/session.slice/") {
        Side::Joel
    } else if [
        "/agents.slice/",
        "/jobs.slice/",
        "/background.slice/",
        "/offload.slice/",
    ]
    .iter()
    .any(|slice| cgroup.contains(slice))
    {
        Side::Agents
    } else {
        Side::Unknown
    }
}

/// Whether a peer is past this network: not loopback, not a private or
/// link-local address, and not the carrier's shared range.
fn reaches_the_uplink(peer: IpAddr) -> bool {
    // A dual-stack socket names an IPv4 peer as ::ffff:a.b.c.d.
    match peer.to_canonical() {
        IpAddr::V4(v4) => {
            let [first, second, ..] = v4.octets();
            let shared = first == 100 && (64..128).contains(&second);
            !(v4.is_loopback()
                || v4.is_private()
                || v4.is_link_local()
                || shared
                || v4.is_unspecified())
        }
        IpAddr::V6(v6) => {
            let first = v6.segments()[0];
            let unique_local = first & 0xfe00 == 0xfc00;
            let link_local = first & 0xffc0 == 0xfe80;
            !(v6.is_loopback() || v6.is_unspecified() || unique_local || link_local)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn owner(name: &str, command: &str, cgroup: &str) -> Owner {
        Owner {
            pid: 1,
            name: name.into(),
            command: command.into(),
            cgroup: cgroup.into(),
        }
    }

    const FAR: &str = "203.0.113.7";
    const AGENTS: &str = "/user.slice/user-1000.slice/user@1000.service/jobs.slice/x.scope";
    const APPS: &str = "/user.slice/user-1000.slice/user@1000.service/app.slice/app-chrome.scope";

    fn far() -> IpAddr {
        FAR.parse().unwrap()
    }

    #[test]
    fn a_peer_on_this_network_has_no_purpose_on_the_uplink() {
        let git = owner("git", "git push", AGENTS);
        for near in [
            "192.168.4.49",
            "10.0.0.2",
            "127.0.0.1",
            "100.100.1.1",
            "fe80::1",
            "fd00::2",
        ] {
            assert_eq!(purpose(near.parse().unwrap(), Some(&git)), None, "{near}");
        }
    }

    #[test]
    fn each_kind_of_upload_is_told_by_its_process() {
        let cases = [
            (
                owner(
                    "python3.14",
                    "python gcloud.py compute start-iap-tunnel vm 22",
                    AGENTS,
                ),
                Purpose::Builders,
            ),
            (
                owner(".podman-wrapped", "docker push registry/image:tag", AGENTS),
                Purpose::Images,
            ),
            (
                owner("podman", "podman pull registry/image:tag", AGENTS),
                Purpose::Other,
            ),
            (
                owner(".session-backup", "session-backup", AGENTS),
                Purpose::Backups,
            ),
            (
                owner("git-remote-http", "git-remote-https origin", AGENTS),
                Purpose::Git,
            ),
            (owner("pi", "pi --session x", AGENTS), Purpose::Sessions),
            (
                owner(
                    "node",
                    "node scripts/pi-host.ts background",
                    "/x/agents.slice/pi-host-work-background.service",
                ),
                Purpose::Sessions,
            ),
            (owner("chrome", "chrome", APPS), Purpose::Other),
        ];
        for (held, wanted) in cases {
            assert_eq!(purpose(far(), Some(&held)), Some(wanted), "{held:?}");
        }
        assert_eq!(purpose(far(), None), Some(Purpose::Unknown));
    }

    #[test]
    fn joels_apps_are_his_and_the_slices_of_work_are_the_agents() {
        assert_eq!(side(Some(&owner("chrome", "chrome", APPS))), Side::Joel);
        assert_eq!(side(Some(&owner("git", "git push", AGENTS))), Side::Agents);
        let host = owner(
            "node",
            "node pi-host.ts attended",
            "/x/app.slice/pi-host-attended.service",
        );
        assert_eq!(side(Some(&host)), Side::Agents);
        assert_eq!(
            side(Some(&owner("cron", "cron", "/system.slice/cron.service"))),
            Side::Unknown
        );
        assert_eq!(side(None), Side::Unknown);
    }
}
