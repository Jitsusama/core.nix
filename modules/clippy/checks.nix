# The checks of clippy, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  clippyConfig,
  clippyLints,
  lib,
  onLinux,
  pkgs,
  ...
}:
onLinux {
  # clippy finds nothing in the programs that are one Rust file on the
  # standard library alone, their tests included, held to clippyLints.
  rust-programs-pass-clippy =
    let
      programs = [
        ../ergodox/ergodox-watch.rs
        ../linux-id/pinentry.rs
        ../litra/litra.rs
        ../quickshell/keyboard-lights.rs
        ../quickshell/quests.rs
        ../quickshell/runs.rs
        ../touchpad/click-strength.rs
        ../tuple/people.rs
      ];
      clippy = "clippy-driver --edition 2024 --emit=metadata -o $TMPDIR/program";
    in
    pkgs.runCommandLocal "rust-programs-pass-clippy"
      {
        nativeBuildInputs = [
          pkgs.clippy
          pkgs.rustc
        ];
        CLIPPY_CONF_DIR = clippyConfig;
      }
      ''
        for program in ${lib.escapeShellArgs programs}; do
          ${clippy} ${lib.escapeShellArgs clippyLints} "$program"
          ${clippy} --test ${lib.escapeShellArgs clippyLints} "$program"
        done
        touch $out
      '';
}
