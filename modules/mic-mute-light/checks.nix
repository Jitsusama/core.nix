# The checks of mic-mute-light, which checks.nix gathers with every other
# folder's; its opening comment says what each kind of check proves.
{
  onLinux,
  pkgs,
  ...
}:
onLinux {
  # The light's program asks after the mute only once an array of
  # pw-dump's that touched a mute or the default source has ended, or any
  # array while the light has yet to be set, reads wpctl's mark, and
  # lights the light for a muted source alone.
  the-mic-mute-light-follows-the-mute =
    pkgs.runCommandCC "the-mic-mute-light-follows-the-mute" { nativeBuildInputs = [ pkgs.rustc ]; }
      ''
        rustc --edition 2024 --test -D warnings -o tests ${./mic-mute-light.rs}
        ./tests
        touch $out
      '';
}
