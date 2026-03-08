{ lib, wallpapers, ... }:
{
  xdg.configFile."wezterm/wezterm.lua".text = builtins.replaceStrings
    [ "WALLPAPER_PATH" ]
    [ "${wallpapers.tiles}/baroque-ornate-charcoal.png" ]
    (builtins.readFile ./wezterm.lua);
}
