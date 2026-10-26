# The check that a repository's docs name only what exists, for core and for
# any repository built on it, as `core.lib.docsNameWhatExists pkgs`:
# docs-name-what-exists.sh, beside this, says what it reads and fails on.
{
  lib,
  bash,
  runCommandLocal,
  writeText,
}:
{
  # The repository's tree, as its flake sees it.
  src,
  # Mentions of something deliberately absent, each `file path`, each
  # with its reason in a comment where it's listed.
  exempt ? [ ],
  name ? "docs-name-what-exists",
}:
runCommandLocal name { } ''
  ${lib.getExe bash} ${./docs-name-what-exists.sh} ${src} ${writeText "exempt" (lib.concatLines exempt)}
  touch $out
''
