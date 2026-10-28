# GIMP in the theme. GIMP's own theme draws every widget from a handful of
# named colours, which gimp.css, a stylesheet GIMP reads after its theme and
# never writes, redefines from jitsusama.theme's roles. The theme's pigments
# are a palette beside the ones GIMP ships. Both go in GIMP's settings
# folder, with shortcutsrc, the keys that differ from GIMP's, which the
# system gimprc keeps GIMP from saving over. GIMP owns everything else
# there, such as gimprc and sessionrc, which it rewrites and a link would
# break.
#
# GIMP comes with G'MIC's filters and Resynthesizer's healing, and starts
# with GIMP3_SYSCONFDIR pointing at its own system folder with gimprc
# replaced by this module's: preferences GIMP reads before the account's
# own, so a choice made in Preferences still wins.
#
# The launcher offers the newest screenshot to edit in GIMP
# (Screenshot.qml), at home for a couple of minutes after it's taken.
#
# The launcher lists GIMP's own keys over its window, as the Picture editor.
#
# GIMP's main window opens across the screen's whole width, since at niri's
# half a column its docks cover the picture; its dialogs still float.
{ niri, quickshell }:
{
  config,
  lib,
  pkgs,
  ...
}:
let
  inherit (config.jitsusama.theme) roles font appearance;

  # GIMP keeps its settings in a folder for each release series.
  series = "3.2";
  folder = "${config.xdg.configHome}/GIMP/${series}";

  # Where niri's screenshot-path and the shell's region pictures
  # (Recorder.qml) both save.
  screenshots = "${config.home.homeDirectory}/Pictures/Screenshots";

  # The colours GIMP's theme names, in themes/Default/gimp-dark.css and
  # gimp-light.css, and what each means here.
  colours = {
    fg-color = roles.text;
    bg-color = roles.background;
    border-color = roles.edge;
    dimmed-fg-color = roles.muted;
    disabled-fg-color = roles.rule;
    disabled-button-color = roles.raised;
    hover-color = roles.raised;
    widget-bg-color = roles.sunken;
    selected-color = roles.selection;
    extreme-bg-color = roles.deepest;
    extreme-selected-color = roles.raised;
    strong-border-color = roles.edge;
    stronger-border-color = roles.rule;
    edge-border-color = roles.edge;
    scrollbar-slider-color = roles.muted;
    scrollbar-trough-color = roles.sunken;
  };

  stylesheet = ''
    /* GIMP's colours, from jitsusama.theme's roles. */
    ${lib.concatLines (lib.mapAttrsToList (name: colour: "@define-color ${name} ${colour};") colours)}
    /* The rulers' marks, drawn faint over the canvas's edge. */
    @define-color ruler-color alpha(${roles.rule}, 0.3);
  '';

  grouped = group: map (key: key // { inherit group; });

  # From GIMP 3.2.6's own defaults: its tools' keys in app/tools and app/paint,
  # the rest in app/actions, with shortcutsrc's ? beside its own /.
  keys =
    grouped "tools" [
      {
        chord = "r";
        does = "Select a rectangle";
      }
      {
        chord = "e";
        does = "Select an ellipse";
      }
      {
        chord = "f";
        does = "Select freehand";
      }
      {
        chord = "u";
        does = "Select a colour's area";
      }
      {
        chord = "m";
        does = "Move";
      }
      {
        chord = "Shift+c";
        does = "Crop";
      }
      {
        chord = "Shift+t";
        does = "Scale, rotate and shear";
      }
      {
        chord = "t";
        does = "Write on the picture";
      }
      {
        chord = "p";
        does = "Paint with a brush";
      }
      {
        chord = "Shift+e";
        does = "Erase";
      }
      {
        chord = "Shift+b";
        does = "Fill an area of colour";
      }
      {
        chord = "o";
        does = "Pick a colour from the picture";
      }
      {
        chord = "h";
        does = "Heal";
      }
    ]
    ++ grouped "view" [
      {
        chord = "1";
        does = "The picture at its own size";
      }
      {
        chord = "Ctrl+Shift+j";
        does = "The picture to fit the window";
      }
      {
        chord = "Plus";
        does = "The picture larger";
      }
      {
        chord = "Minus";
        does = "The picture smaller";
      }
      {
        chord = "Tab";
        does = "Show or hide the docks";
      }
      {
        chord = "F11";
        does = "The window on the whole screen";
      }
    ]
    ++ grouped "selection" [
      {
        chord = "Ctrl+a";
        does = "Select everything";
      }
      {
        chord = "Ctrl+Shift+a";
        does = "Select nothing";
      }
      {
        chord = "Ctrl+i";
        does = "Select what isn't selected";
      }
    ]
    ++ grouped "layers" [
      {
        chord = "Ctrl+Shift+n";
        does = "A new layer";
      }
      {
        chord = "Ctrl+Shift+d";
        does = "Duplicate the layer";
      }
      {
        chord = "Ctrl+h";
        does = "Anchor the floating selection";
      }
    ]
    ++ [
      {
        chord = "Ctrl+Shift+v";
        does = "Paste as a new picture";
      }
      {
        chord = "?";
        does = "Search GIMP's commands";
      }
      {
        chord = "Ctrl+z";
        does = "Undo the last change";
      }
      {
        chord = "Ctrl+y";
        does = "Redo the change undone";
      }
      {
        chord = "Ctrl+s";
        does = "Save the picture";
      }
      {
        chord = "Ctrl+Shift+e";
        does = "Export the picture";
      }
    ];

  gimp = pkgs.gimp3-with-plugins.override {
    plugins = [
      pkgs.gimpPlugins.gmic
      pkgs.gimpPlugins.resynthesizer
    ];
  };

  gimprc = pkgs.writeText "gimprc" ''
    ${builtins.readFile ./gimprc}
    # From the theme: its colour scheme, and its sans for new text.
    (theme-color-scheme ${appearance})
    (default-font "${font.sans.family}")
  '';

  # GIMP's system folder, with gimprc replaced and the rest as it ships, since
  # GIMP3_SYSCONFDIR stands in for the whole folder.
  sysconf = pkgs.runCommandLocal "gimp-sysconf" { } ''
    mkdir $out
    for file in ${gimp}/etc/gimp/3.0/*; do
      ln -s "$file" "$out/$(basename "$file")"
    done
    ln -sf ${gimprc} $out/gimprc
  '';

  # The desktop entry starts gimp-3.2, and gimp and gimp-3 link to it.
  wrapped = pkgs.symlinkJoin {
    name = "gimp-${gimp.version}";
    inherit (gimp) version meta;
    outputs = [
      "out"
      "man"
    ];
    paths = [ gimp ];
    nativeBuildInputs = [ pkgs.makeWrapper ];
    postBuild = ''
      for program in gimp-${series} gimp-console-${series}; do
        wrapProgram $out/bin/$program --set GIMP3_SYSCONFDIR ${sysconf}
      done
      ln -s ${gimp.man} $man
    '';
    passthru = { inherit sysconf; };
  };
in
{
  imports = [
    ../keys/home.nix
    ../theme/home.nix
    niri
    quickshell
  ];

  home.packages = [ wrapped ];

  # GIMP's keys, for the launcher to list over its window.
  jitsusama.keys.gimp = keys;

  # The main window is the only one titled this as it opens; by the time a
  # picture is in it, the title names the picture. Maximized in niri keeps
  # the gaps around it, as every window has.
  xdg.configFile."niri/config.kdl".text = lib.mkAfter ''
    window-rule {
        match app-id="^gimp$" title="^GNU Image Manipulation Program$"
        open-maximized true
    }
  '';

  # After Tuple's, beside the desktop's actions, since it's one.
  jitsusama.quickshell.launcher.providers.gimp = {
    order = 46;
    source = ./Screenshot.qml;
    folder = screenshots;
  };

  # niri makes the folder only with the first screenshot, and a folder that
  # isn't there can't be watched, so it's made at login, before the shell.
  systemd.user.tmpfiles.rules = [ "d ${screenshots} - - - -" ];

  home.file."${folder}/gimp.css".text = stylesheet;
  # GIMP made one as it first closed, holding only its own keys written out
  # as comments, so the declared one takes its place.
  home.file."${folder}/shortcutsrc" = {
    force = true;
    source = ./shortcutsrc;
  };
  home.file."${folder}/palettes/${config.jitsusama.theme.title}.gpl".text =
    import ../theme/palette-file.nix { inherit lib; }
      config.jitsusama.theme;

  assertions = [
    {
      assertion = lib.versions.majorMinor gimp.version == series;
      message = ''
        GIMP is ${gimp.version}, which keeps its settings somewhere other than
        GIMP/${series}; modules/gimp/home.nix's series needs to follow it.
      '';
    }
  ];
}
