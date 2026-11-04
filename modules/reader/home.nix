# The reader, for reading a markdown file as a page: a quest's sheet, a plan,
# a README. Typst sets it as one tall page with his type on one pitch, its
# tables laid out from the words in them, its code in the theme's colours,
# its diagrams and equations drawn in place and its pictures beside it or
# on the web, fetched once without cookies and kept. The GPU scrolls it, so
# a frame never waits on setting or drawing. It reads and never writes,
# sets the page again as the file is saved, keeping the same words where
# they were, and opens a file where it was last closed, marks and all.
#
# Its keys are Neovim's own, from the key language every surface of ours
# speaks (../keys/language.toml), which the reader reads as it builds: j
# and k a line, Ctrl+D and Ctrl+U half a screen, Ctrl+F and Ctrl+B a screen,
# gg and G, counts, / and ? to search with n and N, marks, '' and Ctrl+O and
# Ctrl+I through the jumps, g x to follow a link by its label, q, ZZ or
# Space w to close, and Space ? or F1 for every key. v and V select, from the
# first word in view, with the terminal's cursor, moved by the arrows and
# h j k l, w b e, 0 $ and { }, with iw aw, is as, ip ap and i` for text
# objects and o for the other end; y or Ctrl+C copies the selection as the
# file's markdown through wl-copy, and at rest y y and y i p copy the
# first block in view, or its paragraph, each flashing what it took for
# 200 ms, as his Neovim's yank does. A link to another markdown
# file sets it in the same window, as Neovim's gf, keeping the place on the
# first, and Ctrl+O and Ctrl+I go back and forth across files. Escape only cancels. The same table, as the launcher lists it over
# the reader's window, is below, and a check holds the two alike
# (reader-says-the-keys-it-takes).
#
# Every colour and face is the theme's, written here as reader/theme.json
# in the configuration home, which the reader reads as it starts: its
# words and headings in the serif face, the words at its large step, and
# code in the mono; the page on the background, headings in the strong text, code on the sunken
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
  inherit (config.jitsusama.theme)
    colours
    font
    terminal
    type
    ;

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
      action
      program
      machine
      person
      voice
      selection
      ;
    # Visual mode's cursor is the terminal's, as kitty and Neovim draw it.
    inherit (terminal) cursor cursor_text;
    faces = {
      text = font.serif.family;
      code = font.mono.family;
    };
    # The serif's large step, which sets its capitals level with the
    # mono's at that step.
    sizes.text = builtins.fromJSON type.serif.large;
    # One tracking for every eyebrow, the launcher's plate's and the page's.
    eyebrow.tracking = type.eyebrow.tracking;
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
  # launcher's groups, each with the key language's act it does: a mark's
  # keys sit with the jumps, since two keys make no group of their own, and
  # moving is Scrolling there, since Moving is the picture viewer's. An
  # arrow comes before its letter. Zoom is the reader's own, as Vim has
  # none.
  key = group: act: chord: does: {
    inherit
      group
      act
      chord
      does
      ;
  };
  moving = key "scrolling";
  jumping = key "jumping";
  searching = key "searching";
  selecting = key "selecting";
  objects = key "objects";
  copying = key "copying";
  page = key "page";
  keys = [
    (moving "down" "Down" "Down a line")
    (moving "down" "j" "Down a line")
    (moving "scroll-down" "Ctrl+E" "Down a line")
    (moving "up" "Up" "Up a line")
    (moving "up" "k" "Up a line")
    (moving "scroll-up" "Ctrl+Y" "Up a line")
    (moving "half-down" "Ctrl+D" "Down half a screen")
    (moving "half-up" "Ctrl+U" "Up half a screen")
    (moving "screen-down" "Ctrl+F" "Down a screen")
    (moving "screen-down" "Page_Down" "Down a screen")
    (moving "screen-up" "Ctrl+B" "Up a screen")
    (moving "screen-up" "Page_Up" "Up a screen")
    (jumping "first" "g g" "To the top, or a count's line")
    (jumping "last" "G" "To the end, or a count's line")
    (jumping "last-jump" "' '" "Back to before the last jump")
    (jumping "last-jump" "` `" "Back to before the last jump")
    (jumping "jump-back" "Ctrl+O" "Back through the jumps")
    (jumping "jump-forward" "Ctrl+I" "Forward through the jumps")
    (jumping "links" "g x" "Follow a link on screen, by its label")
    (jumping "set-mark" "m" "Mark this place with a letter")
    (jumping "go-to-mark" "'" "Go to a letter's mark")
    (jumping "go-to-mark" "`" "Go to a letter's mark")
    (searching "search-forward" "/" "Search down the page")
    (searching "search-backward" "?" "Search up the page")
    (searching "next" "n" "On to the next match")
    (searching "previous" "N" "Back to the match before")
    (selecting "select" "v" "Select from the first word in view")
    (selecting "select-lines" "V" "Select whole paragraphs, items and lines")
    (selecting "left" "Left" "A character left")
    (selecting "left" "h" "A character left")
    (selecting "right" "Right" "A character right")
    (selecting "right" "l" "A character right")
    (selecting "word-on" "w" "On to the next word")
    (selecting "word-back" "b" "Back to a word's start")
    (selecting "word-end" "e" "On to a word's end")
    (selecting "line-start" "0" "To the line's start")
    (selecting "line-end" "$" "To the line's end")
    (selecting "paragraph-back" "{" "Back a paragraph")
    (selecting "paragraph-on" "}" "On a paragraph")
    (selecting "other-end" "o" "To the selection's other end")
    (objects "in-word" "i w" "A word")
    (objects "a-word" "a w" "A word and the space after it")
    (objects "in-sentence" "i s" "A sentence")
    (objects "a-sentence" "a s" "A sentence and the space after it")
    (objects "in-paragraph" "i p" "A paragraph")
    (objects "a-paragraph" "a p" "A paragraph and the blank line after it")
    (objects "in-code" "i `" "The code between backticks")
    (copying "copy-selection" "y" "Copy the selection as markdown")
    (copying "copy-selection" "Ctrl+C" "Copy the selection as markdown")
    (copying "copy" "y y" "Copy the first block in view")
    (copying "copy-paragraph" "y i p" "Copy the paragraph in view")
    (page "" "Ctrl+Equal" "Larger")
    (page "" "Ctrl+Plus" "Larger")
    (page "" "Ctrl+Minus" "Smaller")
    (page "" "Ctrl+0" "Back to its own size")
    (page "show-keys" "Space ?" "Show every key here")
    (page "show-keys" "F1" "Show every key here")
    (page "close" "q" "Close the page")
    (page "close" "Z Z" "Close the page")
    (page "close" "Space w" "Close the page")
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

    # The launcher lists these over the reader's window, by the same words,
    # and the reader is a page, as the key language has it.
    jitsusama.keys.reader = keys;
    jitsusama.keys.surfaces.reader = [ "page" ];
  };
}
