{ wallpapers }:
let
  wallpaper = "${wallpapers.tiles}/baroque-ornate-charcoal.png";
  # Fills in the placeholder the configuration file leaves for its background.
  withWallpaper = builtins.replaceStrings [ "WALLPAPER_PATH" ] [ wallpaper ];
in
{ pkgs, ... }:
{
  home.packages = [ pkgs.wezterm ];
  xdg.configFile."wezterm/wezterm.lua".text = withWallpaper (builtins.readFile ./wezterm.lua);
}
