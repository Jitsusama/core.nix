# The checks of litra, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  onLinux,
  pkgs,
  ...
}:
onLinux {
  # The Litra's command finds the lights by their HID ids, builds the
  # reports they read and reads their answers, maps brightness and warmth
  # to the light's range, and counts a camera streaming only by a file
  # that names a program, never one half-written, and never while the
  # screen is locked. Its watch takes in the
  # light's buttons and anybody's answers, asks after anybody's command,
  # and says in JSON only the lights that have said everything.
  the-litra-follows-the-cameras =
    pkgs.runCommandCC "the-litra-follows-the-cameras" { nativeBuildInputs = [ pkgs.rustc ]; }
      ''
        rustc --edition 2024 --test -D warnings -o tests ${./litra.rs}
        ./tests
        touch $out
      '';
}
