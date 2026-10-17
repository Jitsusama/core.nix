# Tells each person at the machine when a device stops working, rather than
# leaving them to find it. A device whose runtime power management failed is
# held failed by the kernel, which won't wake it until the next boot, and says
# nothing: a camera gives black frames, a port goes dead. The hardware's
# module names the devices the machine should have, and one that's gone counts
# too. Each person's own service looks two minutes after they sign in and
# every two minutes after, which, since its clock stops in sleep, includes two
# minutes after every resume. It names each finding once, in the journal and
# as a notification, and again when it's fixed.
#
# A disk that's failing says so before it fails: its spare blocks run low,
# it overheats, it finds errors in what it stored, or it turns read-only. On
# NVMe those are the critical warning flags (-H), new media errors or wear
# past 95% (-f), and new errors of the disk's own in its error log (-l
# error), which smartd tells apart from commands the system sent that the
# disk didn't support. smartd reads each disk every 30 minutes, and hands a
# fault to device-check-disk, which keeps it in /run for device-check to
# name. It runs no self-tests, since Btrfs's monthly scrub reads and checks
# every block, and it doesn't write to everyone's terminals, which NixOS
# has it do unless told otherwise. On a machine with no disk it can read,
# such as a virtual machine or a USB stick, it finds nothing and stops.
{
  config,
  lib,
  pkgs,
  ...
}:
let
  deviceCheck = pkgs.callPackage ./package.nix { };
in
{
  options.jitsusama.devices.expected = lib.mkOption {
    type = lib.types.attrsOf lib.types.str;
    default = { };
    example = {
      Bluetooth = "/sys/class/bluetooth/hci0";
    };
    description = ''
      Devices the machine should have, each a path whose last part may be a
      glob, by the name to give one that's missing. The hardware's module
      names them.
    '';
  };

  config = {
    environment.etc."device-check".text = lib.concatLines (
      lib.mapAttrsToList (name: path: "${name}=${path}") config.jitsusama.devices.expected
    );
    environment.systemPackages = [ deviceCheck ];

    services.smartd = {
      enable = true;
      autodetect = true;
      # A machine with no disk smartd can read stops it cleanly, where it
      # would otherwise fail at every boot.
      extraOptions = [ "-q nodev0" ];
      notifications.wall.enable = false;
      defaults.autodetected = lib.concatStringsSep " " [
        "-H -f -l error"
        "-m <nomailer> -M exec ${deviceCheck}/bin/device-check-disk"
      ];
    };

    # Every account's, but the greeter's and the system's own.
    systemd.user.services.device-check = {
      description = "Name the devices that have stopped working";
      unitConfig.ConditionUser = "!@system";
      serviceConfig = {
        Type = "oneshot";
        ExecStart = lib.getExe deviceCheck;
      };
    };
    systemd.user.timers.device-check = {
      description = "Look for devices that have stopped working";
      wantedBy = [ "timers.target" ];
      unitConfig.ConditionUser = "!@system";
      timerConfig = {
        OnStartupSec = "2min";
        OnUnitActiveSec = "2min";
        AccuracySec = "10s";
      };
    };
  };
}
