//! Whether a run sent elsewhere goes ahead of the runs waiting on a
//! builder's turns, as `--first` asks.
//!
//! On 8 November the net's comparison waited an hour for a turn on the
//! builders while sessions' gates held every one, and the net tested
//! nothing in that time. `nix-offload there --first` marks the run it
//! sends, and the builder's gate lets a marked run into a free turn or one
//! of two it keeps for them. Only the net's own unit may ask: a session
//! that passed it by habit would put its gate back ahead of the net's.

/// The unit the net runs in, which alone may send a run first.
pub const THE_NETS: &str = "merge-net.service";

/// Whether a run asked to go first does, and why not where it doesn't.
#[derive(Debug, PartialEq, Eq)]
pub enum First {
    /// Not asked.
    No,
    /// Asked, by the net, of a nix-offload that takes it.
    Yes,
    /// Asked, but it waits its turn, for the reason given.
    Refused(&'static str),
}

/// Whether a run asked to go first (`asked`) does, given the words
/// `nix-offload there --takes` printed and this process's `/proc/self/cgroup`.
pub fn first(asked: bool, takes: &str, cgroup: &str) -> First {
    if !asked {
        First::No
    } else if !the_nets(cgroup) {
        First::Refused("--first is the net's alone, so this run waits its turn")
    } else if !takes.lines().any(|word| word.trim() == "first") {
        First::Refused("this nix-offload can't send a run first, so it waits its turn")
    } else {
        First::Yes
    }
}

/// Whether a process's cgroup, as `/proc/self/cgroup` gives it, is the net's
/// unit's.
fn the_nets(cgroup: &str) -> bool {
    cgroup
        .lines()
        .any(|line| line.rsplit('/').next() == Some(THE_NETS))
}

#[cfg(test)]
mod tests {
    use super::*;

    const NET: &str =
        "0::/user.slice/user-1000.slice/user@1000.service/app.slice/merge-net.service\n";
    const SESSION: &str =
        "0::/user.slice/user-1000.slice/user@1000.service/jobs.slice/work-1.scope\n";

    #[test]
    fn only_the_nets_unit_goes_first_and_only_where_nix_offload_takes_it() {
        let takes = "tree\nchanged\nfirst\n";
        assert_eq!(first(false, takes, NET), First::No);
        assert_eq!(first(true, takes, NET), First::Yes);
        assert!(matches!(first(true, takes, SESSION), First::Refused(_)));
        assert!(matches!(
            first(true, "tree\nchanged\n", NET),
            First::Refused(_)
        ));
    }

    #[test]
    fn a_unit_named_like_the_nets_is_not_it() {
        let lookalike = "0::/user.slice/app.slice/not-merge-net.service\n";
        assert!(matches!(
            first(true, "first\n", lookalike),
            First::Refused(_)
        ));
    }
}
