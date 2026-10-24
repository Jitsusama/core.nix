# A check's virtual machine goes ahead of the builds beside it, each build in
# a cgroup of its own, and a build asking for a range of user ids, as a
# container's check does, gets one (vm-first-vm-test.py).
{ self, pkgs }:
let
  # Python under qemu's name, so the kernel names it as qemu, and bash under
  # a compiler's.
  impostors = pkgs.runCommandLocal "impostors" { } ''
    mkdir -p $out/bin
    cp ${pkgs.python3}/bin/python3 $out/bin/qemu-system-x86_64
    cp ${pkgs.bash}/bin/bash $out/bin/cc1
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
      fakeWriter
    ];
    environment.etc."vm-build.nix".source = buildOf "vm-build" fakeVm;
    environment.etc."compiler-build.nix".source = buildOf "compiler-build" fakeCompiler;
    environment.etc."quest-build.nix".source = buildOf "quest-build" fakeCompiler;
    environment.etc."writer-build.nix".source = buildOf "writer-build" fakeWriter;
    environment.etc."uid-range.nix".source = ./uid-range.nix;
    # getfattr, to read the mark vm-first leaves for systemd-oomd.
    environment.systemPackages = [ pkgs.attr ];
    # Looking every 20 ms rather than every second, so a hand-off racing a
    # restart of the daemon is caught rather than missed.
    systemd.services.vm-first.serviceConfig.ExecStart = pkgs.lib.mkForce "${
      pkgs.callPackage ../modules/nix-daemon/vm-first/package.nix { }
    }/bin/vm-first --every 20";
  };

  testScript = builtins.readFile ./vm-first-vm-test.py;
  # nix fmt holds the script to mypy --strict against the driver's own types,
  # through driver_names.pyi, which the driver's lighter check can't find.
  skipTypeCheck = true;
}
