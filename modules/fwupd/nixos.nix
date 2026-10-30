# Firmware updates through fwupd, for a machine whose maker ships them as
# UEFI capsules, through LVFS or in a remote of the machine's own. With
# lanzaboote, fwupd's EFI program is signed with the machine's keys, and PCR 0
# leaves the TPM's policy as the machine shuts down with an update waiting,
# so an update needs nothing by hand with Secure Boot on.
{
  services.fwupd.enable = true;

  # The refresh asks polkit, and a switch that changes polkit's settings
  # restarts it, so a refresh its timer starts in that gap fails and the
  # switch with it. It tries again a minute later, three times in an hour
  # at most, so a machine with no network still says so.
  systemd.services.fwupd-refresh = {
    serviceConfig = {
      Restart = "on-failure";
      RestartSec = "1min";
    };
    startLimitIntervalSec = 3600;
    startLimitBurst = 3;
  };
}
