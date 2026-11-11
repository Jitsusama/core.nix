# The checks of inkscape, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  account,
  graphical,
  lib,
  onLinux,
  pkgs,
  shellFolderLint,
  ...
}:
let
  # A stand-in for one of the shell's singletons, holding only what a check
  # needs of it.
  singleton =
    name: body:
    pkgs.writeText name ''
      pragma Singleton
      import Quickshell
      import QtQuick
      Singleton { ${body} }
    '';
in
onLinux {
  # Inkscape's step in the launcher, its provider's as Applications.qml
  # holds it (tests/inkscape-program.qml), over Inkscape's desktop entry,
  # with niri and what's playing stood in, the shell's own watch of the
  # clipboard with no history to hear, and the desktop's list of
  # files opened lately as GLib writes it (tests/inkscape-recent.xbel),
  # recent-files adding one more as a program does, read by recent-files
  # and by the account's own inkscape-drawings over drawings made here,
  # both run by the programs daemon given the sources the account gives:
  # its Recent lists each drawing Inkscape opened by when, newest first,
  # merged with the shared list's, each once, Inkscape's own row kept, one
  # that's gone left out; each says when, and its drawer its size as its
  # SVG says it, in its own units, its weight and its path; Return opens it
  # in Inkscape. New Drawing writes a fresh copy of the template and gives
  # Inkscape, stood in, that, leaving the template be, and New Drawing From
  # the Copied Picture is there only while an SVG is copied, or a picture
  # the history keeps the size of, and makes a drawing of each; and its
  # drawer in Programs says the last and how many. Inkscape.js's own cases
  # run first in Qt's engine.
  the-launcher-lists-inkscapes-drawings-lately =
    let
      home = account graphical;
      settings = home.jitsusama.quickshell.launcher.providers.inkscape;
      # Its drawings lately, as the account's programs daemon is given them.
      drawn = lib.findFirst (
        source: source.name or null == "drawings"
      ) null home.jitsusama.quickshell.launcher.recents;
      qs = lib.getExe' graphical.pkgs.quickshell "qs";
      # What each program had open lately is the programs daemon's.
      programs = pkgs.callPackage ../programs/package.nix { };
      recent-files = lib.getExe (pkgs.callPackage ../open-with/recent-files.nix { });
      # The launcher's own files the step is made from, each by itself so
      # no other change there builds this again.
      launcher = ../quickshell/launcher;
      launched = [
        "Provider.qml"
        "Asking.js"
        "Numbers.js"
        "Applications.qml"
        "HeldWindows.js"
        "Order.js"
        "Program.js"
        "Media.js"
        "Recents.qml"
        "Recents.js"
        "Listed.qml"
        "Recent.js"
        "Captured.js"
        "Ago.js"
        "words.json"
      ];
      # Kept in Nix because it's a stand-in's few properties, as each
      # program's check in the launcher writes its niri.
      niri = singleton "Niri.qml" ''
        property int revision: 0
        property var windows: ({})
        property var workspaces: ({
          "1": { id: 1, idx: 1, is_focused: true, active_window_id: null }
        })
        readonly property int current: -1
        property var acted: []
        property var spawned: []
        function workspace(id) { return workspaces[id ?? "1"] ?? null; }
        function act(action) { acted = acted.concat([action]); }
        function spawn(command) { spawned = command; }
      '';
      playing = singleton "Playing.qml" ''
        property var players: []
        property var streams: []
        property var recorders: []
      '';
      entry = pkgs.writeText "org.inkscape.Inkscape.desktop" ''
        [Desktop Entry]
        Type=Application
        Name=Inkscape
        GenericName=Vector Graphics Editor
        Exec=inkscape %F
        Actions=new-window;

        [Desktop Action new-window]
        Name=Open a New Window
        Exec=inkscape
      '';
      # What wl-paste gives of each copy the case sets.
      paste = pkgs.writeShellScript "paste" ''
        case $2 in
          image/svg+xml) echo '<svg xmlns="http://www.w3.org/2000/svg" width="3" height="1"/>' ;;
          image/png) printf PNG ;;
          *) exit 1 ;;
        esac
      '';
      # Inkscape, saying what it was given.
      inkscape = pkgs.writeShellScript "inkscape" ''printf '%s\n' "$@" > "$HOME/given"'';
    in
    # Kept in Nix because each line copies, or names in a variable, a file
    # Nix gives it, and the drawings it reads are made by a script of their
    # own (tests/drawings-lately.sh).
    pkgs.runCommandLocal "the-launcher-lists-inkscapes-drawings-lately" { } ''
      mkdir -p launcher share/applications
      cp ${entry} share/applications/org.inkscape.Inkscape.desktop
      cp ${home.xdg.configFile."quickshell/Theme.qml".source} Theme.qml
      cp ${../quickshell/Fuzzy.js} Fuzzy.js
      cp ${../quickshell/ProgramGlyphs.js} ProgramGlyphs.js
      cp ${../quickshell/Music.js} Music.js
      cp ${../quickshell/Words.qml} Words.qml
      cp ${../quickshell/Files.qml} Files.qml
      cp ${../quickshell/Daemon.qml} Daemon.qml
      cp ${../quickshell/FileFacts.js} FileFacts.js
      cp ${../quickshell/When.js} When.js
      ${lib.concatMapStrings (file: ''
        cp ${launcher + "/${file}"} launcher/${file}
      '') launched}
      cp ${./InkscapeProgram.qml} launcher/InkscapeProgram.qml
      cp ${./Inkscape.js} launcher/Inkscape.js
      cp ${../../tests/inkscape-program.qml} shell.qml
      cp ${../../tests/lib/Expect.js} Expect.js
      cp ${../../tests/lib/Frames.js} Frames.js
      cp ${../../tests/lib/HeardWindows.js} HeardWindows.js
      cp ${niri} Niri.qml
      cp ${playing} launcher/Playing.qml
      cp ${singleton "Shortcuts.qml" "property var actions: []"} Shortcuts.qml
      cp ${../quickshell/Copied.qml} Copied.qml
      cp ${../quickshell/Copied.js} Copied.js
      export HOME=$TMPDIR XDG_RUNTIME_DIR=$TMPDIR/runtime LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen
      export XDG_DATA_HOME=$TMPDIR/data XDG_DATA_DIRS=$PWD/share NOW=$(date +%s)
      mkdir -m 700 $XDG_RUNTIME_DIR
      export FONTCONFIG_FILE=${pkgs.makeFontsConf { fontDirectories = [ ]; }}
      # The template as the account has it, in a copy that could be
      # written over, so the case sees it wasn't.
      export TEMPLATE=$HOME/default.svg TEMPLATE_KEPT=${settings.template}
      install -m 644 ${settings.template} $TEMPLATE
      export INKSCAPE=${inkscape} PASTE=${paste} RECENT_FILES=${recent-files}
      ${pkgs.bash}/bin/bash ${./tests/drawings-lately.sh} ${./tests/inkscape-recent.xbel} \
        ${recent-files}
      # The programs daemon runs the list programs share, given the source
      # shell.qml's Applications is, as recents.nix gives it, and Inkscape's
      # drawings lately as the account gives them.
      cat > sources.json <<EOF
      [{"watch": "$XDG_DATA_HOME/recently-used.xbel", "list": ["$RECENT_FILES", "list"],
        "dated": true}, ${builtins.toJSON drawn}]
      EOF
      ${lib.getExe programs} "$XDG_RUNTIME_DIR/daemons/programs.sock" sources.json &
      ${shellFolderLint}
      timeout 60 ${qs} -p shell.qml
      touch $out
    '';

  # Inkscape starts a new drawing from the account's profile, on the theme's
  # desk: a template it can't find leaves its own grey one without a word.
  # Inkscape draws nothing without a screen, so GTK reads the stylesheet
  # instead, refusing what it can't parse, and every name it recolours has
  # to be one Inkscape's own stylesheets still use.
  inkscape-accepts-its-configuration =
    let
      home = account graphical;
      inherit (home.jitsusama.theme) colours;
      inkscape = lib.findFirst (
        package: lib.getName package == "inkscape-with-extensions"
      ) null home.home.packages;
      profile = "${home.xdg.configHome}/inkscape";
      files = home.home.file;
      python = pkgs.python3.withPackages (packages: [ packages.pygobject3 ]);
      typelibs = lib.makeSearchPath "lib/girepository-1.0" [
        pkgs.gtk3
        pkgs.gdk-pixbuf
        pkgs.pango.out
        pkgs.harfbuzz
        pkgs.at-spi2-core
        pkgs.gobject-introspection
      ];
      parses = pkgs.writeText "parses.py" ''
        import gi, sys
        gi.require_version("Gtk", "3.0")
        from gi.repository import Gtk
        Gtk.CssProvider().load_from_path(sys.argv[1])
      '';
    in
    pkgs.runCommandLocal "inkscape-accepts-its-configuration"
      {
        nativeBuildInputs = [
          inkscape
          python
        ];
      }
      ''
        export HOME=$PWD INKSCAPE_PROFILE_DIR=$PWD/profile
        mkdir -p profile/ui profile/templates
        cp ${files."${profile}/ui/user.css".source} profile/ui/user.css
        cp ${files."${profile}/templates/default.svg".source} profile/templates/default.svg
        inkscape --actions='file-new;export-filename:new.svg;export-do' > said 2>&1 || true
        if ! grep -q 'inkscape:deskcolor="${colours.sunken}"' new.svg; then cat said; exit 1; fi
        GI_TYPELIB_PATH=${typelibs} python3 ${parses} profile/ui/user.css
        sed -n -E 's/^ *([^ /*].*[^ ]) *[{,]$/\1/p' profile/ui/user.css > selectors
        while read -r selector; do
          if ! grep -q -F -- "$selector" ${inkscape}/share/inkscape/ui/*.css; then
            echo "Inkscape's stylesheets no longer use $selector"
            exit 1
          fi
        done < selectors
        touch $out
      '';
}
