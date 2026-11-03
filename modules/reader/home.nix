# The reader, for reading a markdown file as a page: a quest's sheet, a plan,
# a README. Typst sets it as one tall page with his type on one pitch, its
# tables laid out from the words in them, its code in the theme's colours,
# its diagrams and equations drawn in place and its pictures beside it or
# on the web, fetched once without cookies and kept. The GPU scrolls it, so
# a frame never waits on setting or drawing. It reads and never writes,
# sets the page again as the file is saved, keeping the same words where
# they were, and opens a file where it was last closed, marks and all.
#
# Its keys are Neovim's own, from one table in the reader (src/keys.rs): j
# and k a line, Ctrl+D and Ctrl+U half a screen, Ctrl+F and Ctrl+B a screen,
# gg and G, counts, / and ? to search with n and N, marks, '' and Ctrl+O and
# Ctrl+I through the jumps, q, ZZ or Space w to close, and Space ? for every
# key. Escape only cancels. The same table, as the launcher lists it over
# the reader's window, is below, and a check holds the two alike
# (reader-says-the-keys-it-takes).
#
# Every colour and face is the theme's, written here as reader/theme.json
# in the configuration home, which the reader reads as it starts: its
# text in the sans face, headings in the wide one and code in the mono; the
# page on the background, headings in the strong text, code on the sunken
# ground in the colours bat wears (../theme/tmtheme.nix), and search's marks
# as Neovim's.
#
# Each file it's started with is said to the desktop's list of files opened
# lately (../open-with/says-what-it-opens.nix), and each reader keeps the
# file it shows under the state home's reader/showing, so its Recent in the
# launcher is led by the files open now.
{
  config,
  lib,
  pkgs,
  ...
}:
let
  inherit (config.jitsusama.theme) colours font terminal;

  says = pkgs.callPackage ../open-with/says-what-it-opens.nix { };
  reader = says { package = pkgs.callPackage ./package.nix { }; };

  # Where each reader keeps the file it shows now (src/showing.rs).
  showing = "${config.xdg.stateHome}/reader/showing";

  theme = {
    inherit (colours)
      background
      sunken
      raised
      text
      strong
      muted
      rule
      accent
      on_accent
      ;
    faces = {
      text = font.sans.family;
      wide = font.wide.family;
      code = font.mono.family;
    };
    code = "${
      import ../theme/tmtheme.nix {
        inherit
          lib
          pkgs
          colours
          terminal
          ;
      } colours.sunken
    }/Jitsusama.tmTheme";
  };

  # Every key, as the reader's own list gives it (reader --keys), in the
  # launcher's groups: a mark's keys sit with the jumps, since two keys
  # make no group of their own, and moving is Scrolling there, since
  # Moving is the picture viewer's.
  key = group: chord: does: { inherit group chord does; };
  moving = key "scrolling";
  jumping = key "jumping";
  searching = key "searching";
  page = key "page";
  keys = [
    (moving "j" "Down a line")
    (moving "Down" "Down a line")
    (moving "Ctrl+E" "Down a line")
    (moving "k" "Up a line")
    (moving "Up" "Up a line")
    (moving "Ctrl+Y" "Up a line")
    (moving "Ctrl+D" "Down half a screen")
    (moving "Ctrl+U" "Up half a screen")
    (moving "Ctrl+F" "Down a screen")
    (moving "Page Down" "Down a screen")
    (moving "Ctrl+B" "Up a screen")
    (moving "Page Up" "Up a screen")
    (jumping "g g" "To the top, or a count's line")
    (jumping "G" "To the end, or a count's line")
    (jumping "' '" "Back to before the last jump")
    (jumping "` `" "Back to before the last jump")
    (jumping "Ctrl+O" "Back through the jumps")
    (jumping "Ctrl+I" "Forward through the jumps")
    (jumping "m" "Mark this place with a letter")
    (jumping "'" "Go to a letter's mark")
    (searching "/" "Search down the page")
    (searching "?" "Search up the page")
    (searching "n" "On to the next match")
    (searching "N" "Back to the match before")
    (page "Ctrl+Equal" "Larger")
    (page "Ctrl+Plus" "Larger")
    (page "Ctrl+Minus" "Smaller")
    (page "Ctrl+0" "Back to its own size")
    (page "Space ?" "Show every key here")
    (page "q" "Close the page")
    (page "Z Z" "Close the page")
    (page "Space w" "Close the page")
  ];
in
{
  imports = [
    ../keys/home.nix
    ../quickshell/recents.nix
    ../theme/home.nix
  ];

  options.jitsusama.reader.keys = lib.mkOption {
    type = lib.types.listOf (lib.types.attrsOf lib.types.str);
    readOnly = true;
    default = keys;
    description = "The reader's keys, as the launcher lists them, for the check that holds them to the reader's own.";
  };

  config = lib.mkIf pkgs.stdenv.hostPlatform.isLinux {
    home.packages = [ reader ];

    xdg.configFile."reader/theme.json".text = builtins.toJSON theme;

    # The file each reader shows now, for its Recent in the launcher to lead
    # with, kept as mpv keeps what it plays (../mpv/home.nix says why in the
    # state home).
    systemd.user.tmpfiles.rules = [ "d ${showing} 0700 - - -" ];
    jitsusama.quickshell.launcher.recents = [
      {
        app = "reader";
        kind = "file";
        opened = import ../quickshell/kept-in.nix { inherit lib pkgs; } showing;
      }
    ];

    # The launcher lists these over the reader's window, by the same words.
    jitsusama.keys.reader = keys;
  };
}
