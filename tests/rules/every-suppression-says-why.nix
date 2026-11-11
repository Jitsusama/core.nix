# The check that every lint suppression in a repository says why, where it's
# made, for core and for any repository built on it, as
# `core.lib.everySuppressionSaysWhy pkgs`: every-suppression-says-why.py,
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
  # How many suppressions without a reason each file held before the rule,
  # while each gains one. The check fails on a file holding more, and on one
  # holding fewer until its count is lowered.
  waiting ? { },
  name ? "every-suppression-says-why",
}:
runCommandLocal name { } ''
  ${lib.getExe python3} ${./every-suppression-says-why.py} ${src} ${writeText "waiting.json" (builtins.toJSON waiting)}
  touch $out
''
