# The check that every assertion in a repository that can't show what each
# side held says what was wrong, for core and for any repository built on it,
# as `core.lib.everyAssertSaysWhatWasWrong pkgs`:
# every-assert-says-what-was-wrong.py, beside this, says what counts.
{
  lib,
  python3,
  runCommandLocal,
  writeText,
}:
{
  # The repository's tree, as its flake sees it.
  src,
  # How many assertions without a message each file held before the rule,
  # while each gains one. The check fails on a file holding more, and on one
  # holding fewer until its count is lowered.
  waiting ? { },
  name ? "every-assert-says-what-was-wrong",
}:
runCommandLocal name { } ''
  ${lib.getExe python3} ${./every-assert-says-what-was-wrong.py} ${src} ${writeText "waiting.json" (builtins.toJSON waiting)}
  touch $out
''
