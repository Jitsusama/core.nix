//! The stick's image and payload kept in the Nix store by roots until the
//! writer is done, and a payload gone from it said plainly.

use std::fs;
use std::path::Path;

use guide::stand_in::StandIn;
use guide::{Fault, Tool};

use crate::common::Scratch;

const IMAGE: &str = "/nix/store/abc-stick-1/stick_1.raw";
const PAYLOAD: &str = "/nix/store/def-machine-stick-payload";

fn rooting(link: &Path, store: &str) -> Vec<String> {
    [
        "--extra-experimental-features",
        "nix-command",
        "build",
        "--out-link",
        &link.to_string_lossy(),
        store,
    ]
    .iter()
    .map(ToString::to_string)
    .collect()
}

#[test]
fn the_image_and_payload_are_rooted_until_the_writer_is_done() {
    let directory = Scratch::new("writer-roots");
    let mut writer = StandIn::new();
    let (first, second) = (directory.join("writing-0"), directory.join("writing-1"));
    writer
        .link("/nix/store/abc-stick-1", Some("/nix/store/abc-stick-1"))
        .link(PAYLOAD, Some(PAYLOAD))
        .prints(
            Tool::Nix,
            &rooting(&first, "/nix/store/abc-stick-1")
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>(),
            "",
        )
        .prints(
            Tool::Nix,
            &rooting(&second, PAYLOAD)
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>(),
            "",
        );
    // A path outside the store, as a test's own image is, is left alone.
    let held = guide::writer::roots::hold(&mut writer, &directory, &[IMAGE, "/tmp/x.raw", PAYLOAD])
        .unwrap();
    assert_eq!(held.links(), [first.clone(), second.clone()]);
    assert_eq!(
        writer
            .calls
            .iter()
            .filter(|call| call.starts_with("nix "))
            .count(),
        2,
        "{:#?}",
        writer.calls
    );
    // Nix made the links; done, the writer takes them away.
    fs::write(&first, "").unwrap();
    fs::write(&second, "").unwrap();
    drop(held);
    assert!(
        !first.exists() && !second.exists(),
        "a root outlived the writer: {} {}",
        first.exists(),
        second.exists()
    );
}

#[test]
fn a_payload_gone_from_the_store_says_to_run_the_command_again() {
    let directory = Scratch::new("writer-root-gone");
    let mut writer = StandIn::new();
    writer
        .link("/nix/store/abc-stick-1", Some("/nix/store/abc-stick-1"))
        .link(PAYLOAD, None);
    let refused = guide::writer::roots::hold(&mut writer, &directory, &[PAYLOAD]);
    assert!(
        matches!(&refused, Err(Fault::Blocked { why, .. }) if why.contains("run the command again")),
        "{refused:?}"
    );
    assert!(
        !writer.calls.iter().any(|call| call.starts_with("nix ")),
        "nix rooted a payload that isn't there: {:#?}",
        writer.calls
    );
}
