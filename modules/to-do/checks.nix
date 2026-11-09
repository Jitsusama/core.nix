# The checks of the to-do daemon, which checks.nix gathers with every other
# folder's; its opening comment says what each kind of check proves.
{
  onLinux,
  passesClippy,
  pkgs,
  ...
}:
# The daemon reads Linux's inotify, /proc and sysfs, and builds only there.
onLinux {
  # The daemon's tests run as it builds: each signal is read from its
  # words and a relative file is none; a to-do is keyed by its quest and
  # id and judged only where something here can see it; a boot, a switch,
  # a switch to a commit or past it, a sign-in, a device and a file are
  # each heard, a boot or a switch only after the to-do was raised; a
  # failing check waits longer each time, to ten minutes; a check says its
  # exit and its last line; the verdicts say every figure and are the same
  # text for the same verdicts; and run over a folder of to-dos, a
  # passing check is done, a failing one waits, a file is done as it
  # comes, a boot makes one ready, and a to-do gone takes its verdict.
  to-do-clears-each-to-do-by-itself = pkgs.callPackage ./package.nix { };
  # clippy finds nothing in the to-do daemon, its tests included.
  to-do-passes-clippy = passesClippy "to-do-passes-clippy" (pkgs.callPackage ./package.nix { });
}
