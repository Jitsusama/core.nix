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
  # and says in JSON only the lights that have said everything, each with
  # its lumens and the name and serial its HID device gives. A call after
  # rhythm's wind-down lights it warm and at 70% of its own brightness,
  # whose setting it keeps by serial, and takes a setting Joel picked since
  # as his own. A light's turn is waited for within its bound, taken once
  # let go of, and left free between one litra's sends for another, and a
  # light not told is tried again ever later; its test folders are its
  # user's alone.
  the-litra-follows-the-cameras =
    pkgs.runCommandCC "the-litra-follows-the-cameras" { nativeBuildInputs = [ pkgs.rustc ]; }
      ''
        rustc --edition 2024 --test -D warnings -o tests \
          ${import ./source.nix { inherit (pkgs) lib; }}/litra.rs
        ./tests
        touch $out
      '';
}
