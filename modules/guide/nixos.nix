# The guide on a machine it installed, for the steps that finish it there
# and the repairs after: the program, built with the machine's own systemd
# so its bootctl and cryptenroll are the ones the machine boots with, and
# what it reads about the machine (src/installed/mod.rs's Facts).
{ config, pkgs, ... }:
let
  guide = pkgs.callPackage ./package.nix { systemd = config.systemd.package; };
  showStatus = "${config.systemd.package}/bin/busctl call org.freedesktop.systemd1 /org/freedesktop/systemd1 org.freedesktop.systemd1.Manager SetShowStatus s";
in
{
  # Run by hand, the guide asks for root once through run0, which systemd
  # lets a person do only when polkit says they may.
  security.polkit.enable = true;

  environment.systemPackages = [ guide ];
  environment.etc."guide/machine.json".text = builtins.toJSON {
    disk_name = config.jitsusama.disk.name;
  };

  # Every boot, before anyone signs in, the guide looks at the machine on
  # the first console, as root, so nothing has to be remembered or asked
  # for. It shows only when there's something to do: the first boot after
  # the install, or a TPM seal the boot no longer matches. Otherwise it
  # leaves at once. It follows the unit that makes the disk's measured-boot
  # policy, which its seal has to match, and lanzaboote's, which restarts
  # the first boot to enrol the keys, and Plymouth's quitting, whose
  # splash would hide it, and holds the sign-in back until it's done;
  # Ctrl+C leaves it for the next boot.
  #
  # Where the console is the screen the guide draws on, systemd's lines
  # about the units still starting would land across it, so it asks the
  # manager to hold them while it runs and to go back to the machine's own
  # setting however it leaves, as org.freedesktop.systemd1(5) suggests for
  # SetShowStatus.
  systemd.services.guide-installed = {
    description = "Finish the machine the guide installed";
    wantedBy = [ "multi-user.target" ];
    after = [
      "plymouth-quit.service"
      "prepare-sb-auto-enroll.service"
      "systemd-pcrlock-make-policy.service"
      "systemd-user-sessions.service"
    ];
    before = [
      "display-manager.service"
      "getty@tty1.service"
    ];
    serviceConfig = {
      Type = "oneshot";
      ExecStartPre = "${showStatus} no";
      ExecStart = "${guide}/bin/guide installed";
      ExecStopPost = "${showStatus} ''";
      StandardInput = "tty";
      StandardOutput = "tty";
      StandardError = "journal";
      TTYPath = "/dev/tty1";
      TTYReset = true;
      TTYVHangup = true;
      TTYVTDisallocate = true;
      Environment = [ "TERM=linux" ];
      # Its journal and log, beside the rescue bundle the install left.
      StateDirectory = "guide";
    };
  };
}
