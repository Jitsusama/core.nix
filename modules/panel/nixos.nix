# Names the colour space of the laptop's panel, for the programs that convert
# colours themselves. niri has no colour management, so every program draws
# as if the screen were sRGB, and a panel with a wider gamut stretches sRGB's
# colours over its own: reds, greens and skin come out too strong. Chromium
# converts to the screen's colours when told what they are, so the hardware
# module names the panel's, and Chrome, Slack and 1Password are told.
#
# Under niri, the panel also drops to 60 Hz once the battery falls below 30%,
# until the charger is back (refresh.sh says how). At this size 60 Hz saved
# about 1.4 W against 120 Hz on the XPS 14, a tenth of what it draws in light
# use, but scrolling isn't as smooth, so it waits for a battery running low.
{
  config,
  lib,
  pkgs,
  ...
}:
let
  inherit (config.jitsusama.panel) colourSpace;

  # overrideAttrs rather than override, so the flag outlasts an override of
  # the package's own arguments, such as a machine repository's of Chrome's.
  told =
    makeWrapper: package: programs:
    package.overrideAttrs (old: {
      nativeBuildInputs = (old.nativeBuildInputs or [ ]) ++ [ makeWrapper ];
      postFixup =
        (old.postFixup or "")
        + lib.concatMapStrings (program: ''
          wrapProgram $out/bin/${program} --add-flags --force-color-profile=${colourSpace}
        '') programs;
    });
in
{
  options.jitsusama.panel.colourSpace = lib.mkOption {
    type = lib.types.nullOr (lib.types.enum [ "display-p3-d65" ]);
    default = null;
    example = "display-p3-d65";
    description = ''
      The colour space the built-in panel shows, in Chromium's
      --force-color-profile names, or null for an sRGB panel. Set by the
      hardware module from the panel's own primaries.
    '';
  };

  config = lib.mkMerge [
    (lib.mkIf (colourSpace != null) {
      nixpkgs.overlays = [
        (final: prev: {
          google-chrome = told final.makeWrapper prev.google-chrome [ "google-chrome-stable" ];
          slack = told final.makeWrapper prev.slack [ "slack" ];
          _1password-gui = told final.makeWrapper prev._1password-gui [ "1password" ];
        })
      ];
    })

    (lib.mkIf config.programs.niri.enable {
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
    })
  ];
}
