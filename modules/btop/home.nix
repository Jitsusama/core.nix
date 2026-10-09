# btop in the theme's colours, slot for slot as Omarchy's template gives them:
# https://github.com/omacom/omarchy/blob/a466dcc04f937a41c820aaa990a31f36ecaed543/default/themed/btop.theme.tpl
{ config, lib, ... }:
let
  inherit (config.jitsusama.theme) colors;

  slots = {
    main_bg = colors.background;
    main_fg = colors.foreground;
    title = colors.foreground;
    hi_fg = colors.accent;
    selected_bg = colors.selection;
    selected_fg = colors.accent;
    inactive_fg = colors.muted;
    graph_text = colors.light_foreground;
    meter_bg = colors.selection;
    proc_misc = colors.light_foreground;
    cpu_box = colors.magenta;
    mem_box = colors.green;
    net_box = colors.red;
    proc_box = colors.accent;
    div_line = colors.muted;
    temp_start = colors.green;
    temp_mid = colors.yellow;
    temp_end = colors.red;
    cpu_start = colors.cyan;
    cpu_mid = colors.blue;
    cpu_end = colors.magenta;
    free_start = colors.magenta;
    free_mid = colors.blue;
    free_end = colors.cyan;
    cached_start = colors.blue;
    cached_mid = colors.cyan;
    cached_end = colors.magenta;
    available_start = colors.yellow;
    available_mid = colors.red;
    available_end = colors.red;
    used_start = colors.green;
    used_mid = colors.cyan;
    used_end = colors.blue;
    download_start = colors.yellow;
    download_mid = colors.red;
    download_end = colors.red;
    upload_start = colors.green;
    upload_mid = colors.cyan;
    upload_end = colors.blue;
    process_start = colors.cyan;
    process_mid = colors.blue;
    process_end = colors.magenta;
    # The graphs shade from the background to the brightest text.
    gradient_color_0 = colors.background;
    gradient_color_1 = colors.lighter_background;
    gradient_color_2 = colors.selection;
    gradient_color_3 = colors.muted;
    gradient_color_4 = colors.dark_foreground;
    gradient_color_5 = colors.foreground;
    gradient_color_6 = colors.light_foreground;
    gradient_color_7 = colors.bright_foreground;
  };
in
{
  imports = [ ../theme/home.nix ];

  programs.btop = {
    enable = true;
    themes.theme = lib.concatLines (
      lib.mapAttrsToList (slot: color: ''theme[${slot}]="${color}"'') slots
    );
    settings = {
      color_theme = "theme";
      # The terminal's background shows through instead.
      theme_background = false;
    };
  };
}
