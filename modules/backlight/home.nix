# The screen's brightness, set for the light in the room by backlight, a
# daemon of core's own (src/main.rs), which learns from the brightness Joel
# picks with the keys. It hears the light from the light sensor's buffer,
# which udev sets up (nixos.nix), and on a machine without a sensor it only
# tells the shell of picks; without a backlight it does nothing.
# Learning starts from jitsusama.backlight.standard; what it learns stays in
# the account's state, and each pick is told to the shell's brightness bar
# through $XDG_RUNTIME_DIR/backlight-chosen.
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

  config.systemd.user.services.backlight = {
    Unit = {
      Description = "Sets the screen's brightness for the light";
      PartOf = [ "graphical-session.target" ];
      After = [ "graphical-session.target" ];
    };
    Service = {
      ExecStart = "${lib.getExe package} ${standard}";
      Restart = "on-failure";
      RestartSec = 1;
    };
    Install.WantedBy = [ "graphical-session.target" ];
  };
}
