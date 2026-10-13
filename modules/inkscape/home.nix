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

  inkscape = pkgs.inkscape-with-extensions.override {
    inkscapeExtensions = [ pkgs.inkscape-extensions.applytransforms ];
  };
in
{
  imports = [
    ../theme/home.nix
    niri
  ];

  home.packages = [ inkscape ];

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
