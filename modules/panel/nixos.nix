# Names the colour space of the laptop's panel, for the programs that convert
# colours themselves. niri has no colour management, so every program draws
# as if the screen were sRGB, and a panel with a wider gamut stretches sRGB's
# colours over its own: reds, greens and skin come out too strong. Chromium
# converts to the screen's colours when told what they are, so the hardware
# module names the panel's, and Chrome, Slack and 1Password are told.
{
  config,
  lib,
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

  config = lib.mkIf (colourSpace != null) {
    nixpkgs.overlays = [
      (final: prev: {
        google-chrome = told final.makeWrapper prev.google-chrome [ "google-chrome-stable" ];
        slack = told final.makeWrapper prev.slack [ "slack" ];
        _1password-gui = told final.makeWrapper prev._1password-gui [ "1password" ];
      })
    ];
  };
}
