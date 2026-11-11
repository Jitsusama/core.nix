# Joel's kitty: kitty.conf says how it behaves, and theme.conf, written here
# from jitsusama.theme, how it looks: the theme's terminal colours, and its
# colours for the frame around them, so kitty matches the rest of the desktop.
# keys.conf, written here from jitsusama.kitty.keys, holds its shortcuts,
# which also go into the keyboard map (modules/keys).
#
# ssh in kitty is kitty's ssh kitten, which carries the prompt to the other
# machine and sources it after that machine's own zsh setup, and tints the
# window towards the machine hue while it's connected, so a window that's
# elsewhere says so whatever draws its prompt.
#
# In the launcher, Programs › kitty holds kitty's own as well (Kitty.qml):
# every tab open in it, each pane's folder and what runs there, a new window
# in one of the repositories under ~/src, and one running ssh to a machine
# ~/.ssh/config or ~/.ssh/known_hosts names.
{ quickshell }:
{
  config,
  lib,
  pkgs,
  ...
}:
let
  # kitty, which scrolls by touch on Wayland itself since 0.50.
  package = pkgs.kitty;

  inherit (config.jitsusama.theme)
    colours
    terminal
    font
    space
    ;

  # kitty measures padding in points, 72 to the inch, where the theme's
  # spaces are logical pixels, 96 to the inch.
  points = pixels: toString (pixels * 3 / 4);

  # The text keeps the inset from the window's frame on every side, to the
  # ink, as every viewer's does (docs/design.md, The Ladder), but kitty pads
  # to its cells, and a cell is taller and wider than its letters: the mono
  # face's ascent stands 0.265 of its em above its capitals and its descent
  # 0.25 below the baseline, and an H's ink is 0.033 in from either side of
  # its cell. Those are MonaspiceXe's own, measured on it: another face
  # needs measuring again. The padding is the inset less them, top, right,
  # bottom and left.
  em = builtins.fromJSON font.mono.size * 4 / 3;
  padding = lib.concatMapStringsSep " " (ink: points (space.rung.inset - ink * em)) [
    0.265
    3.3e-2
    0.25
    3.3e-2
  ];

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
    # The launcher, which holds kitty's own as a provider.
    quickshell
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

  config.jitsusama.quickshell.launcher.recents = [
    # What every kitty has open, which its page reads through the
    # programs daemon, read again only once kitty's watcher says it
    # changed (tells.py).
    {
      app = "kitty";
      name = "open";
      asked = true;
      told = true;
      list = [
        (lib.getExe' (pkgs.callPackage ../open-with/recent-files.nix { }) "open-now")
        "kitty"
      ];
    }
  ]
  ++ lib.optional config.programs.zoxide.enable {
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

  config.home.packages = [ package ];

  # kitty's own in Programs › kitty (Kitty.qml): its tabs and panes as its
  # remote control lists them, which open-now asks every kitty for at once,
  # a new window in a repository under ~/src, most recently visited first
  # where zoxide knows one, and one running ssh to a machine ~/.ssh/config
  # or ~/.ssh/known_hosts names, by how many times zsh's history ran ssh
  # to it. The programs daemon reads each as the shell starts and as the
  # launcher opens, zoxide's database whole, since only it holds when each
  # folder was visited, which a repository's row says.
  config.jitsusama.quickshell.launcher.providers.kitty = {
    order = 52;
    source = ./Kitty.qml;
    open = "listed:kitty:open";
    zoxideData = lib.optionalString config.programs.zoxide.enable "${config.xdg.dataHome}/zoxide/db.zo";
    history = lib.optionalString config.programs.zsh.enable config.programs.zsh.history.path;
    ssh = "${config.home.homeDirectory}/.ssh";
    src = "${config.home.homeDirectory}/src";
    kitten = lib.getExe' package "kitten";
    kitty = lib.getExe package;
  };
  config.xdg.configFile."quickshell/launcher/Kitty.js".source = ./Kitty.js;
  config.xdg.configFile."quickshell/launcher/KittyOpens.js".source = ./KittyOpens.js;

  config.xdg.configFile."kitty/kitty.conf".text = builtins.readFile ./kitty.conf;
  config.xdg.configFile."kitty/tells.py".source = ./tells.py;

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

  # kitty draws the face without its contextual alternates, which are the
  # mono face's texture healing: a healed letter is wider than its cell,
  # and kitty shrinks it to fit, so the m of "limit" would draw small.
  # The bold and italic faces are chosen by kitty and take these features
  # too, beside the ligature sets fontconfig gives every face.
  config.xdg.configFile."kitty/theme.conf".text = ''
    font_family family="${font.mono.family}" features="-calt"
    font_size ${font.mono.size}
    # The robot AI spend wears, which pi's footer marks a budget with:
    # MonaspiceXe's proportional cut inks it 1.48 cells wide, and kitty
    # would draw it over the space after it, where the footer holds every
    # mark to one cell. Held to one, kitty scales it into its cell.
    narrow_symbols U+F06A9 1
    # The inset less the cell's room beyond its letters, by MonaspiceXe's
    # metrics (0.265, 0.25 and 0.033 of an em): another face needs them
    # measured again.
    window_padding_width ${padding}
    # The part of a cell a window's size leaves over goes at the bottom and
    # the right, so the text starts on one column, the inset in, whatever
    # the window's size: room held at the far end rather than shared out.
    placement_strategy top-left

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
