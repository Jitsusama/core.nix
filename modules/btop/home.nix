# btop in the theme's colours, slot for slot as Omarchy's template gives them,
# with the theme's roles for its frame, the terminal's colours for its
# meters, and the theme's shades for its graphs:
# https://github.com/omacom/omarchy/blob/a466dcc04f937a41c820aaa990a31f36ecaed543/default/themed/btop.theme.tpl
#
# Its boxes are drawn as the drawer is, in a hairline of the rule colour with
# their titles in the strongest text, the accent kept for the letters that
# are keys, and the chosen row in the strongest text as the launcher's is.
# It leaves out the clock and the battery, which the shell already shows.
#
# In a window too small for every box, such as half of a laptop's screen, it
# gives way rather than refusing to draw: the network box goes, then the
# memory box, keeping the processor and the processes, and they come back
# when the window grows (gives-way.patch).
{
  config,
  lib,
  pkgs,
  ...
}:
let
  inherit (config.jitsusama.theme) roles terminal shades;
  shade = builtins.elemAt shades;

  slots = {
    main_bg = roles.background;
    main_fg = roles.text;
    title = roles.strong;
    hi_fg = roles.accent;
    selected_bg = roles.selection;
    selected_fg = roles.strong;
    inactive_fg = roles.muted;
    graph_text = shade 6;
    meter_bg = roles.selection;
    proc_misc = shade 6;
    cpu_box = roles.rule;
    mem_box = roles.rule;
    net_box = roles.rule;
    proc_box = roles.rule;
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
    package = pkgs.btop.overrideAttrs (old: {
      patches = (old.patches or [ ]) ++ [ ./gives-way.patch ];
    });
    themes.theme = lib.concatLines (
      lib.mapAttrsToList (slot: color: ''theme[${slot}]="${color}"'') slots
    );
    settings = {
      color_theme = "theme";
      # The terminal's background shows through instead.
      theme_background = false;
      clock_format = "";
      show_battery = false;
    };
  };
}
