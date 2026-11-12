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
# through $XDG_RUNTIME_DIR/backlight-chosen. Each level it settles at, the
# light it was heard in, what the curve asks there and whether it follows
# the light now are told in $XDG_RUNTIME_DIR/backlight-now for Screens'
# Brightness, emptied as the daemon stops.
# From jitsusama.backlight.evening.start to its end, by the local clock,
# the built-in screen eases to half the brightness the light asks for, as
# the light research found dimming does most for the body clock
# (src/evening.rs); a pick then is learned as the day would want it.
# A monitor on a DisplayPort or MST bus that answers DDC/CI follows the
# panel at the same share of its own brightest (src/monitor/), once
# udev opens its bus to the seat, as modules/ddcutil does. What such a
# monitor says of itself (src/facts.rs) is kept in $XDG_RUNTIME_DIR/desk
# for the launcher's Displays facts, which desk-facts adds to its own
# (../desk-facts).
# Where a sensor sees the light's colour, the built-in panel's warmth
# follows the room part of the way (src/warmth.rs), and from sunset to
# sunrise where the laptop is the night light warms it whatever the sensor
# sees (src/night.rs), to jitsusama.backlight.night.kelvin, both through
# wl-gammarelay-rs, which holds niri's gamma for each screen and takes its
# warmth over the session's bus. It's the daemon's own, so it comes with it,
# and the bus starts it the first time the daemon sets a warmth
# (nixos.nix), rather than every session: a compositor that can't set
# gamma, as a nested one can't, would fail it at each start.
# Whether it follows the room, and whether the night light is on, are
# Joel's to say, through the daemon's own name on the session's bus
# (src/switch.rs), which the shell's Warmth sets, and how it stands is kept
# in $XDG_RUNTIME_DIR/backlight-warmth for the shell: {} while nothing
# warms the screen, so no Warmth shows.
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

  nightWhite = toString config.jitsusama.backlight.night.kelvin;

  evening =
    let
      inherit (config.jitsusama.backlight) evening;
    in
    if evening.start == null then "off" else "${evening.start}-${evening.end}";

  clock = lib.types.strMatching "([01][0-9]|2[0-3]):[0-5][0-9]";
in
{
  options.jitsusama.backlight.night.kelvin = lib.mkOption {
    type = lib.types.ints.between 1000 6500;
    # GNOME's Night Light's, gentle enough that code's colours still read
    # (VKCF3Z, 2026-11-08).
    default = 4000;
    description = ''
      The built-in screen's white at night, in kelvin, eased into over the
      half hour after sunset and out of over the half hour before sunrise;
      6500, the screen's own, keeps it from warming at all.
    '';
  };

  options.jitsusama.backlight.evening.start = lib.mkOption {
    type = lib.types.nullOr clock;
    # Three hours before a midnight bed, the evening the light research
    # (RSCH-20261108-HGB8JZ) keeps dim, until Joel picks his own bedtime.
    default = "21:30";
    description = ''
      When the built-in screen starts to dim for the evening, by the local
      clock, as "21:30": it eases to half the brightness the light asks for
      over the half hour after, never under about 20 nits. Null keeps it
      from dimming at all.
    '';
  };

  options.jitsusama.backlight.evening.end = lib.mkOption {
    type = clock;
    default = "05:00";
    description = ''
      When the evening's dimming ends, by the local clock: the screen eases
      back to the brightness the light asks for over the half hour after.
    '';
  };

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
    # Started by the bus alone, and again by the daemon's next warmth should
    # it stop, so nothing restarts it in a loop where it can't run.
    Service = {
      # Started once it holds its name, so the daemon finds it there.
      Type = "dbus";
      BusName = "rs.wl-gammarelay";
      ExecStart = lib.getExe pkgs.wl-gammarelay-rs;
    };
  };

  config.systemd.user.services.backlight = {
    Unit = {
      Description = "Sets the screen's brightness and warmth for the light";
      PartOf = [ "graphical-session.target" ];
      After = [ "graphical-session.target" ];
    };
    Service = {
      ExecStart = "${lib.getExe package} ${standard} ${nightWhite} ${evening}";
      Restart = "on-failure";
      RestartSec = 1;
      # A monitor's facts go with the daemon however it ends, a stop or a
      # crash, so none is shown that nobody keeps. The warmth's note is
      # emptied rather than removed, since a file the shell watches is never
      # seen again once it's gone, even when it comes back.
      ExecStopPost = [
        "${pkgs.runtimeShell} -c '${pkgs.coreutils}/bin/rm -f %t/desk/ddc-*'"
        "${pkgs.runtimeShell} -c 'echo {} > %t/backlight-warmth'"
        "${pkgs.runtimeShell} -c ': > %t/backlight-now'"
      ];
    };
    Install.WantedBy = [ "graphical-session.target" ];
  };
}
