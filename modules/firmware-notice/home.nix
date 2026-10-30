# Says, once, each firmware update fwupd offers the machine, as a notice
# from Firmware (src/main.rs says how), so an update to the BIOS, the SSD
# or a dock is heard of without looking in the launcher's Machine ›
# Firmware, where it's installed. It's asked as what's offered may have
# changed: fwupd-refresh writing LVFS's list anew, a device coming or
# going, which fwupd writes its devices down for, and each login, which
# says what was offered while the session was away. On a machine without
# fwupd there's nothing to say, and nothing is added.
{
  lib,
  pkgs,
  osConfig ? { },
  ...
}:
let
  package = pkgs.callPackage ./package.nix { };
  fwupd = osConfig.services.fwupd or { };
in
lib.mkIf (fwupd.enable or false) {
  systemd.user.paths.firmware-notice = {
    Unit.Description = "Asks what firmware fwupd offers once that may have changed";
    Path = {
      # LVFS's list, as fwupd-refresh writes it, and the devices fwupd
      # knows, as it writes them down when one comes or goes.
      PathChanged = [
        "/var/lib/fwupd/metadata/lvfs"
        "/var/cache/fwupd/devices.json"
      ];
      Unit = "firmware-notice.service";
    };
    Install.WantedBy = [ "graphical-session.target" ];
  };

  systemd.user.services.firmware-notice = {
    Unit = {
      Description = "Says each firmware update fwupd offers, once";
      # The shell says the notice, so it waits for the session.
      After = [ "graphical-session.target" ];
    };
    Service = {
      Type = "oneshot";
      ExecStart = lib.escapeShellArgs [
        (lib.getExe package)
        (lib.getExe' fwupd.package "fwupdmgr")
        (lib.getExe' pkgs.libnotify "notify-send")
      ];
      StateDirectory = "firmware-notice";
      Slice = "background.slice";
    };
    Install.WantedBy = [ "graphical-session.target" ];
  };
}
