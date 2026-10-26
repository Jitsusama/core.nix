# inlyne, for reading a markdown file as a page: a quest's sheet, a plan, a
# README, with its pictures, tables and code drawn in place. It reads and
# never writes, follows the file as it's edited, and opens a link to
# another markdown file in place, so ← comes back.
#
# Every colour is the theme's: the page on the background, headings in the
# strong text, links and ticked boxes in the accent, code on a block a step
# below the page and coloured as bat colours it (../theme/tmtheme.nix).
# Its words are in the theme's sans face and its code in the monospaced
# one.
#
# Its keys are the keyboard language's, and only these: ← and → back and
# forward through the pages followed, ↑ and ↓ a few lines, Page Up and Page
# Down a screen, Home and End the top and the bottom, + and - larger and
# smaller, 0 as it was, Ctrl+C copies what's selected, Escape or q closes,
# and ? opens a page saying all of them, which ← leaves. Two patches give
# it what it lacked: a colour for headings, and that page.
{
  config,
  lib,
  pkgs,
  ...
}:
let
  inherit (config.jitsusama.theme) roles terminal font;

  # The theme's #RRGGBB as the number inlyne reads.
  number = colour: lib.fromHexString (lib.removePrefix "#" colour);

  inlyne = pkgs.inlyne.overrideAttrs (old: {
    patches = (old.patches or [ ]) ++ [
      ./heading-colour.patch
      ./keys-page.patch
    ];
  });

  # What each key does, in the order the keys page says them.
  keys = [
    {
      chord = [ "Left" ];
      action = "HistoryPrevious";
      does = "back to the page before";
    }
    {
      chord = [ "Right" ];
      action = "HistoryNext";
      does = "forward again";
    }
    {
      chord = [ "Up" ];
      action = "ScrollUp";
      does = "up a few lines";
    }
    {
      chord = [ "Down" ];
      action = "ScrollDown";
      does = "down a few lines";
    }
    {
      chord = [ "PageUp" ];
      action = "PageUp";
      does = "up a screen";
    }
    {
      chord = [ "PageDown" ];
      action = "PageDown";
      does = "down a screen";
    }
    {
      chord = [ "Home" ];
      action = "ToTop";
      does = "the top";
    }
    {
      chord = [ "End" ];
      action = "ToBottom";
      does = "the bottom";
    }
    {
      chord = [ "=" ];
      action = "ZoomIn";
      does = "larger";
    }
    {
      chord = [ "-" ];
      action = "ZoomOut";
      does = "smaller";
    }
    {
      chord = [ "0" ];
      action = "ZoomReset";
      does = "as it was";
    }
    {
      chord = [
        "Ctrl"
        "c"
      ];
      action = "Copy";
      does = "copy what's selected";
    }
    {
      chord = [
        "Shift"
        "/"
      ];
      action = "Keys";
      does = "these keys";
    }
    {
      chord = [ "Escape" ];
      action = "Quit";
      does = "close";
    }
    {
      chord = [ "q" ];
      action = "Quit";
      does = "close";
    }
  ];

  # A chord as inlyne reads one: its last key, with the others as modifiers.
  combo =
    chord:
    if lib.length chord == 1 then
      lib.head chord
    else
      {
        key = lib.last chord;
        mod = lib.init chord;
      };

  # How the keys page draws a chord: each key in the theme's face for it
  # (../theme/keys.nix), by the name the theme knows it by, a letter as a
  # capital, and Shift+/ as the ? it types.
  themeNames = {
    PageUp = "Page_Up";
    PageDown = "Page_Down";
    "=" = "Equal";
    "-" = "Minus";
  };
  face =
    key:
    config.jitsusama.theme.keys.${themeNames.${key} or key}.text
      or (if lib.stringLength key == 1 then lib.toUpper key else key);
  said =
    chord:
    if
      chord == [
        "Shift"
        "/"
      ]
    then
      "?"
    else
      lib.concatStringsSep " " (map face chord);

  # The page ? opens: one row for each thing a key does, with every key
  # that does it, in the order the keys are listed.
  doings = lib.unique (map (key: key.does) keys);
  keysFor =
    does: lib.concatMapStringsSep " " (key: said key.chord) (lib.filter (key: key.does == does) keys);
  keysPage = pkgs.writeTextDir "keys.md" ''
    # Keys

    | Key | What it does |
    |:--|:--|
    ${lib.concatMapStringsSep "\n" (does: "| ${keysFor does} | ${does} |") doings}
  '';

  settings = {
    theme = "Dark";
    dark-theme = {
      text-color = number roles.text;
      heading-color = number roles.strong;
      background-color = number roles.background;
      code-color = number terminal.magenta;
      quote-block-color = number roles.sunken;
      link-color = number roles.accent;
      select-color = number roles.selection;
      checkbox-color = number roles.accent;
      code-highlighter.path = "${
        import ../theme/tmtheme.nix {
          inherit
            lib
            pkgs
            roles
            terminal
            ;
        } roles.sunken
      }/Jitsusama.tmTheme";
    };
    font-options = {
      regular-font = font.sans.family;
      monospace-font = font.mono.family;
    };
    keybindings.base = map (key: [
      key.action
      (combo key.chord)
    ]) keys;
    keys-page = "${keysPage}/keys.md";
  };

  file = (pkgs.formats.toml { }).generate "inlyne.toml" settings;
in
{
  imports = [ ../theme/home.nix ];

  home.packages = [ inlyne ];

  # Where inlyne looks for its settings: the configuration folder on Linux,
  # Application Support on a Mac.
  xdg.configFile = lib.mkIf pkgs.stdenv.hostPlatform.isLinux { "inlyne/inlyne.toml".source = file; };
  home.file = lib.mkIf pkgs.stdenv.hostPlatform.isDarwin {
    "Library/Application Support/inlyne/inlyne.toml".source = file;
  };
}
