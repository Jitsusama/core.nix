# GIMP in the theme. GIMP's own theme draws every widget from a handful of
# named colours, which gimp.css, a stylesheet GIMP reads after its theme and
# never writes, redefines from jitsusama.theme's roles. The theme's pigments
# are a palette beside the ones GIMP ships. Both go in GIMP's settings
# folder; GIMP owns everything else there, such as gimprc, sessionrc and
# shortcutsrc, which it rewrites and a link would break.
#
# GIMP comes with G'MIC's filters and Resynthesizer's healing, and starts
# with GIMP3_SYSCONFDIR pointing at its own system folder with gimprc
# replaced by this module's: preferences GIMP reads before the account's
# own, so a choice made in Preferences still wins.
#
# The launcher offers the newest screenshot to edit in GIMP
# (Screenshot.qml), at home for a couple of minutes after it's taken.
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
    ../theme/home.nix
    niri
    quickshell
  ];

  home.packages = [ wrapped ];

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
