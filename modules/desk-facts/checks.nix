# The checks of desk-facts, which checks.nix gathers with every other
# folder's; its opening comment says what each kind of check proves.
{
  onLinux,
  pkgs,
  ...
}:
onLinux {
  # desk-facts reads an HP Z27k G3's EDID as the monitor gave it, names a
  # panel by its product code when it names nothing else, says wider colours
  # against sRGB's area and finds HDR in a CTA block, reads what a USB sound
  # card offers and runs at from the kernel's stream file and hw_params,
  # writes the launcher's JSON escaped and in order, and hears only the
  # uevents and sound devices that change a fact.
  desk-facts-read-the-desk =
    pkgs.runCommandCC "desk-facts-read-the-desk" { nativeBuildInputs = [ pkgs.rustc ]; }
      ''
        rustc --edition 2024 --test -D warnings -o tests ${./facts.rs}
        ./tests
        touch $out
      '';
}
