# The checks of creative-pebble-x-plus, which checks.nix gathers with every
# other folder's; its opening comment says what each kind of check proves.
{
  onLinux,
  pkgs,
  ...
}:
onLinux {
  # pebble-lights finds the speakers' control node only on firmware that
  # has the control collection, finds their sound card by its USB device,
  # reads a closed playback device as nothing playing, keeps the lighting on
  # while sound plays and for a while after unless the screen is locked,
  # frames its commands and reads the speakers' answers as they send them,
  # and lets a hand on the speakers' own button win.
  pebble-lights-follow-the-sound =
    pkgs.runCommandCC "pebble-lights-follow-the-sound" { nativeBuildInputs = [ pkgs.rustc ]; }
      ''
        rustc --edition 2024 --test -D warnings -o tests ${./lights.rs}
        ./tests
        touch $out
      '';
}
