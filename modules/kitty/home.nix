# Joel's kitty: kitty.conf says how it behaves, and theme.conf, written here
# from jitsusama.theme, how it looks. The colours map as Omarchy's own kitty
# template maps them, so kitty matches the rest of the desktop.
{ config, pkgs, ... }:
let
  inherit (config.jitsusama.theme) colors font;
in
{
  imports = [ ../theme/home.nix ];

  home.packages = [ pkgs.kitty ];

  xdg.configFile."kitty/kitty.conf".text = builtins.readFile ./kitty.conf;

  # From Omarchy's default/themed/kitty.conf.tpl, which takes the selection
  # colours from the palette's selection and bright foreground. The tab bar
  # is ours: Omarchy colours only the active tab.
  xdg.configFile."kitty/theme.conf".text = ''
    font_family family="${font.family}"
    font_size ${font.size}

    foreground ${colors.foreground}
    background ${colors.background}
    selection_foreground ${colors.bright_foreground}
    selection_background ${colors.selection}

    cursor ${colors.bright_foreground}
    cursor_text_color ${colors.background}

    active_border_color ${colors.accent}
    tab_bar_background ${colors.darker_background}
    active_tab_background ${colors.accent}
    active_tab_foreground ${colors.background}
    inactive_tab_background ${colors.dark_background}
    inactive_tab_foreground ${colors.muted}

    color0 ${colors.background}
    color1 ${colors.red}
    color2 ${colors.green}
    color3 ${colors.yellow}
    color4 ${colors.blue}
    color5 ${colors.magenta}
    color6 ${colors.cyan}
    color7 ${colors.foreground}
    color8 ${colors.muted}
    color9 ${colors.bright_red}
    color10 ${colors.bright_green}
    color11 ${colors.bright_yellow}
    color12 ${colors.bright_blue}
    color13 ${colors.bright_magenta}
    color14 ${colors.bright_cyan}
    color15 ${colors.bright_foreground}
  '';
}
