# The machine's administrators switch it to a system it has run with no
# password, to any other only by authenticating, and nobody else can
# (modules/switch-system). With a YubiKey named to answer polkit and none
# plugged in, the password still does (modules/yubikey/administers.nix).
# The tests are
# switch-system-vm-test.py, which pytest runs inside the driver
# (tests/lib/vm_pytest.py).
{ self, pkgs }:
import ../../lib/pytest-vm-test.nix { inherit pkgs; } {
  name = "switch-system";

  nodes.machine = {
    imports = [
      self.nixosModules.switch-system
      self.nixosModules.yubikey
    ];
    users.users.someone = {
      isNormalUser = true;
      extraGroups = [ "wheel" ];
      # Typed to polkit's sign-in, standing in for the person's own.
      initialPassword = "typed";
    };
    # A key enrolled for someone, never plugged in.
    jitsusama.yubikey.administers.someone = [ "aGFuZGxl,cHVibGlj,es256,+presence" ];
    # pamtester signs in through polkit's PAM service as its agent would.
    environment.systemPackages = [ pkgs.pamtester ];
    users.users.other.isNormalUser = true;
    # A test machine's disk was never made to boot from, so GRUB can't be
    # installed on it; a switch with no boot loader still switches.
    boot.loader.grub.enable = false;
    # Another system to switch to, built with the first, which the tests put
    # in the system profile as update-to-main would.
    specialisation.switched.configuration.environment.etc.switched.text = "switched\n";
    # And one the machine never runs, which no test puts there.
    specialisation.unrun.configuration.environment.etc.unrun.text = "unrun\n";
  };

  tests = ./switch-system-vm-test.py;
}
