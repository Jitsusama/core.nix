# The check that every per-file exception in a repository's ruff config still
# hides a finding, for core and for any repository built on it, as
# `core.lib.everyRuffExceptionHidesAFinding pkgs`:
# every-ruff-exception-hides-a-finding.py, beside this, says what counts.
{
  lib,
  python3,
  ruff,
  runCommandLocal,
}:
{
  # The repository's tree, as its flake sees it.
  src,
  # Its ruff config, a path in the tree.
  config ? "ruff.toml",
  name ? "every-ruff-exception-hides-a-finding",
}:
runCommandLocal name { nativeBuildInputs = [ ruff ]; } ''
  ${lib.getExe python3} ${./every-ruff-exception-hides-a-finding.py} ${src} ${src}/${config}
  touch $out
''
