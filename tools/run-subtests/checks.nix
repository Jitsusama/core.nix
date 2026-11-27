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
  # what was asked for, and that only a failure main's quarantine lists
  # blocks nothing (tools/run-subtests/run-subtests-test.py and
  # run-subtests-quarantine-test.py, against stand_ins.py's stand-ins for
  # nix, nix-offload, git and a check's driver, run from the folder so each
  # finds conftest.py's fixtures).
  run-subtests-keeps-what-is-named =
    let
      run-subtests = pkgs.callPackage ./run-subtests.nix { };
      python = pkgs.python3.withPackages (python: [ python.pytest ]);
    in
    pkgs.runCommandLocal "run-subtests-keeps-what-is-named" { } ''
      cd ${./.}
      RUN_SUBTESTS=${lib.getExe run-subtests} PYTHONPATH=. PYTHONDONTWRITEBYTECODE=1 \
        ${python}/bin/python -m pytest -p no:cacheprovider --import-mode=importlib \
        --quiet run-subtests-test.py run-subtests-quarantine-test.py
      touch $out
    '';

  # The build run-subtests --there asks for, and the merge net's searches
  # too, hands its words on to run-subtests, which reads one starting with a
  # dash as an option unless -- comes first. A stand-in check makes the
  # build's command without a VM, so this holds on any machine.
  subtests-read-no-word-as-an-option =
    let
      standIn = {
        vm-x = {
          overrideTestDerivation = change: change { };
          driver = "/stand-in/driver";
        };
      };
      made = import ./subtests.nix {
        inherit pkgs;
        checks = standIn;
      };
      built = made {
        check = "vm-x";
        words = [
          "-x"
          "the launcher"
        ];
      };
      # The command's words, read past each line break a backslash ends, so a
      # break without one stays a break and fails the match.
      said = lib.concatStringsSep " " (
        lib.filter (word: builtins.isString word && word != "") (
          builtins.split "[ \t]+" (builtins.replaceStrings [ "\\\n" ] [ " " ] built.buildCommand)
        )
      );
    in
    assert lib.assertMsg
      (lib.hasInfix "--driver /stand-in/driver --out $out -- vm-x -x 'the launcher'" said)
      "the subtests build doesn't put -- before its check and words: ${said}";
    pkgs.runCommandLocal "subtests-read-no-word-as-an-option" { } ''
      touch $out
    '';
}
