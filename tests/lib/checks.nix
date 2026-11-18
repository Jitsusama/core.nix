# The checks of what tests share, proven against stand-ins, which checks.nix
# gathers with every other folder's; README.md beside this says what each
# file here is.
{
  bareNixos,
  evaluates,
  onLinux,
  pkgs,
  ...
}:
let
  python = pkgs.python3.withPackages (python: [ python.pytest ]);
in
{
  # What VM tests share (tests/lib), proven against stand-ins for the
  # driver and its machines, which record what they're handed. Named
  # without vm- so the machine queue doesn't take these milliseconds for
  # a VM test.
  what-vm-tests-share-does-what-it-says =
    pkgs.runCommandLocal "what-vm-tests-share-does-what-it-says" { }
      ''
        cd ${./.}
        PYTHONPATH=. PYTHONDONTWRITEBYTECODE=1 ${python}/bin/python -m pytest \
          -p no:cacheprovider --import-mode=importlib --quiet \
          vm-helpers-test.py vm-pytest-test.py
        touch $out
      '';

  # The other half of core.lib.evaluates, which every machine core checks
  # passes: a machine that warns fails it, saying the warning.
  a-machine-that-warns-fails-to-evaluate =
    let
      warning = "a setting here does nothing";
      failed = pkgs.testers.testBuildFailure (
        evaluates "warning-machine" (bareNixos [ { warnings = [ warning ]; } ])
      );
    in
    pkgs.runCommandLocal "a-machine-that-warns-fails-to-evaluate" { } ''
      if ! grep -qF ${pkgs.lib.escapeShellArg "warning-machine warns: ${warning}"} \
        ${failed}/testBuildFailure.log; then
        echo "evaluates failed without naming the warning; its log:"
        cat ${failed}/testBuildFailure.log
        exit 1
      fi
      touch $out
    '';
}
# The qemu wrapper waits on its sockets with inotify, which only Linux has.
// onLinux {
  # The qemu VM tests run, put in front of a stand-in qemu that says how
  # it was run.
  qemu-starts-after-its-sockets =
    let
      standIn =
        pkgs.runCommandLocal "qemu-stand-in" { } ''
          mkdir -p $out/bin
          printf '#!%s\nprintf "%%s\\n" "$@" > "$QEMU_RAN"\n' ${pkgs.runtimeShell} \
            > $out/bin/qemu-system-x86_64
          printf '#!%s\n' ${pkgs.runtimeShell} > $out/bin/qemu-img
          chmod +x $out/bin/*
        ''
        // {
          inherit (pkgs) stdenv;
        };
      qemuAfterItsSockets = pkgs.callPackage ./qemu-after-its-sockets.nix { };
    in
    pkgs.runCommandLocal "qemu-starts-after-its-sockets" { } ''
      QEMU=${qemuAfterItsSockets standIn} PYTHONDONTWRITEBYTECODE=1 \
        ${python}/bin/python -m pytest -p no:cacheprovider --import-mode=importlib \
        --quiet ${./qemu-after-its-sockets-test.py}
      touch $out
    '';
}
