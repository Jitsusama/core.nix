# Leaves a virtual machine idle, on the charger and on battery, and watches
# what logind does. Its battery and charger are the kernel's test_power,
# which starts it on the charger here, so it doesn't sleep before the test
# has a session to watch. The session is a desktop's as logind sees it, of
# Wayland's type, whose idle hint the test sets as swayidle does once the
# screens go off, and clears as a key would. Its sleep stands in, leaving a
# file for the test to find and returning at once, and logind waits five
# seconds of idling rather than five minutes. Idle on the charger, it stays
# awake, and unplugging the charger puts it to sleep, even as the last of a
# burst of power supply events, as a supply settling sends. On battery, a session
# still in use keeps it awake, and an idle one stays awake while a program
# holds logind's lock against sleep, as mycelium does, and sleeps once it
# lets go. Held, it still counts as idle and locks, since a lock against
# idling would stop swayidle's lock. The script is idle-sleep-vm-test.py.
{ self, pkgs }:
import ../../lib/pytest-vm-test.nix { inherit pkgs; } {
  name = "idle-sleep";

  nodes.machine =
    { lib, ... }:
    {
      imports = [ self.nixosModules.idle-sleep ];

      boot.kernelModules = [ "test_power" ];
      # Its USB supply would count as a charger too.
      boot.extraModprobeConfig = ''
        options test_power ac_online=on usb_online=off
      '';

      users.users.away = {
        isNormalUser = true;
        description = "Away from the machine";
      };

      systemd.services.systemd-suspend.serviceConfig.ExecStart = [
        ""
        "${pkgs.coreutils}/bin/touch /run/suspended"
      ];

      services.logind.settings.Login.IdleActionSec = lib.mkForce "5s";
    };

  tests = ./idle-sleep-vm-test.py;
}
