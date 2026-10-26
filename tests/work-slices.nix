# The machine's administrators change how the system's work slices share
# the machine with no password, and nothing else (modules/slices). The tests
# are work-slices-vm-test.py, which pytest runs inside the driver
# (tests/lib/vm_pytest.py).
{ self, pkgs }:
import ./lib/pytest-vm-test.nix { inherit pkgs; } {
  name = "work-slices";

  nodes.machine = {
    imports = [ self.nixosModules.slices ];
    users.users.someone = {
      isNormalUser = true;
      extraGroups = [ "wheel" ];
    };
    users.users.other.isNormalUser = true;
  };

  tests = ./work-slices-vm-test.py;
}
