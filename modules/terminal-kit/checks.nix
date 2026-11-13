# The checks of terminal-kit, which checks.nix gathers with every other
# folder's; its opening comment says what each kind of check proves.
{
  onLinux,
  passesClippy,
  pkgs,
  ...
}:
onLinux {
  # Its tests run as it builds: the shared flags come out of a command line
  # and leave the rest in its order, ink follows --color, NO_COLOR,
  # FORCE_COLOR and the terminal in that order, each role takes the
  # terminal colour holding its pigment, a list holds a mark's cells and
  # gives way by whole items, help reads examples first, a failure reads in
  # three parts and exits with its code, and a panic is a bug.
  terminal-kit-gives-every-command-one-shape = pkgs.callPackage ./package.nix { };
  # clippy finds nothing in terminal-kit, its tests included.
  terminal-kit-passes-clippy = passesClippy "terminal-kit-passes-clippy" (
    pkgs.callPackage ./package.nix { }
  );
}
