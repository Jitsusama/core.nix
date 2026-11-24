# The checks of run-subtests, which checks.nix gathers with every other
# folder's.
{
  lib,
  pkgs,
  ...
}:
{
  # run-subtests runs a VM test whole with pytest picking the tests named, and
  # refuses a script that doesn't hand its tests to pytest, so what it runs is
  # what was asked for (tools/run-subtests/run-subtests-test.py, against
  # stand-ins for nix and a check's driver).
  run-subtests-keeps-what-is-named =
    let
      run-subtests = pkgs.callPackage ./run-subtests.nix { };
      python = pkgs.python3.withPackages (python: [ python.pytest ]);
    in
    pkgs.runCommandLocal "run-subtests-keeps-what-is-named" { } ''
      RUN_SUBTESTS=${lib.getExe run-subtests} PYTHONDONTWRITEBYTECODE=1 \
        ${python}/bin/python -m pytest -p no:cacheprovider --import-mode=importlib \
        --quiet ${./run-subtests-test.py}
      touch $out
    '';
}
