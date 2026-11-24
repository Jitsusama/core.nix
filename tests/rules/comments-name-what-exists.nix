# The check that every comment naming a path from one of a repository's top
# folders names what's there, but those its waiting list names: code-health's
# stale-paths measure (modules/code-health/src/stale.rs) says which, the same
# rule that counts them each hour, held here at each change, once
# comments-name-what-exists.sh, beside this, has seen it refuse a planted tree.
{
  bash,
  callPackage,
  lib,
  runCommandLocal,
  writeText,
}:
{
  # The repository's tree, as its flake sees it.
  src,
  # The comments naming a path that's gone and waiting on their owner,
  # each `file path`, listed once for each mention, while each is pointed
  # at where the thing is now. The check fails on a mention not listed, and on a line listed
  # more often than it's found.
  waiting ? [ ],
  name ? "comments-name-what-exists",
}:
let
  codeHealth = callPackage ../../modules/code-health/package.nix { };
in
runCommandLocal name { nativeBuildInputs = [ codeHealth ]; } ''
  HOME=$TMPDIR ${lib.getExe bash} ${./comments-name-what-exists.sh} ${src} \
    ${writeText "waiting" (lib.concatLines waiting)}
  touch $out
''
