# Programs that ask fontconfig for "monospace" without naming a font of their
# own get the theme's font, as they do when Omarchy sets one. kitty, and so
# Neovim inside it, reads the face's features from fontconfig too, so the
# theme's features come with it: the face's coding ligatures beside its
# texture healing.
{ config, lib, ... }:
let
  inherit (config.jitsusama.theme.font) mono;
in
{
  imports = [ ../theme/home.nix ];

  fonts.fontconfig = {
    enable = true;
    defaultFonts.monospace = [ mono.family ];
    configFile.features = {
      enable = true;
      text = ''
        <?xml version="1.0"?>
        <!DOCTYPE fontconfig SYSTEM "urn:fontconfig:fonts.dtd">
        <fontconfig>
          <match target="font">
            <test name="family"><string>${mono.family}</string></test>
            <edit name="fontfeatures" mode="append">
        ${lib.concatMapStrings (tag: "      <string>${tag}</string>\n") mono.features}    </edit>
          </match>
        </fontconfig>
      '';
    };
  };
}
