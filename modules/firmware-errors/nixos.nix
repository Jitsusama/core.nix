# Keeps what the firmware recorded about the last boot's end, readable without root.
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
# boot, each is copied into /var/lib/firmware-errors/<boot id>/, where the
# account reads it after the fact, whatever has rebooted since. A machine
# without either table (a VM, an Apple machine) copies nothing and is fine.
# A boot's copy is about 30 KB, and a quarter year's are kept.
{ pkgs, ... }:
let
  tables = "/sys/firmware/acpi/tables";
  keep =
    from: name: "-${pkgs.coreutils}/bin/install -D -m 0644 ${from} %S/firmware-errors/%b/${name}";
in
{
  systemd.services.firmware-errors = {
    description = "Keeps what the firmware recorded about the last boot's end";
    wantedBy = [ "multi-user.target" ];
    serviceConfig = {
      Type = "oneshot";
      RemainAfterExit = true;
      StateDirectory = "firmware-errors";
      StateDirectoryMode = "0755";
      # Each copy is optional: the leading "-" lets a machine without the
      # table pass, and the others still run.
      ExecStart = [
        (keep "${tables}/data/BERT" "bert-region")
        (keep "${tables}/BERT" "bert-table")
        (keep "${tables}/PHAT" "phat-table")
      ];
      ProtectHome = true;
      PrivateNetwork = true;
      NoNewPrivileges = true;
    };
  };

  systemd.tmpfiles.settings.firmware-errors."/var/lib/firmware-errors".e.age = "90d";
}
