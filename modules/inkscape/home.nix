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
{ niri }:
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
    title
    ;
  profile = "${config.xdg.configHome}/inkscape";

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
      box-shadow: 0 0 0 1px ${roles.accent};
    }

    .mono-font {
      font-family: "${font.mono.family}";
    }
  '';

  # Inkscape's own default.svg, with the desk around the page and the page's
  # outline in the theme. The page stays white, since its colour is the
  # drawing's and not the desktop's, and anything else in the file, even a
  # comment, lands in every new drawing.
  template =
    lib.replaceStrings
      [
        "@border@"
        "@desk@"
      ]
      [
        roles.edge
        roles.sunken
      ]
      (builtins.readFile ./default.svg);

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
        does = "Fill an area";
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
        does = "Zoom in";
      }
      {
        chord = "Minus";
        does = "Zoom out";
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
        does = "Search for a command";
      }
      {
        chord = "Escape";
        does = "Select nothing";
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
    ../theme/home.nix
    niri
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
  home.file."${profile}/templates/default.svg".text = template;
  home.file."${profile}/palettes/${title}.gpl".text = import ../theme/palette-file.nix {
    inherit lib;
  } config.jitsusama.theme;
}
