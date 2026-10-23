# A check's virtual machine goes ahead of the builds beside it, and a build
# asking for a range of user ids, as a container's check does, gets one
# (vm-first-vm-test.py).
{ self, pkgs }:
let
  # Python under qemu's name, so the kernel names it as qemu, and bash under
  # a compiler's.
  impostors = pkgs.runCommandLocal "impostors" { } ''
    mkdir -p $out/bin
    cp ${pkgs.python3}/bin/python3 $out/bin/qemu-system-x86_64
    cp ${pkgs.bash}/bin/bash $out/bin/cc1
  '';
  asBuild = pkgs.writeShellApplication {
    name = "as-build";
    runtimeInputs = [ pkgs.util-linux ];
    text = builtins.readFile ./as-build.sh;
  };
  # A VM's threads, and a compiler that's busy until stopped.
  fakeVm = pkgs.writeShellApplication {
    name = "fake-vm";
    runtimeEnv.PYTHONHOME = pkgs.python3;
    text = "exec ${impostors}/bin/qemu-system-x86_64 ${./vm-first-guest.py}";
  };
  fakeCompiler = pkgs.writeShellApplication {
    name = "fake-compiler";
    text = "exec ${impostors}/bin/cc1 -c 'while :; do sleep 1; done'";
  };
in
pkgs.testers.runNixOSTest {
  name = "vm-first";

  nodes.machine = {
    imports = [ self.nixosModules.nix-daemon ];
    environment.systemPackages = [
      asBuild
      fakeVm
      fakeCompiler
    ];
    environment.etc."uid-range.nix".source = ./uid-range.nix;
  };

  testScript = builtins.readFile ./vm-first-vm-test.py;
  # nix fmt holds the script to mypy --strict against the driver's own types,
  # through driver_names.pyi, which the driver's lighter check can't find.
  skipTypeCheck = true;
}
