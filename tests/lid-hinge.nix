# Shuts a virtual machine's lid with its stand-in hinge coming down, left
# wide open and slammed, and watches what logind does. logind must hear
# lid-hinge's switch and not the lid's own, which udev hides: a lid shut
# with the hinge coming down sleeps; one shut with the hinge wide open and
# the light on keeps running; and one slammed shut sleeps once the light
# goes dark. The machine is on battery, from the kernel's test_power, with
# no monitor, and its sleep stands in, leaving a file and returning at
# once, as in lid.nix. The hinge and the light sensor stand in under /run:
# their attributes are files, and the hinge's buffer a named pipe the test
# writes its samples to. The hinge appears only once lid-hinge has looked
# for it, as the sensor hub's devices come at a real boot, so lid-hinge
# must find it later. The script is lid-hinge-vm-test.py.
{ self, pkgs }:
import ./lib/pytest-vm-test.nix { inherit pkgs; } {
  name = "lid-hinge";

  nodes.machine =
    { config, ... }:
    {
      imports = [
        self.nixosModules.lid
        self.nixosModules.lid-hinge
      ];

      boot.extraModulePackages = [
        (config.boot.kernelPackages.callPackage ./stand-in-lid { })
      ];
      boot.kernelModules = [
        "stand-in-lid"
        "test_power"
      ];
      boot.extraModprobeConfig = ''
        options test_power ac_online=off usb_online=off
      '';

      # lid-docked reads the connectors from here, the laptop's own screen
      # alone, so it holds nothing.
      services.udev.extraRules = ''
        SUBSYSTEM=="drm", ENV{DEVTYPE}=="drm_minor", ENV{LID_DOCKED_DRM}="/run/stand-in-drm"
      '';

      systemd.tmpfiles.settings.stand-ins = {
        "/run/stand-in-drm/card0-eDP-1/status".f.argument = "connected";
        "/run/stand-in-iio/iio:device1/name".f.argument = "als";
        "/run/stand-in-iio/iio:device1/in_illuminance_raw".f.argument = "10000";
        "/run/stand-in-iio-nodes/iio:device0".p = { };
      };
      systemd.services.lid-hinge.environment = {
        LID_HINGE_IIO = "/run/stand-in-iio";
        LID_HINGE_NODES = "/run/stand-in-iio-nodes";
      };
      # The hinge, made once lid-hinge has said what it follows, its name
      # last so it's never found half made; then a writer held open, so
      # its pipe never reads as hung up between the test's samples, which
      # would read as the hinge gone.
      systemd.services.stand-in-hinge = {
        wantedBy = [ "multi-user.target" ];
        after = [
          "systemd-tmpfiles-setup.service"
          "lid-hinge.service"
        ];
        script = builtins.readFile ./lid-hinge-stand-in-hinge.sh;
      };

      systemd.services.systemd-suspend.serviceConfig.ExecStart = [
        ""
        "${pkgs.coreutils}/bin/touch /run/suspended"
      ];

      # logind ignores the lid for a while after it starts and after every
      # sleep, which would let a lid that should sleep seem held.
      services.logind.settings.Login.HoldoffTimeoutSec = "0";
    };

  tests = ./lid-hinge-vm-test.py;
}
