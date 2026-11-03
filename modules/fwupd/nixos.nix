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

  # fwupd reads a machine's own quirks from its state directory, which
  # is fwupd's, so the file is linked in rather than written; it's read
  # as fwupd starts. The file says why.
  systemd.tmpfiles.settings.fwupd-quirks = {
    "/var/lib/fwupd/quirks.d".d = { };
    "/var/lib/fwupd/quirks.d/hp-usb-c-controller.quirk"."L+".argument = "${
      ./hp-usb-c-controller.quirk
    }";
  };
}
