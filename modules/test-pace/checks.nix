# The checks of test-pace, which checks.nix gathers with every other
# folder's; its opening comment says what each kind of check proves.
{
  passesClippy,
  pkgs,
  ...
}:
{
  # Its tests run as it builds: an idle host allows a step its idle bound
  # and the slack, a busy one stretches the bound by its ready tasks per
  # processor up to the most pace, a step begun on a busy host keeps that
  # pace, and a failure says the step's time against both bounds.
  test-pace-stretches-a-bound-with-the-machine = pkgs.callPackage ./package.nix { };
  # clippy finds nothing in test-pace, its tests included.
  test-pace-passes-clippy = passesClippy "test-pace-passes-clippy" (
    pkgs.callPackage ./package.nix { }
  );
}
