# The check that every file read whose failure becomes nothing says why, in
# a repository's Rust outside its tests, for core and for any repository
# built on it, as `core.lib.everyFailedReadSaysWhy pkgs`:
# every-failed-read-says-why.py, beside this, says what counts.
{
  lib,
  python3,
  runCommandLocal,
  writeText,
}:
{
  # The repository's tree, as its flake sees it.
  src,
  # How many reads taking a failure as nothing each file held before the
  # rule, while each says why or says it couldn't read. The check fails on a
  # file holding more, and on one holding fewer until its count is lowered.
  waiting ? { },
  name ? "every-failed-read-says-why",
}:
runCommandLocal name { } ''
  ${lib.getExe python3} ${./every-failed-read-says-why.py} ${src} ${writeText "waiting.json" (builtins.toJSON waiting)}
  touch $out
''
