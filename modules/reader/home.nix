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
# speaks (../keys/language.toml), which the reader reads as it builds. The
# page has a cursor, as a help buffer has, its line marked by a thin rule
# in the accent in the left margin: j and k move it a line as shown, the
# view following four lines from its edge, Ctrl+E and Ctrl+Y move the view
# alone, Ctrl+D and Ctrl+U half a screen, Ctrl+F and Ctrl+B a screen,
# gg and G, { and } a paragraph, ]] and [[ a heading, H, M and L and zt, zz
# and zb, % and N%, counts, / and ? to search with n and N, * and # for the
# word here, marks, '' and Ctrl+O and Ctrl+I through the jumps, which a
# move longer than a screen joins, g x to follow a link by its label and
# Ctrl+] the one here, gf for the file a line names, ]b and [b to the
# folder's next markdown file, gO for the headings as a list, :{n}, :q and
# :noh, ZZ, Space w or :q to close, and Space ? or F1 for every key. Each
# search, heading, mark or jump puts the cursor on the line it lands on. v
# and V select, from the cursor's line, with the terminal's cursor, moved
# by the arrows and h j k l, w b e, 0 $ and { }, with iw aw, is as, ip ap
# and i` for text objects and o for the other end; y or Ctrl+C copies the
# selection as the file's markdown through wl-copy, and at rest y y, y i p,
# y a p and y } copy the block the cursor is in, its paragraph, that and
# the blank line after it, or on to the paragraph's end, each flashing what
# it took for 200 ms, as his Neovim's yank does. q does nothing, since
# nothing records. A link to another markdown
# file sets it in the same window, as Neovim's gf, keeping the place on the
# first, and Ctrl+O and Ctrl+I go back and forth across files. Escape only
# cancels. The same table, as the launcher lists it over
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
  # moving is Scrolling there, since Moving is the picture viewer's. The
  # lines in view, and the links and files, are groups of their own, so
  # none lists more than the launcher's thirteen rows. An arrow comes before
  # its letter. Zoom is the reader's own, as Vim has none.
  key = group: act: chord: does: {
    inherit
      group
      act
      chord
      does
      ;
  };
  moving = key "scrolling";
  view = key "view";
  jumping = key "jumping";
  links = key "links";
  searching = key "searching";
  selecting = key "selecting";
  objects = key "objects";
  copying = key "copying";
  page = key "page";
  keys = [
    (moving "down" "Down" "Down a line")
    (moving "down" "j" "Down a line")
    (moving "scroll-down" "Ctrl+E" "The view down a line")
    (moving "up" "Up" "Up a line")
    (moving "up" "k" "Up a line")
    (moving "scroll-up" "Ctrl+Y" "The view up a line")
    (moving "half-down" "Ctrl+D" "Down half a screen")
    (moving "half-up" "Ctrl+U" "Up half a screen")
    (moving "screen-down" "Ctrl+F" "Down a screen")
    (moving "screen-down" "Page_Down" "Down a screen")
    (moving "screen-up" "Ctrl+B" "Up a screen")
    (moving "screen-up" "Page_Up" "Up a screen")
    (moving "paragraph-on" "}" "On a paragraph")
    (moving "paragraph-back" "{" "Back a paragraph")
    (view "view-top" "H" "To the top line in view")
    (view "view-middle" "M" "To the middle line in view")
    (view "view-bottom" "L" "To the bottom line in view")
    (view "place-top" "z t" "This line to the top")
    (view "place-middle" "z z" "This line to the middle")
    (view "place-bottom" "z b" "This line to the bottom")
    (jumping "first" "g g" "To the top, or a count's line")
    (jumping "last" "G" "To the end, or a count's line")
    (jumping "heading-on" "] ]" "On to the next heading")
    (jumping "heading-back" "[ [" "Back to the heading before")
    (jumping "bracket" "%" "To a bracket's other end, or a count's percent")
    (jumping "outline" "g O" "Go to a heading from a list of them")
    (page "command-line" ":" "A command: a line's number, q or noh")
    (jumping "last-jump" "' '" "Back to before the last jump")
    (jumping "last-jump" "` `" "Back to before the last jump")
    (jumping "jump-back" "Ctrl+O" "Back through the jumps")
    (jumping "jump-forward" "Ctrl+I" "Forward through the jumps")
    (jumping "set-mark" "m" "Mark this place with a letter")
    (jumping "go-to-mark" "'" "Go to a letter's mark")
    (jumping "go-to-mark" "`" "Go to a letter's mark")
    (links "links" "g x" "Follow a link on screen, by its label")
    (links "follow-link" "Ctrl+]" "Follow the link here")
    (links "follow-file" "g f" "Open the file a link here names")
    (links "next-file" "] b" "On to the next markdown file here")
    (links "previous-file" "[ b" "Back to the markdown file before")
    (searching "search-word-forward" "*" "Search down for the word here")
    (searching "search-word-backward" "#" "Search up for the word here")
    (searching "search-forward" "/" "Search down the page")
    (searching "search-backward" "?" "Search up the page")
    (searching "next" "n" "On to the next match")
    (searching "previous" "N" "Back to the match before")
    (selecting "select" "v" "Select from the cursor's line")
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
    (selecting "other-end" "o" "To the selection's other end")
    (objects "in-word" "i w" "A word")
    (objects "a-word" "a w" "A word and the space after it")
    (objects "in-sentence" "i s" "A sentence")
    (objects "a-sentence" "a s" "A sentence and the space after it")
    (objects "in-paragraph" "i p" "A paragraph")
    (objects "a-paragraph" "a p" "A paragraph and the blank line after it")
    (objects "in-code" "i `" "The code between backticks")
    (objects "a-code" "a `" "The code, its backticks and a space")
    (copying "copy-selection" "y" "Copy the selection as markdown")
    (copying "copy-selection" "Ctrl+C" "Copy the selection as markdown")
    (copying "copy" "y y" "Copy the block here")
    (copying "copy-paragraph" "y i p" "Copy the paragraph here")
    (copying "" "y a p" "Copy the paragraph and the blank line after it")
    (copying "" "y }" "Copy to the paragraph's end")
    (page "" "Ctrl+Equal" "Larger")
    (page "" "Ctrl+Plus" "Larger")
    (page "" "Ctrl+Minus" "Smaller")
    (page "" "Ctrl+0" "Back to its own size")
    (page "show-keys" "Space ?" "Show every key here")
    (page "show-keys" "F1" "Show every key here")
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
    jitsusama.surfaces.reader = [ "page" ];
  };
}
