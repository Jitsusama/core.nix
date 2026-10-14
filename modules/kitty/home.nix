# Joel's kitty: kitty.conf says how it behaves, and theme.conf, written here
# from jitsusama.theme, how it looks: the theme's terminal colours, and its
# roles for the frame around them, so kitty matches the rest of the desktop.
# keys.conf, written here from jitsusama.kitty.keys, holds its shortcuts,
# which also go into the keyboard map (modules/keys). Its kitty scrolls by
# touch too, which upstream's doesn't on Wayland (touch-scrolling.patch).
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

  # kitty measures padding in points, 72 to the inch, where the theme's
  # spaces are logical pixels, 96 to the inch.
  points = pixels: toString (pixels * 3 / 4);

  keys = config.jitsusama.kitty.keys;

  # The hints kitten in the theme: each label is keys to type, so it takes
  # the accent, and what it points at reads in the strongest text.
  hints = lib.concatStringsSep " " [
    "kitten hints"
    "--hints-foreground-color=${roles.on_accent}"
    "--hints-background-color=${roles.accent}"
    "--hints-text-color=${roles.strong}"
  ];
in
{
  imports = [
    ../theme/home.nix
    ../keys/home.nix
  ];

  options.jitsusama.kitty.keys = lib.mkOption {
    type = lib.types.listOf (
      lib.types.submodule {
        options = {
          chord = lib.mkOption {
            type = lib.types.str;
            description = "The chord, as kitty spells it: ctrl+shift+t.";
          };
          does = lib.mkOption {
            type = lib.types.str;
            description = "What it does in plain words, as the launcher finds it.";
          };
          action = lib.mkOption {
            type = lib.types.str;
            description = "kitty's action, as kitty.conf would spell it: new_tab_with_cwd.";
          };
        };
      }
    );
    default = [ ];
    description = ''
      kitty's shortcuts, the only ones it has, keys.nix's to begin with. A
      machine's own are added to them; lib.mkForce replaces them all.
    '';
  };

  config.jitsusama.kitty.keys = import ./keys.nix { inherit lib; };

  config.jitsusama.keys.terminal = map (key: { inherit (key) chord does; }) keys;

  config.home.packages = [
    (pkgs.kitty.overrideAttrs (old: {
      patches = (old.patches or [ ]) ++ [ ./touch-scrolling.patch ];
    }))
  ];

  config.xdg.configFile."kitty/kitty.conf".text = builtins.readFile ./kitty.conf;

  config.xdg.configFile."kitty/keys.conf".text = lib.concatMapStrings (
    key: "map ${key.chord} ${key.action}\n"
  ) keys;

  config.xdg.configFile."kitty/theme.conf".text = ''
    font_family family="${font.mono.family}"
    font_size ${font.mono.size}
    window_padding_width ${points space.l} ${points space.xl}

    foreground ${terminal.foreground}
    background ${terminal.background}
    selection_foreground ${terminal.selection_foreground}
    selection_background ${terminal.selection_background}

    cursor ${terminal.cursor}
    cursor_text_color ${terminal.cursor_text}

    active_border_color ${roles.accent}
    # The tab in view is drawn in the page's own colour against a darker
    # strip, as Chrome marks its current tab, the rest muted on the strip.
    tab_bar_background ${roles.deepest}
    active_tab_background ${roles.background}
    active_tab_foreground ${roles.strong}
    inactive_tab_background ${roles.deepest}
    inactive_tab_foreground ${roles.muted}

    action_alias hints ${hints}

    # A link under the pointer underlined in the theme's strongest text, as
    # glow underlines one, rather than in kitty's own blue curl.
    url_color ${roles.strong}
    url_style straight

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
