{ wallpapers }:
let
  wallpaper = "${wallpapers.tiles}/baroque-ornate-charcoal.png";
  # Fills in the placeholder the configuration file leaves for its background.
  withWallpaper = builtins.replaceStrings [ "WALLPAPER_PATH" ] [ wallpaper ];
in
{
  # Ghostty itself comes from Homebrew; this is only its configuration.
  xdg.configFile."ghostty/config".text = withWallpaper (builtins.readFile ./config);
}
