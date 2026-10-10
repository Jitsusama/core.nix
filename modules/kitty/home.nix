# Joel's kitty: kitty.conf says how it behaves, and theme.conf, written here
# from jitsusama.theme, how it looks: the theme's terminal colours, and its
# roles for the frame around them, so kitty matches the rest of the desktop.
{ config, pkgs, ... }:
let
  inherit (config.jitsusama.theme) roles terminal font;
in
{
  imports = [ ../theme/home.nix ];

  home.packages = [ pkgs.kitty ];

  xdg.configFile."kitty/kitty.conf".text = builtins.readFile ./kitty.conf;

  xdg.configFile."kitty/theme.conf".text = ''
    font_family family="${font.mono.family}"
    font_size ${font.mono.size}

    foreground ${terminal.foreground}
    background ${terminal.background}
    selection_foreground ${terminal.selection_foreground}
    selection_background ${terminal.selection_background}

    cursor ${terminal.cursor}
    cursor_text_color ${terminal.cursor_text}

    active_border_color ${roles.accent}
    tab_bar_background ${roles.deepest}
    active_tab_background ${roles.accent}
    active_tab_foreground ${roles.on_accent}
    inactive_tab_background ${roles.sunken}
    inactive_tab_foreground ${roles.muted}

    color0 ${terminal.black}
    color1 ${terminal.red}
    color2 ${terminal.green}
    color3 ${terminal.yellow}
    color4 ${terminal.blue}
    color5 ${terminal.magenta}
    color6 ${terminal.cyan}
    color7 ${terminal.white}
    color8 ${terminal.bright_black}
    color9 ${terminal.bright_red}
    color10 ${terminal.bright_green}
    color11 ${terminal.bright_yellow}
    color12 ${terminal.bright_blue}
    color13 ${terminal.bright_magenta}
    color14 ${terminal.bright_cyan}
    color15 ${terminal.bright_white}
  '';
}
