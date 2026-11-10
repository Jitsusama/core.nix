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
# folder's next markdown file, gO for the headings as a list, ZZ, Space w
# or :q to close, and Space ? or F1 for every key. The command line takes
# Vim's ranges and names: :{n} and :$, :noh, :y with a range, :%y the
# whole file and :'<,'>y the last selection's lines, as the file's
# markdown, :e for another file, :bn and :bp, :h for the keys, and
# :g/pat/, :marks and :ju, which list the lines they name in gO's place,
# Return going to one, saying Vim's error of anything else, E492 for a
# command it hasn't. Each
# search, heading, mark or jump puts the cursor on the line it lands on. v
# and V select, from the cursor's line, with the terminal's cursor, moved
# by the arrows and h j k l, w b e, 0 $ and { }, with iw aw, is as, ip ap
# and i` for text objects and o for the other end; y or Ctrl+C copies the
# selection as the file's markdown through wl-copy, and at rest y y, y i p,
# y a p and y } copy the block the cursor is in, its paragraph, that and
# the blank line after it, or on to the paragraph's end, each flashing what
# it took for 200 ms, as his Neovim's yank does. q does nothing, since
# nothing records. A change, d d or d with its motion, c c, S or c with
# its motion, x, p, or d, c and p on a selection, and :d, says Vim's
# E21: Cannot make changes, as a buffer that can't be modified does. A
# link to another markdown file sets it in the same window, as Neovim's
# gf, keeping the place on the first, and Ctrl+O and Ctrl+I go back and
# forth across files. Escape only cancels: at rest it does nothing, as
# Vim's does, and :noh puts the search's marks away. The find line and
# the command line take the key language's keys, as Vim's command line:
# Escape or Ctrl+C drops one, Backspace or Ctrl+H takes a letter and
# leaves an empty line, Ctrl+W a word, Ctrl+U the rest, and Return makes
# it; held, a key never leaves the line. The moment : is typed, a menu of
# every command the page has rises over the line, as his completion menu
# does, each in Vim's help notation with its name and key: typing narrows
# it, Tab, Shift+Tab, the arrows, Ctrl+N and Ctrl+P choose a line and
# write it in, Ctrl+Y takes it, Ctrl+E puts the menu away, and after :e
# and a space it lists the files. :his lists the lines run, Return running
# one again, and @: runs the last. The same table, as
# the launcher lists it over
# the reader's window, is below, and a check holds the two alike
# (reader-says-the-keys-it-takes).
#
# Every colour and face is the theme's, written here as reader/theme.json
# in the configuration home, which the reader reads as it starts: every
# run of text at one of the theme's type levels, the page's words and
# headings in the serif and code in the mono; the page on the background, headings in the strong text, code on the sunken
# ground in the colours bat wears (../theme/tmtheme.nix), and search's marks
# as Neovim's.
#
# Each file it's started with is said to the desktop's list of files opened
# lately (../open-with/says-what-it-opens.nix), and each reader keeps the
# file it shows under the state home's reader/showing, so its Recent in the
# launcher is led by the files open now. After them come the files it kept
# a place in, each saying the heading it was left under, how far down and
# when, Return reading on there and each mark reading from its letter
# (reader --places and --at). While the clipboard holds a markdown file's
# path, Read the Copied Markdown opens it from the launcher
# (ReaderProgram.qml).
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
    inherit levels space;
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

  # The page's space, in logical pixels: the theme's rungs, which the page
  # and its overlays take across, and down the page a line and a beat of
  # its own. The page is the one surface with a beat of its own, since it's
  # read at length: its lines take a reader's leading, 28 against Words'
  # pitch of 24, as a page's rhythm wants (Bringhurst), and nothing sits
  # beside it whose lines it must meet on the theme's beat. Every space
  # down it, its margins too, is a whole number of its beat, half a line.
  # Its outer column, either side of the words, is the glyph column, a
  # name's cell and a mark: a spread table or code ends on it, a list's
  # numbers hang within it, and the cursor's rule stands an items rung
  # past it.
  space = rec {
    inherit (config.jitsusama.theme.space) rung;
    pitch = 28;
    beat = pitch / 2;
    outer = type.levels.name.pitch + rung.mark;
  };

  # A level of the theme's type as the page sets it, each value worked out:
  # its face's family, its step's size in points, its weight (the serif's
  # from its axis) and its other axes, and whether it's in capitals.
  set = level: {
    family = font.${level.face}.family;
    size = builtins.fromJSON type.${level.face}.${level.step};
    weight = level.axes.wght or level.weight;
    axes = removeAttrs level.axes [ "wght" ];
    capitals = level.case == "capitals";
    inherit (level)
      italic
      tracking
      features
      colour
      strong
      ;
  };
  inherit (type.levels)
    title
    words
    voice
    figure
    label
    note
    ;
  # The words' size, which code and the fleuron are set to a share of.
  wordsSize = (set words).size;
  # The page is a document, so it has tiers of its own, each one of the
  # theme's levels and, where the page bends it, the bend said beside it
  # (docs/design.md, Type). Every run of text the page and its foot set
  # names one, never a size or a weight (type-is-set-by-level).
  levels = {
    # The page's title, its threshold and the one place the serif is set at
    # Display's step: drawn for a poster's size, softened and wonky, at
    # 540, and drawn 0.02 em closer, as a title this large holds together.
    title = set (
      title
      // {
        step = "display";
        axes = {
          wght = 540;
          opsz = 144;
          SOFT = 100;
          WONK = 1;
        };
        tracking = -2.0e-2;
      }
    );
    # A section (##) in the title's cut a step down, at the section step
    # the theme keeps for the page, drawn for that size: a clear step above
    # the words that still lets a page of many short sections read as one
    # page rather than a list of headings, as Title's step would not.
    section = set (
      title
      // {
        step = "section";
        axes = title.axes // {
          opsz = 17;
        };
      }
    );
    # ### at the words' size, in the italic and a touch heavier, in strong:
    # it heads, so the voice's rule bends here, and only here.
    part = set (
      words
      // {
        italic = true;
        axes = words.axes // {
          wght = 460;
        };
        strong = null;
        colour = "strong";
      }
    );
    # The words in the text, as a page's are rather than beside a name. A
    # strong span is 100 heavier than the line, since the theme's 600 draws
    # this face black at a reading size.
    words = set (
      words
      // {
        strong = 480;
        colour = "text";
      }
    );
    # The lede and a quote: the voice, a touch lighter, in the text, and
    # in the words' plainer italic. Voice's wonky cut leans its h and b,
    # which is whimsy in the shell's short lines but in a paragraph reads
    # 'shed' as 'sfied'.
    voice = set (
      voice
      // {
        axes = voice.axes // {
          wght = 360;
          SOFT = 50;
          WONK = 0;
        };
        colour = "text";
      }
    );
    # Code in a line: the figures' mono at 0.93 of the words, so its
    # x-height meets theirs (1035 and 964 of 2000).
    code = set figure // {
      size = 0.93 * wordsSize;
    };
    # A table's figures, in the mono, as everywhere.
    inherit (lib.mapAttrs (_: set) { inherit figure label note; }) figure label note;
    # A code block: the figures' mono at the note's size, on its own line.
    listing = set (figure // { inherit (note) step; });
    # The fleuron's asterisks: the title's cut, lighter, at 1.3 of the
    # words, in the accent.
    ornament =
      set (
        title
        // {
          axes = {
            wght = 500;
            opsz = 144;
            SOFT = 100;
            WONK = 1;
          };
          tracking = 0;
          colour = "accent";
        }
      )
      // {
        size = 1.3 * wordsSize;
      };
  };

  # Every key, as the reader's own list gives it (reader --keys), in the
  # launcher's groups, each with the key language's act it does: a mark's
  # keys sit with the jumps, since two keys make no group of their own, and
  # moving is Scrolling there, since Moving is the picture viewer's. The
  # lines in view, and the links and files, are groups of their own, so
  # none lists more than the launcher's thirteen rows. An arrow moves as its
  # letter but is never listed. Zoom is the reader's own, as Vim has none.
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
  commands = key "commands";
  # The lines' keys work only while one is typed, where some of them mean
  # something else on the page, as Ctrl+E scrolls it.
  on =
    where: act: chord: does:
    commands act chord does
    // {
      when = where;
    };
  onLines = on "on the find line or the command line";
  onCommand = on "on the command line";
  onFind = on "on the find line";
  keys = [
    (moving "down" "j" "Down a line")
    (moving "scroll-down" "Ctrl+E" "The view down a line")
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
    (selecting "left" "h" "A character left")
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
    (copying "copy-a-paragraph" "y a p" "Copy the paragraph and the blank line after it")
    (copying "copy-to-paragraph-end" "y }" "Copy to the paragraph's end")
    (page "" "Ctrl+Equal" "Larger")
    (page "" "Ctrl+Plus" "Larger")
    (page "" "Ctrl+Minus" "Smaller")
    (page "" "Ctrl+0" "Back to its own size")
    (page "ending" "d d" "Nothing: a page can't be changed")
    (page "change-line" "c c" "Nothing: a page can't be changed")
    (page "change-line" "S" "Nothing: a page can't be changed")
    (page "delete-character" "x" "Nothing: a page can't be changed")
    (page "put-after" "p" "Nothing: a page can't be changed")
    (page "delete-motion" "d" "Nothing: a page can't be changed")
    (page "change-motion" "c" "Nothing: a page can't be changed")
    (page "show-keys" "Space ?" "Show every key here")
    (page "show-keys" "F1" "Show every key here")
    (page "close" "Z Z" "Close the page")
    (page "close" "Space w" "Close the page")
    (commands "back" "Escape" "Cancel the line, a selection or a count")
    (onLines "leave" "Ctrl+C" "Cancel the line")
    (onLines "delete-letter" "Backspace" "Delete a letter, or leave an empty line")
    (onLines "delete-letter" "Ctrl+H" "Delete a letter, or leave an empty line")
    (onLines "delete-word" "Ctrl+W" "Delete the word before")
    (onLines "delete-to-start" "Ctrl+U" "Delete everything typed")
    (onCommand "run" "Return" "Run the command typed")
    (onFind "open" "Return" "Search for what's typed")
    (onCommand "next-row" "Tab" "Choose the next command in the menu")
    (onCommand "next-row" "Ctrl+N" "Choose the next command in the menu")
    (onCommand "previous-row" "Shift+Tab" "Choose the command before in the menu")
    (onCommand "previous-row" "Ctrl+P" "Choose the command before in the menu")
    (onCommand "take-name" "Ctrl+Y" "Take the command chosen")
    (onCommand "menu-away" "Ctrl+E" "Put the menu away")
    (commands "again-command" "@ :" "Run the last command again")
  ];
in
{
  imports = [
    ../keys/home.nix
    ../quickshell/providers.nix
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
      # Its places, which the programs daemon reads as its page asks.
      {
        app = "reader";
        name = "places";
        asked = true;
        list = [
          "${reader.unwrapped}/bin/reader"
          "--places"
        ];
      }
    ];

    # The launcher lists these over the reader's window, by the same words,
    # and the reader is a page, as the key language has it.
    jitsusama.keys.reader = keys;
    jitsusama.surfaces.reader = [ "page" ];

    # The reader's own in the launcher: its Recent, each file left where
    # it was read to, and its act reading the markdown file whose path was
    # copied, after every other provider, as it adds no rows of its own
    # but in the reader's step. Its places are listed by the reader
    # itself, unwrapped, since listing them opens no file to say to the
    # desktop's list.
    xdg.configFile."quickshell/launcher/Reader.js".source = ./Reader.js;
    jitsusama.quickshell.launcher.providers.reader = {
      order = 52;
      source = ./ReaderProgram.qml;
      reader = "${reader}/bin/reader";
      # The programs daemon's key for its places, the list above.
      places = "listed:reader:places";
    };
  };
}
