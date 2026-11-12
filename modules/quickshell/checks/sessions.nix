# The checks of the sessions kitty, and reopening sessions where they were,
# which the folder's checks.nix gathers with every other place's; its opening
# comment says why each place keeps its own.
{
  account,
  copied,
  graphical,
  holds,
  lib,
  noFonts,
  pkgs,
  qs,
  shellFolderLint,
  singleton,
  ...
}:
let
  # Joel away, so what waits for him to go opens; a case sets it as it needs.
  away = singleton "Away.qml" "property bool now: true";
in
{
  # A switch never restarts the kitty every session's window is in, so no
  # switch closes them.
  the-sessions-kitty-outlives-a-switch =
    holds "the-sessions-kitty-outlives-a-switch"
      "A switch would restart the sessions' kitty, closing every session's window."
      (
        ((account graphical).systemd.user.services.sessions-kitty.Unit.X-SwitchMethod or null) == "keep-old"
      );

  # The sessions' kitty draws only the frames niri asks for, so the windows
  # niri doesn't show stop being drawn.
  the-sessions-kitty-keeps-niris-pace =
    let
      service = (account graphical).systemd.user.services.sessions-kitty.Service;
    in
    holds "the-sessions-kitty-keeps-niris-pace"
      "The sessions' kitty draws as it pleases, so every hidden session's window is drawn too."
      (lib.any (lib.hasInfix "-o sync_to_monitor=yes") (lib.toList (service.ExecStart or [ ])));

  # The sessions' kitty keeps what it swaps out compressed in memory, so a
  # session's window Joel comes back to never draws from disk.
  the-sessions-kitty-stays-off-the-disk =
    holds "the-sessions-kitty-stays-off-the-disk"
      "The sessions' kitty writes its swapped memory out to disk, so a window he comes back to reads it back."
      (
        ((account graphical).systemd.user.services.sessions-kitty.Service.MemoryZSwapWriteback or true)
        == false
      );

  # Reopen Them All puts each session back on its own screen, found by what
  # the screen is rather than its connector, a gone screen's workspaces
  # after the one in use's and to it once it's back, and each window that
  # reopens itself beside those kept with it, and begins the next session
  # only while the machine queue says there's room, as tests/places.qml
  # holds Places.js and Viewers.js to it in Qt's own engine.
  reopening-finds-each-screen =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "reopening-finds-each-screen" { } ''
      cp ${../../../tests/places.qml} places.qml
      ${copied [
        "Places.js"
        "Viewers.js"
      ]}
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml places.qml
      touch $out
    '';

  # pi's sessions are known by their own records as the machine's node
  # holds them, never by a window's title, and each one's window by the app
  # id it opened with, or for one opened without, through the kitty it runs
  # in (tests/sessions.qml), with the node, the agents daemon, niri and
  # kitten stood in over what modules/quickshell/tests/stand-ins/
  # sessions-state.sh lays out.
  a-session-is-known-by-its-own-record =
    let
      # Three windows of the same title, one of kitty 500's and two of 600's,
      # the second opened as pi-session.7 and laid out in workspace 3's
      # second column, each with the app id it opened with kept, as Niri.qml
      # keeps it; the windows daemon tells of the same three
      # (tests/sessions.qml).
      niri = singleton "Niri.qml" ''
        property var windows: ({
          "10": { id: 10, pid: 500, title: "\u03c0 - Lighthouse", opened: "kitty" },
          "20": { id: 20, pid: 600, title: "\u03c0 - Lighthouse", opened: "kitty" },
          "21": { id: 21, pid: 600, title: "\u03c0 - Lighthouse", opened: "pi-session.7",
            workspace_id: 3, layout: { pos_in_scrolling_layout: [2, 1] } }
        })
        property int revision: 0
        property var current: null
        property var acted: null
        property var spawned: []
        property var spawns: []
        function act(request) { acted = request; }
        // Asked anything, as Sessions asks for the outputs, it never
        // answers, as niri doesn't a request it refused.
        function request(asked, answer) {}
        // No workspace is known, the focused one included.
        function workspace(id) { return null; }
        function spawn(command) {
          spawned = command;
          spawns = spawns.concat([command.join(" ")]);
        }
      '';
      options = singleton "Options.qml" ''
        readonly property var sounds: ({ wants: "", player: "" })
      '';
      quiet = singleton "Quiet.qml" ''
        property bool on: false
      '';
      notices = singleton "Notices.qml" ''
        property var live: ({})
        function mark(id, read) {}
      '';
      waiting = singleton "WaitingOnYou.qml" ''
        readonly property var standing: ({})
      '';
      report = singleton "Report.qml" "function say(text) {}";
    in
    pkgs.runCommandLocal "a-session-is-known-by-its-own-record" { } ''
      export HOME=$TMPDIR XDG_STATE_HOME=$PWD/state XDG_DATA_HOME=$PWD/data
      ${lib.getExe pkgs.bash} ${../tests/stand-ins/sessions-state.sh} $XDG_STATE_HOME
      export PATH=$XDG_STATE_HOME/bin:$PATH
      ${copied [
        "Daemon.qml"
        "PiSource.qml"
        "Sessions.qml"
      ]}
      cp ${waiting} WaitingOnYou.qml
      cp ${report} Report.qml
      cp ${../../../tests/sessions.qml} shell.qml
      cp ${niri} Niri.qml
      cp ${options} Options.qml
      cp ${quiet} Quiet.qml
      cp ${away} Away.qml
      cp ${notices} Notices.qml
      export XDG_RUNTIME_DIR=$TMPDIR/runtime LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen
      mkdir -m 700 $XDG_RUNTIME_DIR $XDG_RUNTIME_DIR/mycelium $XDG_RUNTIME_DIR/daemons
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      timeout 60 ${qs} -p shell.qml
      touch $out
    '';

  # A session handed over while the screen is locked, beside its sender in
  # the sessions' kitty, waits once the lock is gone until the node has
  # said which window is the sender's, then goes to the sender's workspace
  # (tests/sessions-behind-the-lock.qml), with niri stood in.
  a-session-behind-the-lock-waits-for-its-kin =
    let
      # The sender's kitty's two windows, on the second workspace, the
      # second opened as pi-session.3, each with the app id it opened with
      # kept, as Niri.qml keeps it.
      niri = singleton "Niri.qml" ''
        property var windows: ({
          "20": { id: 20, pid: 600, workspace_id: 2, opened: "kitty" },
          "21": { id: 21, pid: 600, workspace_id: 2, opened: "pi-session.3" }
        })
        property int revision: 0
        property var current: null
        property var focused: null
        property var acts: []
        property var spawns: []
        signal windowOpenedOrChanged(var window)
        function act(request) { acts = acts.concat([request]); }
        function request(asked, answer) {}
        function workspace(id) {
          return ({ 1: { id: 1, idx: 1, output: "Virtual-1" },
            2: { id: 2, idx: 2, output: "Virtual-1" } })[id] ?? null;
        }
        function spawn(command) { spawns = spawns.concat([command.join(" ")]); }
      '';
      options = singleton "Options.qml" ''
        readonly property var sounds: ({ wants: "", player: "" })
      '';
      quiet = singleton "Quiet.qml" ''
        property bool on: false
      '';
      notices = singleton "Notices.qml" ''
        property var live: ({})
        function mark(id, read) {}
      '';
      waiting = singleton "WaitingOnYou.qml" ''
        readonly property var standing: ({})
      '';
      report = singleton "Report.qml" "function say(text) {}";
    in
    pkgs.runCommandLocal "a-session-behind-the-lock-waits-for-its-kin" { } ''
      export HOME=$TMPDIR XDG_STATE_HOME=$PWD/state XDG_DATA_HOME=$PWD/data
      ${lib.getExe pkgs.bash} ${../tests/stand-ins/sessions-behind-the-lock-state.sh} $XDG_STATE_HOME
      ${copied [
        "Daemon.qml"
        "PiSource.qml"
        "Sessions.qml"
      ]}
      cp ${waiting} WaitingOnYou.qml
      cp ${report} Report.qml
      cp ${../../../tests/sessions-behind-the-lock.qml} shell.qml
      cp ${niri} Niri.qml
      cp ${options} Options.qml
      cp ${quiet} Quiet.qml
      cp ${away} Away.qml
      cp ${notices} Notices.qml
      export XDG_RUNTIME_DIR=$TMPDIR/runtime LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen
      mkdir -m 700 $XDG_RUNTIME_DIR $XDG_RUNTIME_DIR/mycelium
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      timeout 60 ${qs} -p shell.qml
      touch $out
    '';

  # Reopen Them All opens only the viewer of a session pi's host brought
  # back, quietly where the last boot kept it (tests/sessions-hosted-reopen.qml),
  # with the node and niri stood in.
  a-hosted-viewer-reopens-where-it-was =
    let
      # Joel's window on the monitor's first workspace, and the empty one
      # niri keeps last; the test makes more as windows open there.
      niri = ../tests/stand-ins/hosted-reopen-niri.qml;
      options = singleton "Options.qml" ''
        readonly property var sounds: ({ wants: "", player: "" })
      '';
      quiet = singleton "Quiet.qml" ''
        property bool on: false
      '';
      notices = singleton "Notices.qml" ''
        property var live: ({})
        function mark(id, read) {}
      '';
      waiting = singleton "WaitingOnYou.qml" ''
        readonly property var standing: ({})
      '';
      report = singleton "Report.qml" "function say(text) {}";
    in
    # Kept in Nix because each line copies a file Nix gives it beside the
    # cases, or sets what Quickshell runs them with.
    pkgs.runCommandLocal "a-hosted-viewer-reopens-where-it-was" { } ''
      export HOME=$TMPDIR XDG_STATE_HOME=$PWD/state XDG_DATA_HOME=$PWD/data
      ${lib.getExe pkgs.bash} ${../tests/stand-ins/sessions-behind-the-lock-state.sh} $XDG_STATE_HOME
      ${copied [
        "Daemon.qml"
        "PiSource.qml"
        "Sessions.qml"
      ]}
      cp ${waiting} WaitingOnYou.qml
      cp ${report} Report.qml
      cp ${../../../tests/sessions-hosted-reopen.qml} shell.qml
      cp ${../../../tests/lib/Expect.js} Expect.js
      cp ${../../../tests/lib/Watch.js} Watch.js
      cp ${niri} Niri.qml
      cp ${options} Options.qml
      cp ${quiet} Quiet.qml
      cp ${away} Away.qml
      cp ${notices} Notices.qml
      export XDG_RUNTIME_DIR=$TMPDIR/runtime LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen
      mkdir -m 700 $XDG_RUNTIME_DIR $XDG_RUNTIME_DIR/mycelium
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      timeout 60 ${qs} -p shell.qml
      touch $out
    '';

  # A fresh session the harness hands over is asked of its spawn decider
  # before a window opens (tests/sessions-spawn-decides.qml), with niri and
  # the decider stood in.
  a-spawn-kept-in-the-background-opens-no-window =
    let
      options = singleton "Options.qml" ''
        readonly property var sounds: ({ wants: "", player: "" })
      '';
      quiet = singleton "Quiet.qml" ''
        property bool on: false
      '';
      notices = singleton "Notices.qml" ''
        property var live: ({})
        function mark(id, read) {}
      '';
      waiting = singleton "WaitingOnYou.qml" ''
        readonly property var standing: ({})
      '';
      report = singleton "Report.qml" "function say(text) {}";
    in
    # Kept in Nix because each line copies a file Nix gives it beside the
    # cases, or sets what Quickshell runs them with.
    pkgs.runCommandLocal "a-spawn-kept-in-the-background-opens-no-window" { } ''
      export HOME=$TMPDIR XDG_STATE_HOME=$PWD/state XDG_DATA_HOME=$PWD/data
      ${lib.getExe pkgs.bash} ${../tests/stand-ins/sessions-behind-the-lock-state.sh} $XDG_STATE_HOME
      ${copied [
        "Daemon.qml"
        "PiSource.qml"
        "Sessions.qml"
      ]}
      cp ${waiting} WaitingOnYou.qml
      cp ${report} Report.qml
      cp ${../../../tests/sessions-spawn-decides.qml} shell.qml
      cp ${../../../tests/lib/Expect.js} Expect.js
      cp ${../tests/stand-ins/hosted-reopen-niri.qml} Niri.qml
      cp ${options} Options.qml
      cp ${quiet} Quiet.qml
      cp ${away} Away.qml
      cp ${notices} Notices.qml
      export SPAWN_DECIDE=${../tests/stand-ins/spawn-decide.sh} DECIDED=$PWD/decided
      touch $DECIDED
      mkdir -p $HOME/.config/pi/agentic-harness.pi
      echo '{"quest-workflow": {"startHost": "attended"}}' \
        > $HOME/.config/pi/agentic-harness.pi/config.json
      export PATH=${lib.makeBinPath [ pkgs.bash ]}:$PATH
      export XDG_RUNTIME_DIR=$TMPDIR/runtime LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen
      mkdir -m 700 $XDG_RUNTIME_DIR $XDG_RUNTIME_DIR/mycelium
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      timeout 60 ${qs} -p shell.qml
      touch $out
    '';
}
