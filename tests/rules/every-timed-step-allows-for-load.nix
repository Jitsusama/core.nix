# The check that every bound on how long a test's step took allows for how
# busy the host is: every-timed-step-allows-for-load.py, beside this, says
# what counts. It's core's alone, unlike the rules beside it, since its way
# through in Rust is core's own test-pace crate, named by path.
{
  lib,
  python3,
  runCommandLocal,
  writeText,
}:
{
  # The repository's tree, as its flake sees it.
  src,
  # How many bounds at idle each test file held before the rule, while each
  # comes to allow for the host's pace. The check fails on a file holding
  # more, and on one holding fewer until its count is lowered.
  waiting ? { },
  name ? "every-timed-step-allows-for-load",
}:
runCommandLocal name { } ''
  ${lib.getExe python3} ${./every-timed-step-allows-for-load.py} ${src} ${writeText "waiting.json" (builtins.toJSON waiting)}
  touch $out
''
