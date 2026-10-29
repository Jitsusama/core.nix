# The screen's brightness, set for the light in the room by backlight, a
# daemon of core's own (src/main.rs), which learns from the brightness Joel
# picks with the keys. It hears the light from the light sensor's buffer,
# which udev sets up (nixos.nix), and on a machine without a sensor it only
# tells the shell of picks; without a backlight it does nothing.
# The keyboard's light, where there is one, follows the room from the same
# sensor: lit in the dark, out in daylight, and left as its key sets it
# until the light moves well away from where it was set (src/follow.rs).
# Learning starts from jitsusama.backlight.standard; what it learns stays in
# the account's state, and each pick is told to the shell's brightness bar
# through $XDG_RUNTIME_DIR/backlight-chosen.
# A monitor on a DisplayPort or MST bus that answers DDC/CI follows the
# panel at the same share of its own brightest (src/monitor.rs), once
# udev opens its bus to the seat, as modules/ddcutil does. What such a
# monitor says of itself (src/facts.rs) is kept in $XDG_RUNTIME_DIR/desk
# for the launcher's Displays facts, which desk-facts adds to its own
# (../desk-facts).
# Where a sensor sees the light's colour, the built-in panel's warmth
# follows the room part of the way (src/warmth.rs), through
# wl-gammarelay-rs, which holds niri's gamma for each screen and takes its
# warmth over the session's bus. It's the daemon's own, so it comes with it.
{
  config,
  lib,
  pkgs,
  ...
}:
let
  package = pkgs.callPackage ./package.nix { };

  standard = pkgs.writeText "backlight-standard" (
    lib.concatMapStrings (
      point: "${toString point.lux} ${toString point.level}\n"
    ) config.jitsusama.backlight.standard
  );
in
{
  options.jitsusama.backlight.standard = lib.mkOption {
    type = lib.types.listOf (
      lib.types.submodule {
        options = {
          lux = lib.mkOption {
            type = lib.types.number;
            description = "The light, in lux.";
          };
          level = lib.mkOption {
            type = lib.types.number;
            description = "The brightness in that light, as a share of the brightest, from 0 to 1.";
          };
        };
      }
    );
    # A dark room to sunlight. A pick replaces the points near it and the
    # ones it contradicts, so these only fill the light not yet picked for.
    default = [
      {
        lux = 0;
        level = 0.15;
      }
      {
        lux = 20;
        level = 0.3;
      }
      {
        lux = 150;
        level = 0.5;
      }
      {
        lux = 1000;
        level = 0.75;
      }
      {
        lux = 10000;
        level = 1;
      }
    ];
    description = ''
      The brightness for each light that learning starts from, between which
      the brightness follows the light's ratio. The first light heard learns
      the brightness the screen already has, and each brightness picked with
      the keys is learned for the light it was picked in.
    '';
  };

  config.systemd.user.services.wl-gammarelay = {
    Unit = {
      Description = "Sets the screens' warmth for the backlight daemon";
      PartOf = [ "graphical-session.target" ];
      After = [ "graphical-session.target" ];
    };
    Service = {
      # Started once it holds its name, so the daemon finds it there.
      Type = "dbus";
      BusName = "rs.wl-gammarelay";
      ExecStart = lib.getExe pkgs.wl-gammarelay-rs;
      Restart = "on-failure";
      RestartSec = 1;
    };
    Install.WantedBy = [ "graphical-session.target" ];
  };

  config.systemd.user.services.backlight = {
    Unit = {
      Description = "Sets the screen's brightness and warmth for the light";
      PartOf = [ "graphical-session.target" ];
      After = [
        "graphical-session.target"
        "wl-gammarelay.service"
      ];
      Wants = [ "wl-gammarelay.service" ];
    };
    Service = {
      ExecStart = "${lib.getExe package} ${standard}";
      Restart = "on-failure";
      RestartSec = 1;
      # A monitor's facts go with the monitor, and with the daemon however
      # it ends, a stop or a crash, so none is shown that nobody keeps.
      ExecStopPost = "${pkgs.runtimeShell} -c '${pkgs.coreutils}/bin/rm -f %t/desk/ddc-*'";
    };
    Install.WantedBy = [ "graphical-session.target" ];
  };
}
