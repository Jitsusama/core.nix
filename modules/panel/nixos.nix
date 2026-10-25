# Under niri, the laptop's panel drops to 60 Hz once the battery falls below
# 30%, until the charger is back (refresh.sh says how). At this size 60 Hz
# saved about 1.4 W against 120 Hz on the XPS 14, a tenth of what it draws in
# light use, but scrolling isn't as smooth, so it waits for a battery running
# low.
#
# A panel wider than sRGB is niri's to map (modules/niri's srgb-gamut), for
# every program at once, so no program is told the panel's colours: one that
# converts to them as well would convert twice.
{
  config,
  lib,
  pkgs,
  ...
}:
{
  config = lib.mkIf config.programs.niri.enable {
    systemd.user.services.panel-refresh = {
      description = "Slow the panel on a low battery";
      # niri's session, whose socket the service reaches niri through.
      wantedBy = [ "graphical-session.target" ];
      partOf = [ "graphical-session.target" ];
      after = [ "graphical-session.target" ];
      path = [
        config.programs.niri.package
        config.systemd.package
        pkgs.jq
        (lib.getBin pkgs.glib)
      ];
      serviceConfig = {
        ExecStart = "${lib.getExe pkgs.bash} ${./refresh.sh}";
        Restart = "on-failure";
        RestartSec = 10;
      };
    };
  };
}
