# Ghostty, a terminal for the Mac: config says how it behaves, and theme,
# written here from jitsusama.theme, how it looks, so it draws exactly as
# kitty does on Linux.
{ wallpapers }:
{ config, lib, ... }:
let
  inherit (config.jitsusama.theme) terminal font space;

  wallpaper = "${wallpapers.tiles}/baroque-ornate-charcoal.png";
  # Fills in the placeholder the configuration file leaves for its background.
  withWallpaper = builtins.replaceStrings [ "WALLPAPER_PATH" ] [ wallpaper ];

  ansi = [
    "black"
    "red"
    "green"
    "yellow"
    "blue"
    "magenta"
    "cyan"
    "white"
    "bright_black"
    "bright_red"
    "bright_green"
    "bright_yellow"
    "bright_blue"
    "bright_magenta"
    "bright_cyan"
    "bright_white"
  ];

  # macOS counts 72 points to the inch where Linux counts 96, so the theme's
  # size is a third bigger here to draw the same.
  macSize =
    let
      tenths = builtins.floor (builtins.fromJSON font.mono.size * 40 / 3 + 0.5);
    in
    "${toString (tenths / 10)}.${toString (tenths - tenths / 10 * 10)}";
in
{
  imports = [ ../theme/home.nix ];

  # Ghostty itself comes from Homebrew; this is only its configuration.
  xdg.configFile."ghostty/config".text = withWallpaper (builtins.readFile ./config);

  xdg.configFile."ghostty/theme".text = ''
    font-family = ${font.mono.family}
    font-size = ${macSize}
    window-padding-x = ${toString space.xl}
    window-padding-y = ${toString space.l}
    background = ${terminal.background}
    foreground = ${terminal.foreground}
    cursor-color = ${terminal.cursor}
    cursor-text = ${terminal.cursor_text}
    selection-background = ${terminal.selection_background}
    selection-foreground = ${terminal.selection_foreground}
    ${lib.concatLines (lib.imap0 (index: name: "palette = ${toString index}=${terminal.${name}}") ansi)}
  '';
}
