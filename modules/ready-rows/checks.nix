# The checks of ready-rows, which checks.nix gathers with every other
# folder's; its opening comment says what each kind of check proves.
{
  onLinux,
  passesClippy,
  pkgs,
  ...
}:
onLinux {
  # Its tests run as it builds: a moved row takes an order midway and the
  # rest keep theirs, a making is answered with only the records that read
  # differently and the keys that went, each figure is said as
  # numbers.json's cases say, and a time travels as its moment and form.
  ready-rows-keeps-each-rows-place = pkgs.callPackage ./package.nix { };
  # clippy finds nothing in ready-rows, its tests included.
  ready-rows-passes-clippy = passesClippy "ready-rows-passes-clippy" (
    pkgs.callPackage ./package.nix { }
  );
}
