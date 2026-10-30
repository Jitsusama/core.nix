# The clock the firmware starts from. At every power-on the firmware sets the
# CMOS clock from the Surface Aggregator's own clock, less an offset of its
# own, so what the kernel wrote to the CMOS clock is gone at the next start,
# and the Aggregator's clock, which nothing set, ran 531 days behind.
# surface-clock keeps the Aggregator's clock at the true time, which makes the
# next start's CMOS clock right; src/main.rs says how.
{ lib, pkgs, ... }:
let
  clock = pkgs.callPackage ./package.nix { };
  run = command: "${lib.getExe clock} ${command}";
  # It reaches only the Aggregator's device, the clocks' state in sysfs and its
  # own folder.
  confined = {
    StateDirectory = "surface-clock";
    DevicePolicy = "closed";
    DeviceAllow = [ "/dev/surface/aggregator rw" ];
    NoNewPrivileges = true;
    PrivateNetwork = true;
    PrivateTmp = true;
    ProtectHome = true;
    ProtectSystem = "strict";
    MemoryDenyWriteExecute = true;
    SystemCallArchitectures = "native";
  };
in
{
  # The Aggregator's character device, which passes the controller's requests
  # from root; udev tags it so the unit below can wait for it.
  boot.kernelModules = [ "surface_aggregator_cdev" ];
  services.udev.extraRules = ''
    SUBSYSTEM=="misc", KERNEL=="surface_aggregator", TAG+="systemd"
  '';

  # It reads the offset before timesyncd starts, since once the time is
  # synchronised the kernel writes it to the CMOS clock, and the firmware's
  # difference is gone until the next start. It sets the Aggregator's clock
  # as the system stops.
  systemd.services.surface-clock = {
    description = "Keeps the clock the firmware starts from at the true time";
    unitConfig.DefaultDependencies = false;
    wants = [ "dev-surface-aggregator.device" ];
    after = [
      "dev-surface-aggregator.device"
      "local-fs.target"
    ];
    before = [
      "systemd-timesyncd.service"
      "shutdown.target"
    ];
    conflicts = [ "shutdown.target" ];
    wantedBy = [ "sysinit.target" ];
    serviceConfig = confined // {
      Type = "oneshot";
      RemainAfterExit = true;
      ExecStart = run "calibrate";
      ExecStop = run "keep";
    };
  };

  # And every hour, so a crash or a held power button, which skip the stop,
  # leave it only an hour's drift out.
  systemd.services.surface-clock-keep = {
    description = "Sets the clock the firmware starts from to the true time";
    after = [ "surface-clock.service" ];
    requisite = [ "surface-clock.service" ];
    serviceConfig = confined // {
      Type = "oneshot";
      ExecStart = run "keep";
    };
  };
  systemd.timers.surface-clock-keep = {
    wantedBy = [ "timers.target" ];
    timerConfig = {
      OnBootSec = "5min";
      OnUnitActiveSec = "1h";
    };
  };
}
