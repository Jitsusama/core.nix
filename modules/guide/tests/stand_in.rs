//! The stand-in machine refusing what a real one would quietly get wrong,
//! so a step's test can't pass on a call that does nothing on a machine.

use guide::stand_in::StandIn;
use guide::tool::Path;
use guide::{Fault, Given, Machine, Program, Tool};

const OPEN: &[&str] = &[
    "open",
    "--test-passphrase",
    "--token-only",
    "--token-type",
    "systemd-fido2",
    "/dev/disk/by-partlabel/system",
];

fn opens() -> StandIn {
    let mut stand_in = StandIn::new();
    stand_in.prints(Tool::Cryptsetup, OPEN, "");
    stand_in
}

#[test]
fn cryptsetup_asked_for_a_token_without_systemds_plugins_is_refused() {
    let mut stand_in = opens();
    let refused = stand_in.run_given(Program::Tool(Tool::Cryptsetup), OPEN, Given::default());
    assert!(
        matches!(&refused, Err(Fault::Unscripted { call }) if call.ends_with("without systemd's token plugins")),
        "{refused:?}"
    );
}

#[test]
fn the_plain_variables_a_call_was_given_are_kept_by_name() {
    let mut stand_in = opens();
    stand_in
        .run_given(
            Program::Tool(Tool::Cryptsetup),
            OPEN,
            Given {
                plain: vec![("LD_LIBRARY_PATH", Path::Tokens.get().unwrap())],
                ..Given::default()
            },
        )
        .unwrap();
    assert_eq!(
        stand_in.calls,
        [format!("cryptsetup {} {{LD_LIBRARY_PATH}}", OPEN.join(" "))]
    );
}
