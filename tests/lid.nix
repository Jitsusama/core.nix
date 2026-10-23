# Shuts a virtual machine's lid, with and without a monitor plugged in, and
# watches what logind does. The machine has no monitor, so it reads the
# graphics card's connectors from a stand-in directory, where a test writes
# a monitor's connector plugged in or not, then says so as the card would,
# with a change on the card. Its suspend stands in too, leaving a file for
# the test to find rather than sleeping. Shut with no monitor, it suspends;
# shut with a monitor plugged in whose screen is off, which logind doesn't
# count, it keeps running, holding one lock through the card's changes;
# and unplugging that monitor with the lid still shut suspends it.
{ self, pkgs }:
pkgs.testers.runNixOSTest {
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

  testScript = ''
    held = "systemd-inhibit --list --no-legend --no-pager | grep -q lid-docked"
    holder = "systemctl show -P MainPID lid-docked.service"
    opened = (
        "busctl get-property org.freedesktop.login1 /org/freedesktop/login1"
        " org.freedesktop.login1.Manager LidClosed | grep -qx 'b false'"
    )
    settled = (
        "busctl get-property org.freedesktop.login1 /org/freedesktop/login1"
        " org.freedesktop.login1.Manager PreparingForSleep | grep -qx 'b false'"
        " && ! systemctl list-jobs --no-legend | grep -q suspend"
        " && systemctl show -P ActiveState systemd-suspend.service | grep -qx inactive"
    )

    def lid(shut):
        machine.succeed(f"echo {shut} > /sys/module/stand_in_lid/parameters/shut")

    def monitor(status):
        machine.succeed(
            f"echo {status} > /run/stand-in-drm/card0-DP-5/status",
            "udevadm trigger --settle --action=change /sys/class/drm/card0",
        )

    machine.wait_for_unit("multi-user.target")
    machine.succeed(
        "test -e /sys/class/drm/card0",
        "mkdir -p /run/stand-in-drm/card0-eDP-1 /run/stand-in-drm/card0-DP-5",
        "echo connected > /run/stand-in-drm/card0-eDP-1/status",
    )

    with subtest("a laptop shut with no monitor suspends"):
        monitor("disconnected")
        machine.fail(held)
        lid(1)
        machine.wait_until_succeeds("test -e /run/suspended", timeout=60)
        lid(0)
        # The stand-in suspend returns at once, and logind, finding the lid
        # still shut, may queue another, which a loaded machine starts after
        # the lid opens. The file is cleared once logind has seen the lid
        # open and isn't preparing to sleep, and no suspend is queued or
        # running.
        machine.wait_until_succeeds(opened, timeout=30)
        machine.wait_until_succeeds(settled, timeout=60)
        machine.succeed("rm /run/suspended")

    with subtest("a laptop shut at a desk keeps running once the monitor is off"):
        monitor("connected")
        machine.wait_until_succeeds(held, timeout=10)
        machine.succeed("echo disabled > /run/stand-in-drm/card0-DP-5/enabled")
        first = machine.succeed(holder).strip()
        lid(1)
        machine.sleep(5)
        # A monitor waking or sleeping changes the card again.
        monitor("connected")
        machine.sleep(5)
        t.assertEqual(machine.succeed(holder).strip(), first)
        machine.fail("test -e /run/suspended")

    with subtest("unplugging the monitor with the lid shut suspends"):
        monitor("disconnected")
        machine.wait_until_succeeds("test -e /run/suspended", timeout=60)
        machine.fail(held)
  '';
}
