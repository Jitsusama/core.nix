# The machine's administrators switch it to a system already built with no
# password, and nobody else can (modules/switch-system). The tests are
# switch-system-vm-test.py, which pytest runs inside the driver
# (tests/lib/vm_pytest.py).
{ self, pkgs }:
pkgs.testers.runNixOSTest {
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

  testScript = ''import vm_pytest; vm_pytest.run(globals(), "${./switch-system-vm-test.py}")'';
  extraPythonPackages = python: [ (pkgs.callPackage ./lib/vm-helpers.nix { } python) ];
  # nix fmt holds the tests and vm_pytest to mypy --strict against the
  # driver's own types, so the driver's check of this one line would add only
  # its time.
  skipTypeCheck = true;
}
