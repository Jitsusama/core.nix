# The checks of what tests share, proven against stand-ins, which checks.nix
# gathers with every other folder's; README.md beside this says what each
# file here is.
{
  bareDarwin,
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

  # The halves of core.lib.evaluates that every machine core checks passes,
  # and that a repository built on core relies on: a machine that warns
  # fails it, saying the check and each warning, and so does one whose
  # assertion fails. The third warning names a package that can't build, so
  # this fails if evaluates ever builds what a warning names.
  evaluates-refuses-a-machine-that-warns =
    let
      never = pkgs.runCommandLocal "never-built" { } ''
        echo "evaluates built a package a warning names"
        exit 1
      '';
      said = [
        "warning-machine warns:"
        "a setting here does nothing"
        "another does nothing either"
        "nor does never-built, at ${builtins.storeDir}/"
      ];
      failed = pkgs.testers.testBuildFailure (
        evaluates "warning-machine" (bareNixos [
          {
            warnings = [
              "a setting here does nothing"
              "another does nothing either"
              "nor does never-built, at ${never}"
            ];
          }
        ])
      );
    in
    # Kept in Nix because it only looks for the words set just above in the
    # failed build's log, which a file of its own would take back as
    # arguments.
    pkgs.runCommandLocal "evaluates-refuses-a-machine-that-warns" { } ''
      for words in ${pkgs.lib.escapeShellArgs said}; do
        if ! grep -qF "$words" ${failed}/testBuildFailure.log; then
          echo "evaluates failed without saying '$words'; its log:"
          cat ${failed}/testBuildFailure.log
          exit 1
        fi
      done
      touch $out
    '';

  # A failed assertion throws as the machine evaluates, before anything is
  # built, so tryEval is what sees evaluates refuse it. The same machine
  # with its assertion holding has to pass evaluates, built before this is,
  # so only the assertion's value tells them apart, and a refusal for any
  # other reason fails this check's evaluation, saying why. That machine
  # holds a package that can't build, so evaluates building the machine it
  # passes fails here too.
  evaluates-refuses-a-machine-that-fails =
    let
      never = pkgs.runCommandLocal "never-built" { } ''
        echo "evaluates built the machine it passed"
        exit 1
      '';
      asserting =
        holds:
        bareNixos [
          {
            environment.systemPackages = [ never ];
            assertions = [
              {
                assertion = holds;
                message = "a setting here is wrong";
              }
            ];
          }
        ];
      holding = evaluates "holding-machine" (asserting true);
      tried = builtins.tryEval (evaluates "failing-machine" (asserting false)).drvPath;
    in
    pkgs.runCommandLocal "evaluates-refuses-a-machine-that-fails" { } (
      if tried.success then
        ''
          echo "evaluates passed a machine whose assertion fails"
          exit 1
        ''
      else
        ''
          test -e ${holding}
          touch $out
        ''
    );

  # What each bare machine sets for itself is weaker than a module's own
  # default, as the platform in a hardware file nixos-generate-config wrote,
  # so a repository can check such a module on it. Read from the machines'
  # settings, which needs nothing built.
  a-bare-machine-takes-a-modules-own-defaults =
    let
      inherit (pkgs) lib;
      nixos =
        (bareNixos [
          {
            nixpkgs.hostPlatform = lib.mkDefault "aarch64-linux";
            boot.loader.grub.enable = lib.mkDefault true;
            fileSystems."/" = lib.mkDefault {
              device = "/dev/disk/by-label/nixos";
              fsType = "ext4";
            };
            system.stateVersion = lib.mkDefault "25.11";
            home-manager.users.someone.home.stateVersion = lib.mkDefault "25.11";
          }
        ]).config;
      darwin =
        (bareDarwin [
          {
            nixpkgs.hostPlatform = lib.mkDefault "x86_64-darwin";
            system.primaryUser = lib.mkDefault "someone-else";
          }
        ]).config;
      # Its own, since what home-manager sets is read from packages for the
      # machine's platform, which nixpkgs refuses for x86_64-darwin.
      versioned =
        (bareDarwin [
          {
            system.stateVersion = lib.mkDefault 5;
            home-manager.users.someone.home.stateVersion = lib.mkDefault "25.11";
          }
        ]).config;
      home = machine: machine.home-manager.users.someone.home.stateVersion;
      kept =
        lib.optional (nixos.nixpkgs.hostPlatform.system != "aarch64-linux") "NixOS platform"
        ++ lib.optional (!nixos.boot.loader.grub.enable) "NixOS boot loader"
        ++ lib.optional (nixos.fileSystems."/".fsType != "ext4") "NixOS disk"
        ++ lib.optional (nixos.system.stateVersion != "25.11") "NixOS state version"
        ++ lib.optional (home nixos != "25.11") "NixOS home state version"
        ++ lib.optional (darwin.nixpkgs.hostPlatform.system != "x86_64-darwin") "Mac platform"
        ++ lib.optional (darwin.system.primaryUser != "someone-else") "Mac primary user"
        ++ lib.optional (versioned.system.stateVersion != 5) "Mac state version"
        ++ lib.optional (home versioned != "25.11") "Mac home state version";
      said = "the bare machine kept its own ${lib.concatStringsSep ", " kept} over a module's";
    in
    pkgs.runCommandLocal "a-bare-machine-takes-a-modules-own-defaults" { } (
      if kept == [ ] then
        "touch $out"
      else
        ''
          echo ${lib.escapeShellArg said}
          exit 1
        ''
    );
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
