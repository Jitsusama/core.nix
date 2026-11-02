# The checks of lid-hinge, which checks.nix gathers with every other
# folder's; its opening comment says what each kind of check proves.
{
  onLinux,
  pkgs,
  ...
}:
onLinux {
  # The program believes a shut switch once the hinge last read nearly
  # shut, or a light sensor lit at the switch reads dark, or none was lit,
  # and doubts it otherwise; it reads the hinge's angle from its buffer.
  the-lid-shuts-when-the-hinge-agrees =
    pkgs.runCommandCC "the-lid-shuts-when-the-hinge-agrees" { nativeBuildInputs = [ pkgs.rustc ]; }
      ''
        rustc --edition 2024 --test -D warnings -o tests ${./lid-hinge.rs}
        ./tests
        touch $out
      '';
}
