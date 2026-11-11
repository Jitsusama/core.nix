# The check that every long code file and crowded folder of a repository says
# why, for core and for any repository built on it, as
# `core.lib.everyLongFileSaysWhy pkgs`: every-long-file-says-why.py, beside
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
  # Files and folders past their limit that wait to be split. The check fails
  # once one is split, says why or is gone.
  waiting ? [ ],
  name ? "every-long-file-says-why",
}:
runCommandLocal name { } ''
  ${lib.getExe python3} ${./every-long-file-says-why.py} ${src} ${writeText "waiting" (lib.concatLines waiting)}
  touch $out
''
