# btop in the theme's colours, slot for slot as Omarchy's template gives them,
# with the theme's roles for its frame, the terminal's colours for its boxes
# and meters, and the theme's shades for its graphs:
# https://github.com/omacom/omarchy/blob/a466dcc04f937a41c820aaa990a31f36ecaed543/default/themed/btop.theme.tpl
{ config, lib, ... }:
let
  inherit (config.jitsusama.theme) roles terminal shades;
  shade = builtins.elemAt shades;

  slots = {
    main_bg = roles.background;
    main_fg = roles.text;
    title = roles.text;
    hi_fg = roles.accent;
    selected_bg = roles.selection;
    selected_fg = roles.accent;
    inactive_fg = roles.muted;
    graph_text = shade 6;
    meter_bg = roles.selection;
    proc_misc = shade 6;
    cpu_box = terminal.magenta;
    mem_box = terminal.green;
    net_box = terminal.red;
    proc_box = roles.accent;
    div_line = roles.rule;
    temp_start = terminal.green;
    temp_mid = terminal.yellow;
    temp_end = terminal.red;
    cpu_start = terminal.cyan;
    cpu_mid = terminal.blue;
    cpu_end = terminal.magenta;
    free_start = terminal.magenta;
    free_mid = terminal.blue;
    free_end = terminal.cyan;
    cached_start = terminal.blue;
    cached_mid = terminal.cyan;
    cached_end = terminal.magenta;
    available_start = terminal.yellow;
    available_mid = terminal.red;
    available_end = terminal.red;
    used_start = terminal.green;
    used_mid = terminal.cyan;
    used_end = terminal.blue;
    download_start = terminal.yellow;
    download_mid = terminal.red;
    download_end = terminal.red;
    upload_start = terminal.green;
    upload_mid = terminal.cyan;
    upload_end = terminal.blue;
    process_start = terminal.cyan;
    process_mid = terminal.blue;
    process_end = terminal.magenta;
    # The graphs shade from the background to the brightest text.
    gradient_color_0 = shade 0;
    gradient_color_1 = shade 1;
    gradient_color_2 = shade 2;
    gradient_color_3 = shade 3;
    gradient_color_4 = shade 4;
    gradient_color_5 = shade 5;
    gradient_color_6 = shade 6;
    gradient_color_7 = shade 7;
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
