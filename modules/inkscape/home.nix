# Inkscape in the theme. It's a GTK 3 program, so the gtk module's colours
# already reach most of it; ui/user.css, which Inkscape reads after
# its own stylesheet and never writes, recolours what Inkscape draws in its
# own colours: the eight it offers layers and objects to be highlighted in,
# and the ring around a slider being dragged. It also sets the theme's
# monospace where Inkscape shows code, the XML editor and the attributes, at
# the size Preferences chooses. A new drawing starts from default.svg, on a
# desk in the theme, and the theme's pigments are a palette among the rest.
# Inkscape owns everything else in its profile, preferences.xml above all,
# which it rewrites whenever it quits.
#
# Inkscape comes with Apply Transforms, which bakes an object's transforms
# into its paths.
#
# The launcher lists Inkscape's own keys over its window, as the Drawing
# editor. They're the keys Inkscape ships with, which already speak the
# desktop's language where Inkscape has the thing it means: ? opens its
# command palette, and Page Up and Down move the selection a level of the
# stack. Its digits stay its zoom presets, since it has nothing numbered to
# go to.
#
# Its drawing window opens across the screen's whole width, with the gaps
# every window has, rather than over the whole screen as it asks to; its
# splash and dialogs still float.
#
# Programs › Inkscape in the launcher holds New Drawing, a fresh copy of
# the template in the drawings folder, opened in Inkscape, and, while the
# clipboard holds an SVG or a picture, New Drawing From the Copied
# Picture; and its Recent is the drawings Inkscape opened lately, each
# saying its size, from the desktop's list of files opened lately, which
# Inkscape adds to itself (InkscapeProgram.qml).
{ niri, quickshell }:
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
    title
    ;
  profile = "${config.xdg.configHome}/inkscape";

  # Where a new drawing from the launcher is written, beside the camera's
  # photos and the screenshots, so saving it in Inkscape keeps it.
  drawings = "${config.home.homeDirectory}/Pictures/Drawings";

  # The desk around the page and the page's outline, which a new drawing
  # has from the template or, made from a copied picture, from the
  # launcher.
  border = colours.edge;
  desk = colours.sunken;

  # Inkscape names its highlights gray, blue, yellow, red, green, orange,
  # cyan and purple, and the terminal's colours are the theme's hues.
  highlights = [
    terminal.bright_black
    terminal.blue
    terminal.bright_yellow
    terminal.red
    terminal.green
    terminal.bright_red
    terminal.cyan
    terminal.magenta
  ];

  highlight = index: colour: ''
    .bright .highlight-color-${toString index},
    .dark .highlight-color-${toString index} {
      color: ${colour};
    }
  '';

  stylesheet = ''
    /* Inkscape's own colours, from jitsusama.theme. */
    ${lib.concatStrings (lib.imap1 highlight highlights)}
    #InkSpinScale box.active {
      box-shadow: 0 0 0 1px ${colours.accent};
    }

    .mono-font {
      font-family: "${font.mono.family}";
    }
  '';

  # Inkscape's own default.svg, with the desk around the page and the page's
  # outline in the theme. The page stays white, since its colour is the
  # drawing's and not the desktop's, and anything else in the file, even a
  # comment, lands in every new drawing.
  template = pkgs.writeText "default.svg" (
    lib.replaceStrings
      [
        "@border@"
        "@desk@"
      ]
      [
        border
        desk
      ]
      (builtins.readFile ./default.svg)
  );

  grouped = group: map (key: key // { inherit group; });

  # From Inkscape 1.4.4's share/keys/inkscape.xml, the keys it ships with.
  keys =
    grouped "tools" [
      {
        chord = "s";
        does = "Select and move";
      }
      {
        chord = "n";
        does = "Edit the nodes";
      }
      {
        chord = "r";
        does = "Draw a rectangle";
      }
      {
        chord = "e";
        does = "Draw an ellipse";
      }
      {
        chord = "p";
        does = "Draw freehand";
      }
      {
        chord = "b";
        does = "Draw with the pen";
      }
      {
        chord = "t";
        does = "Write text";
      }
      {
        chord = "g";
        does = "Edit a gradient";
      }
      {
        chord = "d";
        does = "Pick a colour from the drawing";
      }
      {
        chord = "u";
        does = "Fill a closed shape";
      }
      {
        chord = "z";
        does = "Zoom with the pointer";
      }
      {
        chord = "m";
        does = "Measure";
      }
    ]
    ++ grouped "view" [
      {
        chord = "1";
        does = "The drawing at its own size";
      }
      {
        chord = "2";
        does = "The drawing at half its size";
      }
      {
        chord = "3";
        does = "Zoom to the selection";
      }
      {
        chord = "4";
        does = "Zoom to the whole drawing";
      }
      {
        chord = "5";
        does = "Zoom to the page";
      }
      {
        chord = "Plus";
        does = "The drawing larger";
      }
      {
        chord = "Minus";
        does = "The drawing smaller";
      }
      {
        chord = "`";
        does = "The zoom before";
      }
      {
        chord = "%";
        does = "Snapping on or off";
      }
    ]
    ++ grouped "objects" [
      {
        chord = "Ctrl+g";
        does = "Group the selection";
      }
      {
        chord = "Ctrl+Shift+g";
        does = "Ungroup the selection";
      }
      {
        chord = "Ctrl+d";
        does = "Duplicate the selection";
      }
      {
        chord = "Page_Up";
        does = "Raise the selection a step";
      }
      {
        chord = "Page_Down";
        does = "Lower the selection a step";
      }
      {
        chord = "Home";
        does = "Raise the selection to the top";
      }
      {
        chord = "End";
        does = "Lower the selection to the bottom";
      }
      {
        chord = "h";
        does = "Flip the selection across";
      }
      {
        chord = "v";
        does = "Flip the selection over";
      }
    ]
    ++ grouped "panels" [
      {
        chord = "Ctrl+Shift+f";
        does = "Fill and stroke";
      }
      {
        chord = "Ctrl+Shift+l";
        does = "Layers and objects";
      }
      {
        chord = "Ctrl+Shift+a";
        does = "Align and distribute";
      }
      {
        chord = "Ctrl+Shift+t";
        does = "Text and font";
      }
      {
        chord = "Ctrl+Shift+e";
        does = "Export";
      }
      {
        chord = "Ctrl+Shift+d";
        does = "Document properties";
      }
      {
        chord = "Ctrl+Shift+x";
        does = "The XML editor";
      }
      {
        chord = "F12";
        does = "Show or hide the panels";
      }
    ]
    ++ [
      {
        chord = "?";
        does = "Search Inkscape's commands";
      }
      {
        chord = "Escape";
        does = "Let go of the selection";
      }
      {
        chord = "Ctrl+f";
        does = "Find objects";
      }
      {
        chord = "Ctrl+z";
        does = "Undo";
      }
      {
        chord = "Ctrl+Shift+z";
        does = "Redo";
      }
    ];

  inkscape = pkgs.inkscape-with-extensions.override {
    inkscapeExtensions = [ pkgs.inkscape-extensions.applytransforms ];
  };
in
{
  imports = [
    ../keys/home.nix
    ../quickshell/recents.nix
    ../theme/home.nix
    niri
    quickshell
  ];

  home.packages = [ inkscape ];

  # Inkscape's keys, for the launcher to list over its window.
  jitsusama.keys.inkscape = keys;

  # A drawing's window is titled after it, "<file> - Inkscape", from the
  # moment it opens; the splash is titled with the version instead.
  xdg.configFile."niri/config.kdl".text = lib.mkAfter ''
    window-rule {
        match app-id="^org\\.inkscape\\.Inkscape$" title=" - Inkscape$"
        open-maximized true
        open-maximized-to-edges false
    }
  '';

  home.file."${profile}/ui/user.css".text = stylesheet;
  home.file."${profile}/templates/default.svg".source = template;
  home.file."${profile}/palettes/${title}.gpl".text = import ../theme/palette-file.nix {
    inherit lib;
  } config.jitsusama.theme;

  # What Inkscape's provider works out, beside it, where its import finds
  # it.
  xdg.configFile."quickshell/launcher/Inkscape.js".source = ./Inkscape.js;

  # New Drawing and its drawings lately, in Programs › Inkscape, after
  # every other provider, as it adds no rows of its own.
  jitsusama.quickshell.launcher.providers.inkscape = {
    order = 52;
    source = ./InkscapeProgram.qml;
    inkscape = lib.getExe' inkscape "inkscape";
    template = "${template}";
    inherit drawings border desk;
    # The programs daemon's key for its drawings lately, listed below.
    lately = "listed:org.inkscape.Inkscape:drawings";
    paste = lib.getExe' pkgs.wl-clipboard "wl-paste";
  };

  # Its drawings lately, which the programs daemon reads as its page asks,
  # at most as many as Recent lists (Recent.js).
  jitsusama.quickshell.launcher.recents = [
    {
      app = "org.inkscape.Inkscape";
      name = "drawings";
      asked = true;
      list = [
        (lib.getExe (pkgs.callPackage ./drawings.nix { }))
        "9"
      ];
    }
  ];
}
