# The machine's administrators switch it to a system already built with no
# password, and nobody else can (modules/switch-system). The tests are
# switch-system-vm-test.py, which pytest runs inside the driver
# (tests/lib/vm_pytest.py).
{ self, pkgs }:
import ./lib/pytest-vm-test.nix { inherit pkgs; } {
  name = "switch-system";

  nodes.machine = {
    imports = [ self.nixosModules.switch-system ];
    users.users.someone = {
      isNormalUser = true;
      extraGroups = [ "wheel" ];
    };
    users.users.other.isNormalUser = true;
    # A test machine's disk was never made to boot from, so GRUB can't be
    # installed on it; a switch with no boot loader still switches.
    boot.loader.grub.enable = false;
    # Another system to switch to, built with the first.
    specialisation.switched.configuration.environment.etc.switched.text = "switched\n";
  };

  tests = ./switch-system-vm-test.py;
}
