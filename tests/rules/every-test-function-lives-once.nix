# The check that no function in a repository's test code is copied from one
# file into another, for core and for any repository built on it, as
# `core.lib.everyTestFunctionLivesOnce pkgs`: every-test-function-lives-once.py,
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
  # How many copied functions each test file held before the rule, while each
  # moves to the test library. The check fails on a file holding more, and on
  # one holding fewer until its count is lowered.
  waiting ? { },
  name ? "every-test-function-lives-once",
}:
runCommandLocal name { } ''
  ${lib.getExe python3} ${./every-test-function-lives-once.py} ${src} ${writeText "waiting.json" (builtins.toJSON waiting)}
  touch $out
''
