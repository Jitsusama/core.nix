# Firmware updates through fwupd, for a machine whose maker ships them as
# UEFI capsules, through LVFS or in a remote of the machine's own. With
# lanzaboote, fwupd's EFI program is signed with the machine's keys, and PCR 0
# leaves the TPM's policy as the machine shuts down with an update waiting,
# so an update needs nothing by hand with Secure Boot on.
{
  config,
  pkgs,
  ...
}:
let
  # fwupd's own quirks, and the machine's beside them.
  quirks = pkgs.runCommand "fwupd-quirks" { } ''
    mkdir $out
    ln -s ${config.services.fwupd.package}/share/fwupd/quirks.d/* $out/
    ln -s ${./hp-usb-c-controller.quirk} $out/hp-usb-c-controller.quirk
  '';
in
{
  services.fwupd.enable = true;

  # fwupd lets anyone in the seat's session write signed firmware with no
  # answer, and an administrator any firmware, and logind lets them set
  # the next boot to open the firmware's setup; so would anything running
  # as the account, an agent included. Writing firmware, changing a device's
  # settings and restarting into the setup ask an administrator instead,
  # which in the session is a YubiKey's touch or the password
  # (modules/yubikey/administers.nix), kept for a few minutes so an update
  # of several devices asks once. The rules sit in 10-nixos.rules, which
  # polkit reads before fwupd's own, so these answer first. A plain restart
  # still asks nothing.
  security.polkit.enable = true;
  security.polkit.extraConfig = builtins.readFile ./firmware-asks.rules;

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

  # A switch stops a changed unit before systemd reads the new units and
  # starts it after, and fwupd starts on D-Bus whenever anything asks it
  # something. A question in that gap, from the shell or the firmware
  # notice, starts the old daemon from the old unit, and a new client then
  # refuses it as another version until the next restart. Restarting it
  # once the new units are read leaves no gap.
  systemd.services.fwupd.stopIfChanged = false;

  # fwupd reads its quirks from its package's folder, which this one
  # replaces with the package's quirks and the machine's own side by side;
  # the file says why it's there. Its other folder, in /var/lib/fwupd,
  # can't hold them: fwupd-refresh owns that directory, so tmpfiles
  # refuses to make anything root's inside it.
  systemd.services.fwupd.environment.FWUPD_DATADIR_QUIRKS = "${quirks}";
}
