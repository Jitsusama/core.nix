# Shuts a virtual machine's lid, with and without a monitor plugged in, and
# watches what logind does. The machine has no monitor, so it reads the
# graphics card's connectors from a stand-in directory, where a test writes
# a monitor's connector plugged in or not, then says so as the card would,
# with a change on the card. Its suspend stands in too, leaving a file for
# the test to find rather than sleeping. Shut with no monitor, it suspends;
# shut with a monitor plugged in whose screen is off, which logind doesn't
# count, it keeps running, holding one lock through the card's changes;
# and unplugging that monitor with the lid still shut suspends it. The
# script is lid-vm-test.py.
{ self, pkgs }:
import ./lib/pytest-vm-test.nix { inherit pkgs; } {
  name = "lid";

  nodes.machine =
    { config, ... }:
    {
      imports = [ self.nixosModules.lid ];

      boot.extraModulePackages = [
        (config.boot.kernelPackages.callPackage ./stand-in-lid { })
      ];
      boot.kernelModules = [ "stand-in-lid" ];

      # The connectors, as udev gives lid-docked them.
      services.udev.extraRules = ''
        SUBSYSTEM=="drm", ENV{DEVTYPE}=="drm_minor", ENV{LID_DOCKED_DRM}="/run/stand-in-drm"
      '';

      systemd.services.systemd-suspend.serviceConfig.ExecStart = [
        ""
        "${pkgs.coreutils}/bin/touch /run/suspended"
      ];

      # logind ignores the lid for a while after it starts and after every
      # suspend, which would let a lid that should suspend seem held.
      services.logind.settings.Login.HoldoffTimeoutSec = "0";
    };

  tests = ./lid-vm-test.py;
}
