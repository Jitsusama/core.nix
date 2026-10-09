# Programs that ask fontconfig for "monospace" without naming a font of their
# own get the theme's font, as they do when Omarchy sets one.
{ config, ... }:
{
  imports = [ ../theme/home.nix ];

  fonts.fontconfig = {
    enable = true;
    defaultFonts.monospace = [ config.jitsusama.theme.font.family ];
  };
}
