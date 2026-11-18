# The machine's administrators set eight of the kernel's memory settings to
# listed values with no password, and nothing else (modules/memory). The
# tests are memory-tune-vm-test.py, which pytest runs inside the driver
# (tests/lib/vm_pytest.py).
{ self, pkgs }:
import ../../lib/pytest-vm-test.nix { inherit pkgs; } {
  name = "memory-tune";

  nodes.machine = {
    imports = [ self.nixosModules.memory ];
    # The module offers defrag_mode only on a kernel with both its fixes,
    # 7.3.4 on, newer than nixpkgs' default.
    boot.kernelPackages = pkgs.linuxPackages_latest;
    users.users.someone = {
      isNormalUser = true;
      extraGroups = [ "wheel" ];
    };
    users.users.other.isNormalUser = true;
  };

  tests = ./memory-tune-vm-test.py;
}
