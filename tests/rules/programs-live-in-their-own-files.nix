# The check that no program past five lines is written as a string in Nix, for
# core and for any repository built on it, as
# `core.lib.programsLiveInTheirOwnFiles pkgs`: programs-live-in-their-own-files.py,
# beside this, says what counts.
{
  lib,
  python3,
  runCommandLocal,
  writeText,
}:
{
  # The repository's tree, as its flake sees it.
  src,
  # How many programs each file held before the rule, while they move out. The
  # check fails on a file holding more, and on one holding fewer until its
  # count is lowered.
  waiting ? { },
  name ? "programs-live-in-their-own-files",
}:
runCommandLocal name { } ''
  ${lib.getExe python3} ${./programs-live-in-their-own-files.py} ${src} ${writeText "waiting.json" (builtins.toJSON waiting)}
  touch $out
''
