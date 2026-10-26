# The checks of disk-loss-panic, which checks.nix gathers with every other
# folder's; its opening comment says what each kind of check proves.
{
  onLinux,
  pkgs,
  ...
}:
onLinux {
  # The program reads a record's level past its facility, counts a warning
  # or worse from the disk's path or a hung task as trouble and nothing
  # else, sees an unarchived kernel log in pstore, and finds a disk that's
  # gone failing its read.
  the-disk-loss-panic-tells-trouble-apart =
    pkgs.runCommandCC "the-disk-loss-panic-tells-trouble-apart" { nativeBuildInputs = [ pkgs.rustc ]; }
      ''
        rustc --edition 2024 --test -D warnings -o tests ${./disk-loss-panic.rs}
        ./tests
        touch $out
      '';
}
