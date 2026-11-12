# The checks of health, which checks.nix gathers with every other folder's.
{
  onLinux,
  passesClippy,
  pkgs,
  ...
}:
# It reads systemd's journals, which only Linux keeps.
onLinux {
  # health's tests run as it builds: a journal line is read for its time,
  # priority, the thing that said it (the unit when systemd speaks of
  # one), a crash's program and whose it was by its slice, and a message
  # in bytes; a message's numbers and hex are taken out so its repeats
  # are one signature, and a stack trace is known by its words; a day
  # counts errors, crashes, traces and restarts apart and files each
  # thing under its kind, keeping at most a hundred signatures each; a
  # day is written only once changed, whole, a new one starting empty and
  # a late line of a day done left out; and it's shown a line a kind, the
  # gravest first, with a kind's things or a thing's signatures on asking.
  health-tallies-the-days-warnings = pkgs.callPackage ./package.nix { };
  # clippy finds nothing in health, its tests included.
  health-passes-clippy = passesClippy "health-passes-clippy" (pkgs.callPackage ./package.nix { });
}
