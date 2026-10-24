# A check's virtual machine goes ahead of the builds beside it, each build in
# a cgroup of its own, and a build asking for a range of user ids, as a
# container's check does, gets one (vm-first-vm-test.py).
{ self, pkgs }:
let
  # Python under qemu's name, so the kernel names it as qemu, and bash under
  # a compiler's, a container's and a service's in it.
  impostors = pkgs.runCommandLocal "impostors" { } ''
    mkdir -p $out/bin
    cp ${pkgs.python3}/bin/python3 $out/bin/qemu-system-x86_64
    cp ${pkgs.bash}/bin/bash $out/bin/cc1
    cp ${pkgs.bash}/bin/bash $out/bin/systemd-nspawn
    cp ${pkgs.bash}/bin/bash $out/bin/service
  '';
  # A VM's threads, and a compiler that's busy until stopped.
  fakeVm = pkgs.writeShellApplication {
    name = "fake-vm";
    runtimeEnv.PYTHONHOME = pkgs.python3;
    text = "exec ${impostors}/bin/qemu-system-x86_64 ${./vm-first-guest.py}";
  };
  fakeCompiler = pkgs.writeShellApplication {
    name = "fake-compiler";
    text = "exec ${impostors}/bin/cc1 -c 'while :; do ${pkgs.coreutils}/bin/sleep 1; done'";
  };
  # A container, as a check run in one starts it, with a service in it that
  # bears no name of its own to be known by.
  fakeContainer = pkgs.writeShellApplication {
    name = "fake-container";
    text = "exec ${impostors}/bin/systemd-nspawn -c '${impostors}/bin/service -c \"while :; do ${pkgs.coreutils}/bin/sleep 1; done\" & wait'";
  };
  # A build that writes 64 MiB through to the disk and then finishes.
  fakeWriter = pkgs.writeShellApplication {
    name = "fake-writer";
    # Nix gives a builder $out in its environment.
    excludeShellChecks = [ "SC2154" ];
    text = ''
      ${pkgs.coreutils}/bin/dd if=/dev/zero of=written bs=1M count=64 conv=fsync status=none
      ${pkgs.coreutils}/bin/sleep 1
      ${pkgs.coreutils}/bin/touch "$out"
    '';
  };
  # A build that writes 64 MiB and finishes without waiting for the disk.
  fakeLazyWriter = pkgs.writeShellApplication {
    name = "fake-lazy-writer";
    # Nix gives a builder $out in its environment.
    excludeShellChecks = [ "SC2154" ];
    text = ''
      ${pkgs.coreutils}/bin/dd if=/dev/zero of=written bs=1M count=64 status=none
      ${pkgs.coreutils}/bin/sleep 1
      ${pkgs.coreutils}/bin/touch "$out"
    '';
  };
  # A build that asks for its filesystem to be synced, then for everything,
  # and keeps what each answered.
  fakeSyncer = pkgs.writeShellApplication {
    name = "fake-syncer";
    # Nix gives a builder $out in its environment.
    excludeShellChecks = [ "SC2154" ];
    text = ''
      if ${pkgs.coreutils}/bin/sync -f . 2> refused; then
        echo "sync -f was let through" > "$out"
        exit 0
      fi
      ${pkgs.coreutils}/bin/sync
      { ${pkgs.coreutils}/bin/cat refused; echo "sync ran"; } > "$out"
    '';
  };
  # A build that, as a container's would, mounts its cgroup, makes one below
  # it and moves in, then asks to hand that one a controller, saying what it
  # was answered. A cgroup it makes beside that one says it has asked, and
  # it runs until stopped.
  fakeNester = pkgs.writeShellApplication {
    name = "fake-nester";
    runtimeInputs = [
      pkgs.coreutils
      pkgs.util-linux
    ];
    text = ''
      mkdir cgroup
      mount -t cgroup2 none cgroup
      mkdir cgroup/inner
      echo $$ > cgroup/inner/cgroup.procs
      if { echo +pids > cgroup/cgroup.subtree_control; } 2>&1; then
        echo "handed pids on"
      fi
      mkdir cgroup/asked
      while :; do sleep 1; done
    '';
  };
  # A build that runs one of them as the daemon runs any build, which the
  # machine's store already holds, so nothing is fetched.
  buildOf =
    name: program:
    pkgs.writeText "${name}.nix" ''
      derivation {
        name = "${name}";
        system = builtins.currentSystem;
        builder = "''${builtins.storePath "${program}"}/bin/${program.meta.mainProgram}";
      }
    '';
in
pkgs.testers.runNixOSTest {
  name = "vm-first";

  nodes.machine = {
    imports = [ self.nixosModules.nix-daemon ];
    system.extraDependencies = [
      fakeVm
      fakeCompiler
      fakeContainer
      fakeWriter
      fakeLazyWriter
      fakeSyncer
      fakeNester
    ];
    environment.etc."vm-build.nix".source = buildOf "vm-build" fakeVm;
    environment.etc."compiler-build.nix".source = buildOf "compiler-build" fakeCompiler;
    environment.etc."quest-build.nix".source = buildOf "quest-build" fakeCompiler;
    environment.etc."container-build.nix".source = buildOf "container-build" fakeContainer;
    environment.etc."writer-build.nix".source = buildOf "writer-build" fakeWriter;
    environment.etc."lazy-writer-build.nix".source = buildOf "lazy-writer-build" fakeLazyWriter;
    environment.etc."syncer-build.nix".source = buildOf "syncer-build" fakeSyncer;
    environment.etc."uid-range.nix".source = ./uid-range.nix;
    # As buildOf makes one, given a range of user ids as a container's is.
    environment.etc."nester-build.nix".text = ''
      derivation {
        name = "nester-build";
        system = builtins.currentSystem;
        builder = "''${builtins.storePath "${fakeNester}"}/bin/fake-nester";
        requiredSystemFeatures = [ "uid-range" ];
      }
    '';
    # getfattr, to read the mark vm-first leaves for systemd-oomd.
    environment.systemPackages = [ pkgs.attr ];
    # The module's free-space floor is far more than the machine's disk, so
    # every build would first collect garbage. Run by hand, the machine's
    # store is the host's whole one, and that takes longer than any wait
    # here.
    nix.settings.min-free = pkgs.lib.mkForce 0;
    # Looking every 20 ms rather than every second, so a hand-off racing a
    # restart of the daemon is caught rather than missed.
    systemd.services.vm-first.serviceConfig.ExecStart = pkgs.lib.mkForce "${
      pkgs.callPackage ../modules/nix-daemon/vm-first/package.nix { }
    }/bin/vm-first --every 20";
  };

  # What VM tests' scripts share, and pytest, which runs this one's tests (tests/lib).
  extraPythonPackages = python: [ (pkgs.callPackage ./lib/vm-helpers.nix { } python) ];

  # Run by hand, the driver takes pytest's options from PYTEST_ADDOPTS, so
  # PYTEST_ADDOPTS='-k restart' runs only the tests that name a restart.
  testScript = ''import vm_pytest; vm_pytest.run(globals(), "${./vm-first-vm-test.py}")'';
  # nix fmt holds the tests and vm_pytest to mypy --strict against the
  # driver's own types, so the driver's check of this one line would add only
  # its time.
  skipTypeCheck = true;
}
