# The machine's administrators switch it to a system already built with no
# password, and nobody else can (modules/switch-system). The script is
# switch-system-test.py.
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
    # Another system to switch to, built with the first.
    specialisation.switched.configuration.environment.etc.switched.text = "switched\n";
  };

  testScript = builtins.readFile ./switch-system-test.py;
}
