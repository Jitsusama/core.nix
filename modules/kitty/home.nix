# Joel's kitty: kitty.conf says how it behaves, and theme.conf, written here
# from jitsusama.theme, how it looks: the theme's terminal colours, and its
# colours for the frame around them, so kitty matches the rest of the desktop.
# keys.conf, written here from jitsusama.kitty.keys, holds its shortcuts,
# which also go into the keyboard map (modules/keys). Its kitty scrolls by
# touch too, which upstream's doesn't on Wayland (touch-scrolling.patch).
#
# ssh in kitty is kitty's ssh kitten, which carries the prompt to the other
# machine and sources it after that machine's own zsh setup, and tints the
# window towards the machine hue while it's connected, so a window that's
# elsewhere says so whatever draws its prompt.
{
  config,
  lib,
  pkgs,
  ...
}:
let
  inherit (config.jitsusama.theme)
    colours
    terminal
    font
    space
    ;

  # kitty measures padding in points, 72 to the inch, where the theme's
  # spaces are logical pixels, 96 to the inch.
  points = pixels: toString (pixels * 3 / 4);

  mix = import ../theme/mix.nix { inherit lib; };

  # zsh's start-up files on the other machine, each sourcing that machine's
  # own before carrying on with the prompt's. The ssh kitten sets ZDOTDIR to
  # the folder these are copied into, which is inside the kitten's own so it
  # never replaces a file of that machine's, such as its own prompt.
  carried = ".local/share/kitty-ssh-kitten/prompt";
  startup =
    name: extra:
    pkgs.writeText "carried${name}" ''
      # Carried here by kitty's ssh kitten: this machine's own ${name}, then
      # what the kitten brought.
      _carried_zdotdir=$ZDOTDIR
      ZDOTDIR=''${_carried_home_zdotdir:-$HOME}
      [[ -r $ZDOTDIR/${name} ]] && source $ZDOTDIR/${name}
      _carried_home_zdotdir=$ZDOTDIR
      ZDOTDIR=$_carried_zdotdir
      ${extra}
    '';
  carry = name: extra: "copy --dest ${carried}/${name} ${startup name extra}";

  ssh = ''
    # Written from modules/kitty: what the ssh kitten takes to every host.
    copy --dest ${carried}/prompt.zsh ${config.jitsusama.prompt.file}
    ${carry ".zshenv" ""}
    ${carry ".zprofile" ""}
    ${carry ".zshrc" "source $_carried_zdotdir/prompt.zsh"}
    ${carry ".zlogin" ""}
    env ZDOTDIR=''${HOME}/${carried}
    color_scheme elsewhere.conf
  '';

  keys = config.jitsusama.kitty.keys;

  # The hints kitten in the theme: each label is keys to type, so it takes
  # the accent, and what it points at reads in the strongest text.
  hints = lib.concatStringsSep " " [
    "kitten hints"
    "--hints-foreground-color=${colours.on_accent}"
    "--hints-background-color=${colours.accent}"
    "--hints-text-color=${colours.strong}"
  ];
in
{
  imports = [
    ../theme/home.nix
    ../keys/home.nix
    ../prompt/home.nix
    ../quickshell/recents.nix
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

  # kitty's Recent in the launcher: the folders zoxide learns as each is
  # visited, the one visited last first, as every program's Recent is
  # ordered, read as its database changes. Return opens a kitty window there, through the kitty
  # behind the window used last while one is open. The launcher watches the
  # list from inside its folder, which zoxide makes only at its first cd, so
  # it's made at sign-in and that cd counts.
  config.systemd.user.tmpfiles.rules =
    lib.optional config.programs.zoxide.enable "d ${config.xdg.dataHome}/zoxide 0700 - - -";

  config.jitsusama.quickshell.launcher.recents = lib.optional config.programs.zoxide.enable {
    app = "kitty";
    kind = "folder";
    watch = "${config.xdg.dataHome}/zoxide/db.zo";
    list = [
      (lib.getExe config.programs.zoxide.package)
      "query"
      "--list"
      "--sort-by"
      "last-accessed"
    ];
    open = [
      "kitty"
      "@"
      "--to"
      "unix:@kitty-{pid}"
      "launch"
      "--type=os-window"
      "--cwd"
      "{path}"
    ];
    start = [
      "kitty"
      "--directory"
      "{path}"
    ];
  };

  config.home.packages = [
    (pkgs.kitty.overrideAttrs (old: {
      patches = (old.patches or [ ]) ++ [ ./touch-scrolling.patch ];
    }))
  ];

  config.xdg.configFile."kitty/kitty.conf".text = builtins.readFile ./kitty.conf;

  config.xdg.configFile."kitty/ssh.conf".text = ssh;

  # The page another machine is drawn on: the theme's own, a little towards
  # the hue the prompt gives another machine.
  config.xdg.configFile."kitty/elsewhere.conf".text = ''
    background ${mix 0.07 colours.machine terminal.background}
  '';

  # Only kitty can run the kitten, so ssh is plain ssh anywhere else.
  config.programs.zsh.initContent = ''
    [[ $TERM == xterm-kitty ]] && alias ssh='kitten ssh'
  '';

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

    active_border_color ${colours.accent}
    # The tab in view is drawn in the page's own colour against a darker
    # strip, as Chrome marks its current tab, the rest muted on the strip.
    tab_bar_background ${colours.deepest}
    active_tab_background ${colours.background}
    active_tab_foreground ${colours.strong}
    inactive_tab_background ${colours.deepest}
    inactive_tab_foreground ${colours.muted}

    action_alias hints ${hints}

    # A link under the pointer underlined in the theme's strongest text, as
    # glow underlines one, rather than in kitty's own blue curl.
    url_color ${colours.strong}
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
