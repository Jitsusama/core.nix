# The check that every Cargo.lock names each of the repository's crates'
# dependencies as its manifest does, for core and for any repository built on
# it, as `core.lib.everyLockAgreesWithItsManifests pkgs`:
# every-lock-agrees-with-its-manifests.py, beside this, says what counts.
{
  lib,
  python3,
  runCommandLocal,
  writeText,
}:
{
  # The repository's tree, as its flake sees it.
  src,
  # The locks, by path, that disagreed with their manifests before the rule,
  # while each is refreshed. The check fails on any other lock that disagrees,
  # and on one of these once it agrees.
  waiting ? [ ],
  name ? "every-lock-agrees-with-its-manifests",
}:
runCommandLocal name { } ''
  ${lib.getExe python3} ${./every-lock-agrees-with-its-manifests.py} ${src} ${writeText "waiting.json" (builtins.toJSON waiting)}
  touch $out
''
