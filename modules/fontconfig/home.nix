# Programs that ask fontconfig for "monospace" or "sans-serif" without naming
# a font of their own get the theme's faces, as they do when Omarchy sets
# them, rather than fontconfig's own: a program's words and a page's plain sans in the
# account's sans face, wherever none is chosen. kitty, and so
# Neovim inside it, reads the face's features from fontconfig too, so the
# theme's features come with it: the face's coding ligatures beside its
# texture healing. After each face comes the script face, so a letter the
# theme's faces lack, Hebrew, Arabic or CJK, is drawn from Noto's face for
# its script rather than whichever font fontconfig ranks next. A face
# fontconfig doesn't know, as each of the theme's is, falls back through
# sans-serif, so that list serves every face. It's bound strongly, which
# defaultFonts can't say: a weakly bound family ranks below a font's
# language, so DejaVu Sans, which covers English as well as Hebrew, would
# still win a Hebrew letter over Noto Sans Hebrew, which covers only Hebrew.
{ config, lib, ... }:
let
  inherit (config.jitsusama.theme.font) mono sans script;
in
{
  imports = [ ../theme/home.nix ];

  fonts.fontconfig = {
    enable = true;
    defaultFonts.monospace = [ mono.family ];
    defaultFonts.sansSerif = [ sans.family ];
    # After defaultFonts' file, so its prefer comes after the theme's face.
    configFile.script = {
      enable = true;
      priority = 53;
      text = ''
        <?xml version="1.0"?>
        <!DOCTYPE fontconfig SYSTEM "urn:fontconfig:fonts.dtd">
        <fontconfig>
        ${
          lib.concatMapStrings
            (generic: ''
              <alias binding="strong">
                <family>${generic}</family>
                <prefer>
              ${lib.concatMapStrings (family: "      <family>${family}</family>\n") script.families}    </prefer>
              </alias>
            '')
            [
              "sans-serif"
              "monospace"
            ]
        }</fontconfig>
      '';
    };
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
