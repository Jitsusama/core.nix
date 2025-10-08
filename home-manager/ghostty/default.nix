{ lib, wallpapers, ... }:
{
  xdg.configFile."ghostty/config".text = builtins.replaceStrings
    [ "WALLPAPER_PATH" ]
    [ "${wallpapers.tiles}/baroque-ornate-charcoal.png" ]
    (builtins.readFile ./config);
}