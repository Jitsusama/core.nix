# Keeps what the firmware recorded about the last boot's end, readable without sudo.
#
# When the hardware stops a machine for an error the kernel never sees, a
# machine check that resets the processor or a device's fatal error, the
# firmware writes a record of it into the Boot Error Record Table, BERT, and
# hands it to the next boot. The kernel prints a record only when it's under
# 1 KB; a bigger one, such as Intel's crash logs, it skips with "Skipped 1
# error records", and the record itself is left in
# /sys/firmware/acpi/tables/data/BERT, readable by root alone and gone at the
# next boot. The platform health table, PHAT, says why the last reset
# happened, and is root's alone too.
#
# On 8 and 17 November the XPS 14 powered itself off mid-work with nothing
# in the journal, and each next boot found one skipped record. So at every
# boot, each is copied (keep.sh) into /var/lib/firmware-errors/<boot id>/,
# where an administrator's account reads it after the fact, whatever has
# rebooted since. The kernel keeps them from every account because a record
# may hold a processor's registers and memory addresses, so the copies are
# the wheel group's to read, not everyone's. A boot's copy is about 30 KB,
# and a quarter year's are kept.
#
# The copy runs as early as /var is mounted, before the rest of the
# machine's services start, so a second cut soon after a boot finds the
# first's record kept.
{ pkgs, ... }:
let
  keep = pkgs.writeShellApplication {
    name = "firmware-errors-keep";
    runtimeInputs = [ pkgs.coreutils ];
    text = builtins.readFile ./keep.sh;
  };
in
{
  systemd.services.firmware-errors = {
    description = "Keeps what the firmware recorded about the last boot's end";
    wantedBy = [ "sysinit.target" ];
    after = [ "local-fs.target" ];
    before = [
      "sysinit.target"
      "shutdown.target"
    ];
    conflicts = [ "shutdown.target" ];
    unitConfig.DefaultDependencies = false;
    serviceConfig = {
      Type = "oneshot";
      RemainAfterExit = true;
      ExecStart = "${keep}/bin/firmware-errors-keep";
      # Root reads the tables, which are root's own; the group is wheel's so
      # every folder and copy it makes is the administrators' to read.
      Group = "wheel";
      UMask = "0027";
      StateDirectory = "firmware-errors";
      StateDirectoryMode = "0750";
      CapabilityBoundingSet = "";
      NoNewPrivileges = true;
      ProtectSystem = "strict";
      ProtectHome = true;
      PrivateTmp = true;
      PrivateNetwork = true;
      PrivateDevices = true;
      ProtectKernelModules = true;
      ProtectControlGroups = true;
      RestrictAddressFamilies = "none";
      SystemCallArchitectures = "native";
    };
  };

  systemd.tmpfiles.settings.firmware-errors."/var/lib/firmware-errors".e.age = "90d";
}
