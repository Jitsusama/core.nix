# The checks of firmware-errors, which checks.nix gathers with every other
# folder's; its opening comment says what each kind of check proves.
{
  onLinux,
  passesClippy,
  pkgs,
  ...
}:
onLinux {
  # Its tests run as it builds: the health table's reset reason read past
  # other records and refused when cut or lying about its lengths, Intel's
  # reset cause named by its bits on the one family whose layout names
  # them, a core's timeout named from Intel's own sample Crash Log, an
  # empty, cut or foreign error region each said as what it is, how each
  # end is weighed, the kept boots listed by when they began with a bad
  # folder said rather than ending the list, keep.sh run over stand-ins
  # and read back, and the answer's JSON.
  firmware-errors-says-why = pkgs.callPackage ./package.nix { };

  # clippy finds nothing in firmware-errors, its tests included.
  firmware-errors-passes-clippy = passesClippy "firmware-errors-passes-clippy" (
    pkgs.callPackage ./package.nix { }
  );
}
