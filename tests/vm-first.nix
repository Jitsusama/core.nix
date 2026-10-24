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
    ];
    environment.etc."vm-build.nix".source = buildOf "vm-build" fakeVm;
    environment.etc."compiler-build.nix".source = buildOf "compiler-build" fakeCompiler;
    environment.etc."uid-range.nix".source = ./uid-range.nix;
  };

  testScript = builtins.readFile ./vm-first-vm-test.py;
  # nix fmt holds the script to mypy --strict against the driver's own types,
  # through driver_names.pyi, which the driver's lighter check can't find.
  skipTypeCheck = true;
}
