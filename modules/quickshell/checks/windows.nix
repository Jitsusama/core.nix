# The checks of the launcher's Windows, as niri and the windows daemon tell
# them, which the folder's checks.nix gathers with every other place's; its
# opening comment says why each place keeps its own.
{
  account,
  copied,
  graphical,
  noFonts,
  pkgs,
  qs,
  shellFolderLint,
  singleton,
  ...
}:
{
  # The launcher's windows list the one Joel's in as This Window, then the
  # rest the one used last first, and when the focus moves make again only
  # the rows of the windows it moved between, as Capture's do
  # (tests/windows.qml), with what the windows daemon tells given as its
  # lines, and niri, pi's sessions, the recorder, the shortcuts, what was
  # taken and file-preview stood in.
  the-launcher-remakes-only-the-windows-that-changed =
    let
      # file-preview stood in, saying which process answered, and one that
      # stops before answering, each kept by a programs daemon of its own.
      previewer = pkgs.writeShellScript "file-preview" (builtins.readFile ../../../tests/file-preview.sh);
      silent = pkgs.writeShellScript "file-preview" "exec ${previewer} silent";
      programs = pkgs.callPackage ../../programs/package.nix { };
      sources = pkgs.writeText "sources.json" "[]";
      # What a window's verbs read of niri as they're asked.
      niri = singleton "Niri.qml" ''
        property var windows: ({
          "11": { id: 11, app_id: "tester", title: "Logbook", workspace_id: 1, is_focused: true,
            focus_timestamp: { secs: 200, nanos: 0 }, layout: { pos_in_scrolling_layout: [1, 1] } },
          "12": { id: 12, app_id: "tester", title: "Draft", workspace_id: 2, is_focused: false,
            focus_timestamp: { secs: 100, nanos: 0 }, layout: { pos_in_scrolling_layout: [1, 1] } },
          "13": { id: 13, app_id: "tester", title: "Notes", workspace_id: 1, is_focused: false,
            focus_timestamp: { secs: 50, nanos: 0 }, layout: { pos_in_scrolling_layout: [2, 1] } }
        })
        property var workspaces: ({
          "1": { id: 1, idx: 1, is_focused: true, active_window_id: 11 },
          "2": { id: 2, idx: 2, is_focused: false, active_window_id: 12 }
        })
        property var focused: 11
        property int revision: 0
        readonly property var current: { void revision; return focused; }
        function workspace(id) {
          return id === undefined ? Object.values(workspaces).find(one => one.is_focused) ?? null
            : workspaces[id] ?? null;
        }
        function act(action) {}
      '';
      sessions = singleton "Sessions.qml" ''
        property string waitingWindows: ""
        function aboutOf(id) { return ""; }
      '';
      recorder = singleton "Recorder.qml" ''
        property bool recording: false
        property string what: ""
      '';
      shortcuts = singleton "Shortcuts.qml" ''
        property var actions: []
        property var layers: []
      '';
      # Nothing taken, so Capture's Recent holds no file.
      taken = singleton "Taken.qml" ''
        property var files: []
        function want(folder) {}
        function read() {}
      '';
      files = singleton "Files.qml" "function file(path, thing, acts) { return null; }";
    in
    pkgs.runCommandLocal "the-launcher-remakes-only-the-windows-that-changed" { } ''
      mkdir launcher
      cp ${(account graphical).xdg.configFile."quickshell/Theme.qml".source} Theme.qml
      # Capture's Recent reads its rows ready, through ReadyRows.
      ${copied [
        "Daemon.qml"
        "Words.qml"
        "launcher/Capture.qml"
        "launcher/Previewer.qml"
        "launcher/Provider.qml"
        "launcher/ReadyRows.qml"
        "launcher/Windows.qml"
        "launcher/words.json"
      ]}
      cp ${../../../tests/lib/Expect.js} Expect.js
      cp ${../../../tests/windows.qml} shell.qml
      cp ${niri} Niri.qml
      cp ${sessions} Sessions.qml
      cp ${recorder} Recorder.qml
      cp ${shortcuts} Shortcuts.qml
      cp ${taken} Taken.qml
      cp ${files} Files.qml
      export HOME=$TMPDIR XDG_RUNTIME_DIR=$TMPDIR/runtime LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen
      mkdir -m 700 $XDG_RUNTIME_DIR
      export FONTCONFIG_FILE=${noFonts}
      mkdir $XDG_RUNTIME_DIR/daemons
      PROGRAMS_PREVIEW=${previewer} \
        ${programs}/bin/programs $XDG_RUNTIME_DIR/daemons/programs.sock ${sources} &
      daemon=$!
      PROGRAMS_PREVIEW=${silent} \
        ${programs}/bin/programs $XDG_RUNTIME_DIR/daemons/silent.sock ${sources} &
      mute=$!
      ${shellFolderLint}
      timeout 60 ${qs} -p shell.qml
      kill $daemon $mute
      touch $out
    '';

  # The shell's niri, told niri's events as its stream would, moves the keys
  # and says a window used without making any list again, and still lists
  # a window retitled, opened or closed (tests/niri-focus.qml).
  niri-moving-the-keys-makes-no-list-again =
    # Kept in Nix because each line copies a file Nix gives it beside the
    # cases, or sets what Quickshell runs them with.
    pkgs.runCommandLocal "niri-moving-the-keys-makes-no-list-again" { } ''
      ${copied [
        "Daemon.qml"
        "Niri.qml"
      ]}
      cp ${../../../tests/lib/Expect.js} Expect.js
      cp ${../../../tests/niri-focus.qml} shell.qml
      export HOME=$TMPDIR XDG_RUNTIME_DIR=$TMPDIR/runtime LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen
      mkdir -m 700 $XDG_RUNTIME_DIR
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      timeout 60 ${qs} -p shell.qml
      touch $out
    '';

  # The launcher keeps the windows the windows daemon tells of in the order
  # it lists them, the one used last first, placing each window a change
  # names and letting those gone go, as tests/held-windows.qml holds
  # launcher/HeldWindows.js to records made up there.
  the-launcher-places-each-window-it-hears =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "the-launcher-places-each-window-it-hears" { } ''
      mkdir launcher
      cp ${../../../tests/held-windows.qml} held-windows.qml
      ${copied [ "launcher/HeldWindows.js" ]}
      cp ${../../../tests/lib/Expect.js} Expect.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml held-windows.qml
      touch $out
    '';

  # The shell's niri holds what the windows daemon tells in niri's own
  # shape, each window told made again in place, and says to its readers
  # what niri's events said, a window used or a tile moving making no list
  # again (tests/niri-records.qml over NiriRecords.js).
  niri-is-held-as-the-windows-daemon-tells-it =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "niri-is-held-as-the-windows-daemon-tells-it" { } ''
      cp ${../../../tests/niri-records.qml} niri-records.qml
      ${copied [ "NiriRecords.js" ]}
      cp ${../../../tests/lib/Expect.js} Expect.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml niri-records.qml
      touch $out
    '';
}
