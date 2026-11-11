# The check that every fixed wait in a repository's tests polls or says why,
# for core and for any repository built on it, as
# `core.lib.everyFixedWaitSaysWhy pkgs`: every-fixed-wait-says-why.py, beside
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
  # How many fixed waits without a reason each test file held before the rule,
  # while each says why or becomes a wait on its event. The check fails on a
  # file holding more, and on one holding fewer until its count is lowered.
  waiting ? { },
  name ? "every-fixed-wait-says-why",
}:
runCommandLocal name { } ''
  ${lib.getExe python3} ${./every-fixed-wait-says-why.py} ${src} ${writeText "waiting.json" (builtins.toJSON waiting)}
  touch $out
''
