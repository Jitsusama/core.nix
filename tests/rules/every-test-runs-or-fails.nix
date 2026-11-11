# The check that a test that can't run fails rather than passing unrun, for
# core and for any repository built on it, as
# `core.lib.everyTestRunsOrFails pkgs`: every-test-runs-or-fails.py, beside
# this, says what counts.
{
  lib,
  python3,
  runCommandLocal,
  writeText,
}:
{
  # The repository's tree, as its flake sees it.
  src,
  # How many skips each test file held before the rule, while each test comes
  # to run where it can and fail where it can't. The check fails on a file
  # holding more, and on one holding fewer until its count is lowered.
  waiting ? { },
  name ? "every-test-runs-or-fails",
}:
runCommandLocal name { } ''
  ${lib.getExe python3} ${./every-test-runs-or-fails.py} ${src} ${writeText "waiting.json" (builtins.toJSON waiting)}
  touch $out
''
