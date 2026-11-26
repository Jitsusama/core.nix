# inlyne, for reading a markdown file as a page: a quest's sheet, a plan, a
# README, with its pictures, tables and code drawn in place. It reads and
# never writes, follows the file as it's edited, and opens a link to
# another markdown file in place, so Ctrl+O comes back. Each file it's started
# with is said to the desktop's list of files opened lately
# (../open-with/says-what-it-opens.nix), so its Recent in the launcher
# lists them, led by the file each inlyne shows now, which Return goes to.
#
# Every colour is the theme's: the page on the background, headings in the
# strong text, links and ticked boxes in the accent, code on a block a step
# below the page and coloured as bat colours it (../theme/tmtheme.nix).
# Its words are in the serif upright, as the reader's page sets its text,
# cut as a font of its own (../fonts/serif-text.nix), since inlyne can't
# set a variable font's axes, and its code in the monospaced face.
#
# Its keys are Vim's, as the keyboard language gives them, and only these:
# j and k or ↓ and ↑ a few lines, Ctrl+F and Ctrl+B or Page Down and Page
# Up a screen, gg and G or Home and End the top and the bottom, Ctrl+O and
# Ctrl+I back and forward through the pages followed, Ctrl and + or - larger
# and smaller and Ctrl+0 as it was, Ctrl+C copies what's selected, :q and
# Return, ZZ or Space w close it, as Vim's close a window, and Space ? or F1
# opens a page saying all of them, which Ctrl+O leaves. q alone does
# nothing, since nothing records, and Escape nothing, which closes nothing
# of ours (docs/design.md, Vi's Keys); inlyne has no command line, so :q is
# a sequence its keys wait out. Six patches give it what it lacked: a
# colour for headings, that page, opening without asking the desktop for a
# theme when its settings name one, keeping the file it shows for the
# launcher, since its command line goes stale once a link opens another
# file in place, a space where a paragraph's line wraps in the file, which
# it dropped, joining the words either side, and a zoom it opens at, which
# Ctrl+0 goes back to. A seventh holds q, Escape, j, k, gg, G, the :q
# sequences, ZZ, Space w, Space ?, F1 and the Ctrl chords for a screen,
# the pages followed and the size to Vim's in its own tests, as it's built;
# the arrows, Page keys, Home, End and Ctrl+C aren't pressed there.
#
# It opens at one and a half times the screen's own scale. At its own size
# Joel found its text too small to read (2026-11-01); a logical pixel is
# about as wide on the laptop's screen as on the monitor, so one zoom over
# each screen's scale reads alike on both, where its own `scale`, which
# replaces the screen's, can't.
{
  config,
  lib,
  pkgs,
  ...
}:
let
  inherit (config.jitsusama.theme) colours terminal font;
  serifText = import ../fonts/serif-text.nix { inherit pkgs font; };

  # The theme's #RRGGBB as the number inlyne reads.
  number = colour: lib.fromHexString (lib.removePrefix "#" colour);

  says = pkgs.callPackage ../open-with/says-what-it-opens.nix { };
  inlyne = says { package = patched; };

  # Where each inlyne keeps the file it shows now (keeps-what-it-shows.patch).
  showing = "${config.xdg.stateHome}/inlyne/showing";

  # nixpkgs gives inlyne X's libraries by path but not Wayland's, which its
  # window library looks for by name and, not finding it, opens the window
  # through X instead, so it's given Wayland's too.
  patched = pkgs.inlyne.overrideAttrs (old: {
    patches = (old.patches or [ ]) ++ [
      ./heading-colour.patch
      ./keys-page.patch
      ./named-theme.patch
      ./keeps-what-it-shows.patch
      ./keeps-words-apart.patch
      ./opens-at-a-reading-size.patch
      ./speaks-vim.patch
    ];
    env = (old.env or { }) // {
      INLYNE_KEYBINDINGS = "${bindingsFile}";
    };
    postFixup =
      (old.postFixup or "")
      + lib.optionalString pkgs.stdenv.hostPlatform.isLinux ''
        patchelf $out/bin/inlyne --add-rpath ${lib.makeLibraryPath [ pkgs.wayland ]}
      '';
  });

  # What each key does, in the order the keys page says them, and the
  # group the launcher lists it in over inlyne's window, where it has one,
  # named apart from every other group a word could find there.
  scrolling = map (key: key // { group = "scrolling"; });
  zoom = map (key: key // { group = "zoom"; });
  #
  # Each is a chord, the keys held together with the last pressed, or a
  # sequence of them, with the act of the key language (../keys/language.toml)
  # it is where it's one, which keys-speak-the-language holds it to. An arrow
  # is only its letter's motion, so ← and →, which inlyne has no column to
  # move in, are nothing, and Ctrl+O and Ctrl+I go back and forward through
  # the pages followed, as through Vim's jumps. `says` is how the keys page
  # writes one where it isn't each key's face, as gg for g then g, and
  # `spelled` how the key map does where a held Shift types another key,
  # as : q Return for Shift+; then q then Return. `heard` is the other
  # chords inlyne hears the same key as: its window library names the key
  # a held Shift types, so on Wayland Shift+; arrives as :, and Ctrl with
  # Shift+= as Ctrl+Shift++, though elsewhere as the key pressed.
  keys =
    scrolling [
      {
        chord = [ "j" ];
        says = "j";
        act = "down";
        action = "ScrollDown";
        does = "Down a few lines";
      }
      {
        chord = [ "Down" ];
        act = "down";
        action = "ScrollDown";
        does = "Down a few lines";
      }
      {
        chord = [ "k" ];
        says = "k";
        act = "up";
        action = "ScrollUp";
        does = "Up a few lines";
      }
      {
        chord = [ "Up" ];
        act = "up";
        action = "ScrollUp";
        does = "Up a few lines";
      }
      {
        chord = [
          "Ctrl"
          "f"
        ];
        act = "screen-down";
        action = "PageDown";
        does = "Down a screen";
      }
      {
        chord = [ "PageDown" ];
        act = "screen-down";
        action = "PageDown";
        does = "Down a screen";
      }
      {
        chord = [
          "Ctrl"
          "b"
        ];
        act = "screen-up";
        action = "PageUp";
        does = "Up a screen";
      }
      {
        chord = [ "PageUp" ];
        act = "screen-up";
        action = "PageUp";
        does = "Up a screen";
      }
      {
        sequence = [
          [ "g" ]
          [ "g" ]
        ];
        says = "gg";
        act = "first";
        action = "ToTop";
        does = "The top";
      }
      {
        chord = [ "Home" ];
        action = "ToTop";
        does = "The top";
      }
      {
        chord = [
          "Shift"
          "g"
        ];
        says = "G";
        act = "last";
        action = "ToBottom";
        does = "The bottom";
      }
      {
        chord = [ "End" ];
        action = "ToBottom";
        does = "The bottom";
      }
      {
        chord = [
          "Ctrl"
          "o"
        ];
        act = "jump-back";
        action = "HistoryPrevious";
        does = "Back to the page before";
      }
      {
        chord = [
          "Ctrl"
          "i"
        ];
        act = "jump-forward";
        action = "HistoryNext";
        does = "Forward again";
      }
    ]
    # As the reader's, with Ctrl held, since 0 alone is a line's start.
    ++ zoom [
      {
        chord = [
          "Ctrl"
          "="
        ];
        heard = [
          [
            [
              "Ctrl"
              "Shift"
              "+"
            ]
          ]
        ];
        action = "ZoomIn";
        does = "Larger";
      }
      {
        chord = [
          "Ctrl"
          "-"
        ];
        action = "ZoomOut";
        does = "Smaller";
      }
      {
        chord = [
          "Ctrl"
          "0"
        ];
        action = "ZoomReset";
        does = "As it was";
      }
    ]
    ++ [
      {
        chord = [
          "Ctrl"
          "c"
        ];
        action = "Copy";
        does = "Copy what's selected";
      }
      # ? alone searches up the page, as Vim's, which inlyne can't. Space ?
      # is heard as Shift+/ on the Mac, the only machine that reads with
      # inlyne; on Wayland, where only its checks run, its window library
      # names no key for ?, so there F1, the language's other key, opens it.
      {
        sequence = [
          [ "Space" ]
          [
            "Shift"
            "/"
          ]
        ];
        says = "Space ?";
        spelled = "Space ?";
        act = "show-keys";
        action = "Keys";
        does = "The reader's keys";
      }
      {
        chord = [ "F1" ];
        act = "show-keys";
        action = "Keys";
        does = "The reader's keys";
      }
      # Escape closes nothing of ours (Joel, 2026-11-05: "fuck escape
      # quitting"), so it isn't bound, and q alone does nothing (Joel,
      # 2026-11-04: "q should do nothing by itself. :q to quit"). Each
      # close is a sequence of chords, said on the keys page as typed.
      {
        sequence = [
          [
            "Shift"
            ";"
          ]
          [ "q" ]
          [ "Enter" ]
        ];
        heard = [
          [
            [
              "Shift"
              ":"
            ]
            [ "q" ]
            [ "Enter" ]
          ]
        ];
        says = ":q ↵";
        spelled = ": q Return";
        action = "Quit";
        does = "Close the reader";
      }
      {
        sequence = [
          [
            "Shift"
            "z"
          ]
          [
            "Shift"
            "z"
          ]
        ];
        says = "ZZ";
        act = "close";
        action = "Quit";
        does = "Close the reader";
      }
      {
        sequence = [
          [ "Space" ]
          [ "w" ]
        ];
        says = "Space w";
        act = "close";
        action = "Quit";
        does = "Close the reader";
      }
    ];

  # The chords a key is pressed as, one after another: a chord alone, or a
  # sequence's.
  chordsOf = key: key.sequence or [ key.chord ];

  # A chord as inlyne reads one: its last key, with the others as modifiers.
  comboOf =
    chord:
    if lib.length chord == 1 then
      lib.head chord
    else
      {
        key = lib.last chord;
        mod = lib.init chord;
      };
  # Chords as inlyne reads them: one alone, or a sequence of them.
  combo = chords: if lib.length chords == 1 then comboOf (lib.head chords) else map comboOf chords;

  # How the keys page draws a chord: each key in the theme's face for it
  # (../theme/keys.nix), by the name the theme knows it by, a letter as a
  # capital.
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
  saidChord = lib.concatMapStringsSep " " face;
  said = key: key.says or (lib.concatMapStringsSep " " saidChord (chordsOf key));

  # The keys page: one row for each thing a key does, with every key
  # that does it, in the order the keys are listed.
  doings = lib.unique (map (key: key.does) keys);
  keysFor = does: lib.concatMapStringsSep ", " said (lib.filter (key: key.does == does) keys);
  keysPage = pkgs.writeTextDir "keys.md" ''
    # Keys

    | Key | What it does |
    |:--|:--|
    ${lib.concatMapStringsSep "\n" (does: "| ${keysFor does} | ${does} |") doings}
  '';

  settings = {
    theme = "Dark";
    zoom = 1.5;
    dark-theme = {
      text-color = number colours.text;
      heading-color = number colours.strong;
      background-color = number colours.background;
      code-color = number terminal.magenta;
      quote-block-color = number colours.sunken;
      link-color = number colours.accent;
      select-color = number colours.selection;
      checkbox-color = number colours.accent;
      code-highlighter.path = "${
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
    font-options = {
      regular-font = font.serif.text.family;
      monospace-font = font.mono.family;
    };
    inherit (bindings) keybindings;
    keys-page = "${keysPage}/keys.md";
  };

  file = (pkgs.formats.toml { }).generate "inlyne.toml" settings;

  # The keys alone, which inlyne's own tests read as it's built, so a
  # binding it can't read fails its build, and a key those tests press that
  # inlyne would read otherwise does too (speaks-vim.patch).
  bindings.keybindings.base = lib.concatMap (
    key:
    map (chords: [
      key.action
      (combo chords)
    ]) ([ (chordsOf key) ] ++ key.heard or [ ])
  ) keys;
  bindingsFile = (pkgs.formats.toml { }).generate "inlyne-keys.toml" bindings;
in
{
  imports = [
    ../keys/home.nix
    ../quickshell/recents.nix
    ../theme/home.nix
  ];

  # The serif upright cut with it, where the Mac's fonts and fontconfig
  # find it.
  home.packages = [
    inlyne
    serifText
  ];

  # The file each inlyne shows now, for its Recent in the launcher to lead
  # with, kept as mpv keeps what it plays (../mpv/home.nix says why in the
  # state home) by keeps-what-it-shows.patch; its other files stay the
  # desktop's list of files opened lately.
  systemd.user.tmpfiles.rules = lib.mkIf pkgs.stdenv.hostPlatform.isLinux [
    "d ${showing} 0700 - - -"
  ];
  jitsusama.quickshell.launcher.recents = [
    {
      app = "inlyne";
      kind = "file";
      opened = import ../quickshell/kept-in.nix { inherit lib pkgs; } showing;
    }
  ];

  # The launcher lists these over inlyne's window, by the same words.
  jitsusama.keys.inlyne = map (key: {
    inherit (key) does;
    chord = key.spelled or (lib.concatMapStringsSep " " (lib.concatStringsSep "+") (chordsOf key));
    group = key.group or "";
    act = key.act or "";
  }) keys;

  # Where inlyne looks for its settings: the configuration folder on Linux,
  # Application Support on a Mac.
  xdg.configFile = lib.mkIf pkgs.stdenv.hostPlatform.isLinux { "inlyne/inlyne.toml".source = file; };
  home.file = lib.mkIf pkgs.stdenv.hostPlatform.isDarwin {
    "Library/Application Support/inlyne/inlyne.toml".source = file;
  };
}
