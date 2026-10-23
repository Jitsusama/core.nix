# The machine's administrators switch it to a system already built with no
# password, and nobody else can (modules/switch-system). The script is
# switch-system-vm-test.py.
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

  testScript = builtins.readFile ./switch-system-vm-test.py;
  # nix fmt holds the script to mypy --strict against the driver's own types,
  # through driver_names.pyi, which the driver's lighter check can't find.
  skipTypeCheck = true;
}
