# nix-offload, which lends the machine a builder while its queue holds
# builds back, built from this directory, its tests run as it builds. It
# reads which crates read which from ../../crate-reach, so the source is
# cut from the modules folder holding both, and built from this one.
{
  bubblewrap,
  cacert,
  callPackage,
  coreutils,
  dash,
  git,
  lib,
  rustPlatform,
  stdenv,
  systemd,
  util-linux,
  zsh,
}:
let
  changed-checks = callPackage ../../../tools/changed-checks/package.nix { };
in
rustPlatform.buildRustPackage rec {
  pname = "nix-offload";
  version = "0.1.0";
  src = lib.fileset.toSource {
    root = ../..;
    fileset = lib.fileset.unions [
      ./Cargo.toml
      ./Cargo.lock
      ./src
      ./tests
      ../../crate-reach/Cargo.toml
      ../../crate-reach/src
    ];
  };
  sourceRoot = "${src.name}/nix-offload/offload";
  cargoLock.lockFile = ./Cargo.lock;
  # A proof through the daemon takes this df along to measure its
  # builder's store, whose sandbox has no df of its own (src/proof.rs).
  env = {
    NIX_OFFLOAD_COREUTILS = "${coreutils}";
    # A run kept to the lent builders runs this changed-checks, built from
    # the same core as nix-offload, never the one its commit names
    # (src/changed.rs).
    NIX_OFFLOAD_CHANGED_CHECKS = lib.getExe changed-checks;
    # A gate's answer runs git from the store, and checks the forge's
    # certificate against these, inside a fence that has nothing else of
    # the machine's (src/gate/fence.rs).
    NIX_OFFLOAD_GIT = lib.getExe git;
    NIX_OFFLOAD_CACERT = "${cacert}/etc/ssl/certs/ca-bundle.crt";
  }
  # The fence a gate's answer runs git in, which only Linux has.
  // lib.optionalAttrs stdenv.hostPlatform.isLinux {
    NIX_OFFLOAD_BWRAP = lib.getExe bubblewrap;
  }
  # Each upload runs in a scope of its own, whose memory bounds the cache
  # its reads of the store fill (src/there.rs).
  // lib.optionalAttrs (lib.meta.availableOn stdenv.hostPlatform systemd) {
    NIX_OFFLOAD_SYSTEMD_RUN = lib.getExe' systemd "systemd-run";
  };
  # Its tests bundle a real git tree's commits, and start a run apart with
  # setsid as a builder does, in dash as well, since most builders' sh is
  # dash (src/command.rs), and send a run far through zsh, a Mac's login
  # shell (src/there.rs).
  nativeCheckInputs = [
    dash
    git
    util-linux
    zsh
  ];
  # A Mac's Nix sandbox denies what these 30 use: a loopback listener
  # (machines, relay), and signals to a process group and the shells'
  # turns (there), so they'd fail every Mac's sandboxed update. They're
  # skipped by whole name there alone, so a test added later still runs,
  # and every Linux build runs them all. A Mac runs only the unit tests,
  # since no sandboxed Mac run has reached the one in tests/ yet.
  cargoTestFlags = lib.optionals stdenv.hostPlatform.isDarwin [ "--bins" ];
  checkFlags = lib.optionals stdenv.hostPlatform.isDarwin (
    [ "--exact" ]
    ++ map (test: "--skip=${test}") [
      "machines::tests::a_builder_beside_is_named_to_every_build_for_its_big_work_while_it_answers"
      "machines::tests::a_builder_beside_stands_as_roots_prover_says_where_one_writes"
      "machines::tests::a_lender_started_again_names_no_builder_beside_before_its_proof"
      "machines::tests::a_lent_builder_is_named_for_any_work_to_builds_that_ask"
      "machines::tests::a_standing_builder_is_named_only_to_builds_that_ask_while_it_answers"
      "machines::tests::builders_standing_and_beside_are_read_again_each_time_they_are_named"
      "relay::tests::a_far_side_that_closes_before_speaking_is_dialled_again"
      "relay::tests::a_far_side_that_never_speaks_in_time_is_hung_up_on_once"
      "relay::tests::a_name_is_listened_on_at_every_address_it_gives"
      "relay::tests::a_relay_told_to_leave_takes_no_new_connection_and_ends_once_its_last_closes"
      "relay::tests::a_relay_whose_far_side_answers_no_caller_for_long_enough_ends"
      "relay::tests::a_successors_relay_listens_on_the_port_beside_the_leaving_ones"
      "relay::tests::what_goes_up_through_a_relay_is_written_for_the_lender"
      "there::tests::a_build_sent_nearby_past_its_turns_waits_there_for_one_in_dash_and_zsh"
      "there::tests::a_build_sent_where_no_turn_can_be_made_builds_without_one_and_says_so"
      "there::tests::a_burst_of_runs_sent_nearby_never_builds_more_than_its_turns_at_once"
      "there::tests::a_macs_builds_are_counted_by_the_live_turns_of_its_own_boot"
      "there::tests::a_run_finding_no_turn_it_can_make_looks_again_before_building_without_one"
      "there::tests::a_run_sent_far_ended_as_its_watcher_ends_it_gives_its_turn_back_in_dash_and_zsh"
      "there::tests::a_run_that_loses_a_turn_between_two_looks_finding_none_looks_twice_again"
      "there::tests::a_run_that_waits_between_two_looks_finding_no_turn_looks_twice_again"
      "there::tests::a_run_waiting_as_its_turns_folder_is_removed_makes_it_again_and_takes_a_turn"
      "there::tests::a_run_waiting_beside_a_turn_it_cant_make_counts_it_apart_from_the_shells"
      "there::tests::a_run_waiting_for_a_turn_says_so_again_only_every_turn_every_seconds"
      "there::tests::a_run_whose_look_sleep_refuses_still_looks_a_second_later"
      "there::tests::a_run_whose_new_turn_is_taken_from_it_in_its_first_second_waits_again"
      "there::tests::a_run_whose_turn_is_freed_during_its_look_takes_one_again_in_its_own_boot"
      "there::tests::a_shell_whose_pid_a_dead_turn_names_takes_it_rather_than_wait_on_itself"
      "there::tests::a_stray_folder_in_one_turns_place_leaves_the_others_to_take"
      "there::tests::a_turn_whose_shell_is_gone_or_from_an_earlier_boot_is_taken_in_dash_and_zsh"
    ]
  );
  meta = {
    description = "Lends the machine a builder while its queue has builds one could take";
    license = lib.licenses.mit;
    mainProgram = "nix-offload";
  };
}
