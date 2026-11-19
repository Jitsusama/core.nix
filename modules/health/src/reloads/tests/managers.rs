//! A reload and the managers it speaks for: the machine's systemd and each
//! account's reload apart, and a switch reloads each of them.

use super::*;

/// A line of the account's manager, or of a service of the account's.
fn of_his(line: Entry) -> Entry {
    Entry {
        account: Some("1000".to_owned()),
        ..line
    }
}

/// The backend's ready line, at `at` on both clocks, in the machine's.
fn ready(at: i64) -> Entry {
    Entry {
        at,
        since_boot: at,
        boot: "b".to_owned(),
        ..entry("backend", 6, "backend: ready in 5 ms")
    }
}

#[test]
fn a_restarts_lines_after_its_stop_are_its_own_managers() {
    let cases = vec![
        // A service of one name in both: his says nothing of when the
        // machine's was ready, nor that it ran on.
        (
            "his service's ready line after the machine's restart",
            vec![
                backend("b", 10_800_000, Job::Stopping),
                backend("b", 10_900_000, Job::Started),
                of_his(ready(11_000_000)),
            ],
            vec![100],
        ),
        (
            "his service's stop before the machine's ready line",
            vec![
                backend("b", 10_800_000, Job::Stopping),
                backend("b", 10_900_000, Job::Started),
                of_his(backend("b", 10_950_000, Job::Stopping)),
                ready(11_000_000),
            ],
            vec![200],
        ),
        // A restart of his his tool's reload took, given back by his
        // switch's reload heard after: his own manager's pairing holds
        // through the taking, its start and its next job.
        (
            "his switch's reload heard after his tool's took his stop, then its start",
            vec![
                of_his(backend("b", 12_000_000, Job::Stopping)),
                of_his(reload_in("b", 10_000_000, "systemctl")),
                of_his(backend("b", 12_100_000, Job::Started)),
                of_his(reload_in("b", 11_000_000, "sd-switch")),
            ],
            vec![100],
        ),
        (
            "his switch's reload heard after a stop his tool's made",
            vec![
                of_his(reload_in("b", 10_000_000, "systemctl")),
                of_his(backend("b", 12_000_000, Job::Stopping)),
                of_his(reload_in("b", 11_000_000, "sd-switch")),
                of_his(backend("b", 12_100_000, Job::Started)),
            ],
            vec![100],
        ),
        (
            "his ready line after his next job while his restart was taken back",
            vec![
                of_his(backend("b", 12_000_000, Job::Stopping)),
                of_his(backend("b", 12_100_000, Job::Started)),
                of_his(reload_in("b", 10_000_000, "systemctl")),
                of_his(backend("b", 25_000_000, Job::Stopping)),
                of_his(ready(25_500_000)),
                of_his(reload_in("b", 11_000_000, "sd-switch")),
            ],
            vec![100],
        ),
    ];
    each_restarted(cases);
}

#[test]
fn a_reload_counts_only_for_its_own_managers_services() {
    let cases = vec![
        // Any process of his can have his manager reload, under any name:
        // it says nothing of the machine's services.
        (
            "his tool's reload, then the machine's restart",
            vec![
                of_his(reload_in("b", 10_000_000, "systemctl")),
                backend("b", 10_500_000, Job::Stopping),
                backend("b", 10_600_000, Job::Started),
            ],
            vec![100],
        ),
        (
            "his tool's reload heard after the machine's restart",
            vec![
                backend("b", 10_500_000, Job::Stopping),
                backend("b", 10_600_000, Job::Started),
                of_his(reload_in("b", 10_000_000, "systemctl")),
            ],
            vec![100],
        ),
        (
            "a switch's name in his manager, then the machine's tool's restart",
            vec![
                of_his(reload_in("b", 1_000_000, "sd-switch")),
                reload_in("b", 10_000_000, "systemctl"),
                backend("b", 10_500_000, Job::Stopping),
                backend("b", 10_600_000, Job::Started),
            ],
            vec![],
        ),
        (
            "the machine's tool's reload, then his restart",
            vec![
                reload_in("b", 10_000_000, "systemctl"),
                of_his(backend("b", 10_500_000, Job::Stopping)),
                of_his(backend("b", 10_600_000, Job::Started)),
            ],
            vec![100],
        ),
        (
            "his tool's reload, then his restart",
            vec![
                of_his(reload_in("b", 10_000_000, "systemctl")),
                of_his(backend("b", 10_500_000, Job::Stopping)),
                of_his(backend("b", 10_600_000, Job::Started)),
            ],
            vec![],
        ),
    ];
    each_restarted(cases);
}
