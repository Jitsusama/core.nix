# The check that every folder of a repository says what it holds, for core
# and for any repository built on it, as `core.lib.everyFolderSaysWhatItHolds
# pkgs`: every-folder-says-what-it-holds.sh, beside this, says what counts.
{
  lib,
  bash,
  runCommandLocal,
  writeText,
}:
{
  # The repository's tree, as its flake sees it.
  src,
  # Folders still waiting for their note, each with its reason in a comment
  # where it's listed. The check fails once one has its note or is gone.
  waiting ? [ ],
  name ? "every-folder-says-what-it-holds",
}:
runCommandLocal name { } ''
  ${lib.getExe bash} ${./every-folder-says-what-it-holds.sh} ${src} ${writeText "waiting" (lib.concatLines waiting)}
  touch $out
''
