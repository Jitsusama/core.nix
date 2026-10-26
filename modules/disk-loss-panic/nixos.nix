# Panics the kernel once the disk the system runs from stops answering, so
# the kernel's last messages are kept in pstore (on most machines the
# firmware's variables) rather than lost with the journal. systemd-pstore
# copies them to /var/lib/systemd/pstore at the next boot. It's for chasing a
# disk that goes, imported by the machine while that's chased:
# disk-loss-panic.rs says how it tells a disk that's gone from a false alarm,
# and why it won't panic twice for one record.
#
# It starts as early as the root is mounted, before anything reads the disk
# much, and the kernel waits 10 s after a panic and restarts.
{
  config,
  lib,
  pkgs,
  ...
}:
let
  # The standard library alone, so it builds with rustc and no crates.
  diskLossPanic = pkgs.runCommandCC "disk-loss-panic" { nativeBuildInputs = [ pkgs.rustc ]; } ''
    mkdir -p $out/bin
    rustc --edition 2024 -C opt-level=3 -D warnings \
      -o $out/bin/disk-loss-panic ${./disk-loss-panic.rs}
  '';
in
{
  options.jitsusama.diskLossPanic.disk = lib.mkOption {
    type = lib.types.str;
    example = "/dev/disk/by-id/nvme-eui.0025388b91b40aa1";
    description = ''
      The disk the system runs from, as a path under /dev. A link under
      /dev/disk/by-id names it whatever order the disks were found in.
    '';
  };

  config = {
    boot.kernelParams = [ "panic=10" ];

    systemd.services.disk-loss-panic = {
      description = "Panics the kernel once the disk stops answering, so pstore keeps its log";
      wantedBy = [ "sysinit.target" ];
      # After systemd-pstore, so a record the last panic left is archived
      # and cleared before this looks whether pstore still holds one.
      after = [ "systemd-pstore.service" ];
      before = [
        "sysinit.target"
        "shutdown.target"
      ];
      conflicts = [ "shutdown.target" ];
      unitConfig.DefaultDependencies = false;
      serviceConfig = {
        ExecStart = "${diskLossPanic}/bin/disk-loss-panic ${config.jitsusama.diskLossPanic.disk}";
        Restart = "always";
        RestartSec = 1;
        # It locks itself into memory, so it needs nothing from the disk.
        LimitMEMLOCK = "infinity";
        OOMScoreAdjust = -1000;
        # Reading the kernel's log needs CAP_SYSLOG; writing to it, reading
        # the disk's device and asking sysrq to crash need only root's own
        # files.
        CapabilityBoundingSet = [
          "CAP_SYSLOG"
          "CAP_IPC_LOCK"
        ];
        NoNewPrivileges = true;
        ProtectSystem = "strict";
        ProtectHome = true;
        PrivateNetwork = true;
        RestrictAddressFamilies = "none";
        RestrictNamespaces = true;
        RestrictRealtime = true;
        LockPersonality = true;
        MemoryDenyWriteExecute = true;
        SystemCallArchitectures = "native";
      };
    };
  };
}
