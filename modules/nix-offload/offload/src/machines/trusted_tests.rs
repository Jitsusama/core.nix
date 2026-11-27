//! The tests of how each builder beside stands: as root's prover says, as
//! its own proofs say, and as the lender before said.

use super::*;
use crate::machines::{Machines, Standing};
use crate::testing::executable;
use std::fs;
use std::net::TcpListener;

/// A stand-in for nix that exits as the `status` file beside it says,
/// its proof's log saying the store has what the `free` file does,
/// where there is one.
fn nix_in(dir: &Path) -> String {
    let nix = dir.join("nix");
    let (status, free) = (dir.join("status"), dir.join("free"));
    executable(
        &nix,
        &format!(
            "#!/bin/sh\n\
             if [ -e '{free}' ]; then\n\
             echo \"nix-offload-proof> nix-offload-free $(cat '{free}')\" >&2\n\
             fi\n\
             exit \"$(cat '{status}')\"\n",
            free = free.display(),
            status = status.display()
        ),
    );
    nix.display().to_string()
}

/// Whether the builder beside `line` names is left out, once the proof
/// due at `now` has ended, waiting at most 5 s.
#[cfg(target_os = "linux")]
fn left_out(kept: &mut Vec<(String, Keeper)>, line: &str, nix: &str, now: Instant) -> bool {
    let heard = |kept: &Vec<(String, Keeper)>| {
        kept.first().map(|(_, keeper)| {
            (
                keeper.health.last_success,
                keeper.health.last_failure,
                keeper.health.failures,
                keeper.health.free,
            )
        })
    };
    let before = heard(kept);
    let ending = Instant::now() + Duration::from_secs(5);
    // No prover of root's writes here, so the builder is proven through `nix`.
    let root = Path::new("/nonexistent/nix-offload-standing.json");
    let mut trust = Trust::default();
    let mut dropped = kept_beside((kept, &[]), line, (nix, root, &mut trust), now);
    // One just kept has heard no proof yet.
    let unheard = |kept: &Vec<(String, Keeper)>| {
        heard(kept) == before || heard(kept) == Some((None, None, 0, None))
    };
    while unheard(kept) && Instant::now() < ending {
        std::thread::sleep(Duration::from_millis(10));
        dropped = kept_beside((kept, &[]), line, (nix, root, &mut trust), now);
    }
    assert_ne!(heard(kept), before, "no proof ended");
    dropped
}

#[test]
fn a_builder_beside_stands_as_roots_prover_says_where_one_writes() {
    let dir = tempfile::tempdir().expect("a folder");
    let alone = dir.path().join("machines-beside");
    let root = dir.path().join("standing.json");
    let listening = TcpListener::bind("127.0.0.1:0").expect("a port");
    let address = listening.local_addr().expect("its address").to_string();
    let line = "ssh-ng://work x86_64-linux - 8 1 big-parallel,benchmark";
    // A nix that fails every proof, so only root's word can list it.
    let failing = dir.path().join("nix");
    executable(&failing, "#!/bin/sh\nexit 1\n");
    let mut machines = Machines::new(
        dir.path().join("machines"),
        dir.path().join("machines-with-standing"),
        dir.path().join("store"),
        dir.path().join("lending"),
        vec![],
    )
    .beside(
        alone.clone(),
        vec![Standing {
            address,
            line: line.to_owned(),
        }],
    )
    .proving_with(&failing.display().to_string())
    .proven_by_root_in(root.clone());
    let says = |state: &str, free: u64, at: u64| {
        let said = serde_json::json!({"at": at, "builders": [{"uri": "ssh-ng://work",
            "builder": "work", "state": state, "lastSuccess": at, "lastFailure": null,
            "consecutiveFailures": 0, "freeBytes": free}]});
        fs::write(&root, said.to_string()).expect("write");
    };
    let read = |path| fs::read_to_string(path).expect("read");
    says("passing", 80_000_000_000, seconds());
    machines.show(None);
    assert_eq!(
        read(&alone),
        format!("{line}\n"),
        "listed while root says it passes"
    );
    says("full", 6_000_000_000, seconds());
    machines.show(None);
    assert_eq!(
        read(&alone),
        "",
        "left out while root says its store is under the floor"
    );
    says("passing", 80_000_000_000, seconds() - proof::FRESH - 1);
    machines.show(None);
    assert_eq!(
        read(&alone),
        "",
        "left out once root's prover has gone quiet"
    );
    let health = machines.beside_health().expect("builders beside");
    assert_eq!(
        health[0].state(),
        State::Dropped,
        "a quiet prover vouches for nothing"
    );
}

#[test]
fn a_builder_beside_waits_on_roots_prover_where_nix_doesnt_trust_the_account() {
    let dir = tempfile::tempdir().expect("a folder");
    let nix = dir.path().join("nix");
    let built = dir.path().join("built");
    executable(
        &nix,
        &format!(
            "#!/bin/sh\n\
             if [ \"$3\" = store ]; then echo '{{\"trusted\":false}}'; exit 0; fi\n\
             touch '{}'\n\
             exit 1\n",
            built.display()
        ),
    );
    let nix = nix.display().to_string();
    let root = Path::new("/nonexistent/nix-offload-standing.json");
    let line = "ssh-ng://work x86_64-linux - 8 1 big-parallel,benchmark";
    let (mut kept, mut trust) = (Vec::new(), Trust::default());
    let began = Instant::now();
    for due in [began, began + proof::EVERY, began + proof::EVERY * 2] {
        assert!(
            kept_beside((&mut kept, &[]), line, (&nix, root, &mut trust), due),
            "a builder beside nobody proved was named for builds"
        );
    }
    std::thread::sleep(Duration::from_millis(200));
    assert!(
        !built.exists(),
        "a builder beside was proven through a Nix that ignores this account's builders"
    );
    assert_eq!(
        kept[0].1.health.state(),
        State::Unproven,
        "a builder beside was judged with no proof of root's or its own"
    );
}

// Only a lender proves its builders, and only Linux runs one.
#[cfg(target_os = "linux")]
#[test]
fn a_builder_beside_failing_two_proofs_in_a_row_is_left_out_until_one_passes() {
    let dir = tempfile::tempdir().expect("a folder");
    let nix = nix_in(dir.path());
    let status = dir.path().join("status");
    let every = dir.path().join("machines");
    let alone = dir.path().join("machines-beside");
    let listening = TcpListener::bind("127.0.0.1:0").expect("a port");
    let address = listening.local_addr().expect("its address").to_string();
    let line = "ssh-ng://work x86_64-linux - 8 1 big-parallel,benchmark";
    let mut machines = Machines::new(
        every.clone(),
        dir.path().join("machines-with-standing"),
        dir.path().join("store"),
        dir.path().join("lending"),
        vec![],
    )
    .beside(
        alone.clone(),
        vec![Standing {
            address,
            line: line.to_owned(),
        }],
    )
    .proving_with(&nix);
    let read = |path| fs::read_to_string(path).expect("read");
    let began = Instant::now();
    fs::write(&status, "1").expect("write");
    assert!(
        !left_out(&mut machines.kept, line, &nix, began),
        "one failure is a blip"
    );
    machines.show(None);
    assert_eq!(read(&alone), format!("{line}\n"));
    assert!(
        left_out(&mut machines.kept, line, &nix, began + proof::EVERY),
        "a failure a proof's while after the first wasn't left out"
    );
    machines.show(None);
    assert_eq!(read(&alone), "");
    assert_eq!(read(&every), "");
    let health = machines.beside_health().expect("builders beside");
    assert_eq!(health[0].builder, "work");
    assert_eq!(health[0].state(), State::Dropped);
    fs::write(&status, "0").expect("write");
    assert!(
        !left_out(&mut machines.kept, line, &nix, began + proof::EVERY * 2),
        "a builder whose proof passed again stayed left out"
    );
    machines.show(None);
    assert_eq!(read(&alone), format!("{line}\n"), "back once one passes");
    drop(listening);
}

// On 9 November the work Mac's builder had 6 GB free, and every check
// sent there failed for room while its proofs passed.
// Only a lender proves its builders, and only Linux runs one.
#[cfg(target_os = "linux")]
#[test]
fn a_builder_beside_under_the_floor_is_left_out_until_it_has_room() {
    let dir = tempfile::tempdir().expect("a folder");
    let nix = nix_in(dir.path());
    let (status, free) = (dir.path().join("status"), dir.path().join("free"));
    let (every, alone) = (
        dir.path().join("machines"),
        dir.path().join("machines-beside"),
    );
    let listening = TcpListener::bind("127.0.0.1:0").expect("a port");
    let address = listening.local_addr().expect("its address").to_string();
    let line = "ssh-ng://work x86_64-linux - 8 1 big-parallel,benchmark";
    let mut machines = Machines::new(
        every.clone(),
        dir.path().join("machines-with-standing"),
        dir.path().join("store"),
        dir.path().join("lending"),
        vec![],
    )
    .beside(
        alone.clone(),
        vec![Standing {
            address,
            line: line.to_owned(),
        }],
    )
    .proving_with(&nix);
    let read = |path| fs::read_to_string(path).expect("read");
    let began = Instant::now();
    fs::write(&status, "0").expect("write");
    fs::write(&free, (proof::FLOOR - 1).to_string()).expect("write");
    assert!(
        left_out(&mut machines.kept, line, &nix, began),
        "full at once"
    );
    machines.show(None);
    assert_eq!(read(&alone), "");
    assert_eq!(read(&every), "");
    let health = machines.beside_health().expect("builders beside");
    assert_eq!(health[0].state(), State::Full);
    fs::write(&free, (proof::FLOOR * 2).to_string()).expect("write");
    assert!(
        !left_out(&mut machines.kept, line, &nix, began + proof::EVERY),
        "a builder with room again is taken back"
    );
    machines.show(None);
    assert_eq!(read(&alone), format!("{line}\n"), "back once it has room");
    drop(listening);
}

// On 11 November a full builder beside read as unproven to each lender
// started again, so it was named, and took builds, until its first
// proof ended.
#[test]
fn a_lender_started_again_names_no_builder_beside_before_its_proof() {
    let dir = tempfile::tempdir().expect("a folder");
    let nix = nix_in(dir.path());
    fs::write(dir.path().join("status"), "0").expect("write");
    fs::write(dir.path().join("free"), (proof::FLOOR * 2).to_string()).expect("write");
    let listening = TcpListener::bind("127.0.0.1:0").expect("a port");
    let address = listening.local_addr().expect("its address").to_string();
    let line = "ssh-ng://work x86_64-linux - 8 1 big-parallel,benchmark";
    let mut full = Health::of(&proof::named(line));
    full.heard(
        proof::Proven {
            passed: true,
            free: Some(proof::FLOOR - 1),
            ..proof::Proven::default()
        },
        10,
    );
    let lender = |earlier: Vec<Health>, name: &str| {
        Machines::new(
            dir.path().join(format!("{name}-machines")),
            dir.path().join(format!("{name}-with-standing")),
            dir.path().join(format!("{name}-store")),
            dir.path().join(format!("{name}-lending")),
            vec![],
        )
        .beside(
            dir.path().join(format!("{name}-beside")),
            vec![Standing {
                address: address.clone(),
                line: line.to_owned(),
            }],
        )
        .going_on_from(earlier)
        .proving_with(&nix)
    };
    let read = |name: &str| fs::read_to_string(dir.path().join(name)).expect("read");
    let mut again = lender(vec![full], "again");
    again.show(None);
    assert_eq!(read("again-beside"), "", "full, as the last lender said");
    assert_eq!(read("again-machines"), "", "nor named to every build");
    let health = again.beside_health().expect("builders beside");
    assert_eq!(health[0].state(), State::Full, "its health goes on");
    let mut fresh = lender(vec![], "fresh");
    fresh.show(None);
    assert_eq!(read("fresh-beside"), "", "unproven, so it may be full");
    drop(listening);
}

// The same builder sat full until a person freed it, since nothing
// here could.
// Only a lender proves its builders, and only Linux runs one.
#[cfg(target_os = "linux")]
#[test]
fn a_full_builder_beside_is_freed_by_the_program_it_names() {
    let dir = tempfile::tempdir().expect("a folder");
    let nix = nix_in(dir.path());
    let (status, free) = (dir.path().join("status"), dir.path().join("free"));
    let listening = TcpListener::bind("127.0.0.1:0").expect("a port");
    let address = listening.local_addr().expect("its address").to_string();
    let line = "ssh-ng://work x86_64-linux - 8 1 big-parallel,benchmark";
    let (program, ran) = (dir.path().join("free-work"), dir.path().join("ran"));
    executable(
        &program,
        &format!("#!/bin/sh\necho freed >> '{}'\n", ran.display()),
    );
    let freeing = dir.path().join("freeing");
    fs::write(&freeing, format!("{address} {}\n", program.display())).expect("write");
    let mut machines = Machines::new(
        dir.path().join("machines"),
        dir.path().join("machines-with-standing"),
        dir.path().join("store"),
        dir.path().join("lending"),
        vec![],
    )
    .beside(
        dir.path().join("machines-beside"),
        vec![Standing {
            address,
            line: line.to_owned(),
        }],
    )
    .freeing_from(Some(freeing))
    .proving_with(&nix);
    fs::write(&status, "0").expect("write");
    fs::write(&free, (proof::FLOOR * 2).to_string()).expect("write");
    assert!(
        !left_out(&mut machines.kept, line, &nix, Instant::now()),
        "one with room takes builds"
    );
    machines.show(None);
    assert!(!ran.exists(), "one with room is never freed");
    fs::write(&free, (proof::FLOOR - 1).to_string()).expect("write");
    assert!(
        left_out(
            &mut machines.kept,
            line,
            &nix,
            Instant::now() + proof::EVERY
        ),
        "full"
    );
    let ending = Instant::now() + Duration::from_secs(5);
    while !ran.exists() && Instant::now() < ending {
        machines.show(None);
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(
        fs::read_to_string(&ran).expect("freed"),
        "freed\n",
        "a full one is freed once"
    );
    drop(listening);
}
