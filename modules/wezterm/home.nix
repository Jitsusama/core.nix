# WezTerm, Joel's terminal on the Mac: wezterm.lua says how it behaves, and
# theme.lua, written here from jitsusama.theme, how it looks, so it draws
# exactly as kitty does on Linux.
{ wallpapers }:
{
  config,
  lib,
  pkgs,
  ...
}:
let
  inherit (config.jitsusama.theme)
    roles
    terminal
    font
    space
    ;

  wallpaper = "${wallpapers.tiles}/baroque-ornate-charcoal.png";
  # Fills in the placeholder the configuration file leaves for its background.
  withWallpaper = builtins.replaceStrings [ "WALLPAPER_PATH" ] [ wallpaper ];

  colours = names: map (name: terminal.${name}) names;

  # macOS counts 72 points to the inch where Linux counts 96, so the theme's
  # size is a third bigger here to draw the same.
  macSize = builtins.fromJSON font.mono.size * 4 / 3;

  tab = background: foreground: {
    bg_color = background;
    fg_color = foreground;
  };
in
{
  imports = [ ../theme/home.nix ];

  home.packages = [ pkgs.wezterm ];
  xdg.configFile."wezterm/wezterm.lua".text = withWallpaper (builtins.readFile ./wezterm.lua);

  xdg.configFile."wezterm/theme.lua".text = "return ${
    lib.generators.toLua { } {
      font = font.mono.family;
      font_size = macSize;
      padding = {
        left = space.xl;
        right = space.xl;
        top = space.l;
        bottom = space.l;
      };
      colors = {
        inherit (terminal) foreground background;
        cursor_bg = terminal.cursor;
        cursor_border = terminal.cursor;
        cursor_fg = terminal.cursor_text;
        selection_bg = terminal.selection_background;
        selection_fg = terminal.selection_foreground;
        ansi = colours [
          "black"
          "red"
          "green"
          "yellow"
          "blue"
          "magenta"
          "cyan"
          "white"
        ];
        brights = colours [
          "bright_black"
          "bright_red"
          "bright_green"
          "bright_yellow"
          "bright_blue"
          "bright_magenta"
          "bright_cyan"
          "bright_white"
        ];
        # As kitty's: the tab in view on the accent, the rest quiet.
        tab_bar = {
          background = roles.deepest;
          active_tab = tab roles.accent roles.on_accent;
          inactive_tab = tab roles.sunken roles.muted;
          inactive_tab_hover = tab roles.raised roles.text;
          new_tab = tab roles.deepest roles.muted;
          new_tab_hover = tab roles.raised roles.text;
        };
      };
      frame = {
        active_titlebar_bg = roles.deepest;
        inactive_titlebar_bg = roles.deepest;
      };
    }
  }\n";
}
