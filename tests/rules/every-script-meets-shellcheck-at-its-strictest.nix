# The check that every shell script in a repository meets shellcheck's optional
# checks that find faults, for core and for any repository built on it, as
# `core.lib.everyScriptMeetsShellcheckAtItsStrictest pkgs`:
# every-script-meets-shellcheck-at-its-strictest.py, beside this, says which.
{
  lib,
  python3,
  runCommandLocal,
  shellcheck,
  writeText,
}:
{
  # The repository's tree, as its flake sees it.
  src,
  # How many findings each script held before the rule, while each is fixed.
  # The check fails on a file holding more, and on one holding fewer until
  # its count is lowered.
  waiting ? { },
  # Scripts named without a shell's extension, as treefmt's shellcheck
  # includes them, each a path from the tree.
  includes ? [ ],
  name ? "every-script-meets-shellcheck-at-its-strictest",
}:
runCommandLocal name { nativeBuildInputs = [ shellcheck ]; } ''
  ${lib.getExe python3} ${./every-script-meets-shellcheck-at-its-strictest.py} ${src} ${writeText "waiting.json" (builtins.toJSON waiting)} ${lib.escapeShellArgs includes}
  touch $out
''
