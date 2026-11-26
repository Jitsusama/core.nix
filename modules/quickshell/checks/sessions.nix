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

  # The sessions' kitty is asked for windows only over its own socket, in a
  # runtime folder only Joel's login opens, never as a single instance whose
  # name any program could take first and be handed every session's window.
  the-sessions-kitty-answers-only-its-own-socket =
    let
      service = (account graphical).systemd.user.services.sessions-kitty.Service;
      started = lib.concatStringsSep " " (lib.toList (service.ExecStart or [ ]));
    in
    holds "the-sessions-kitty-answers-only-its-own-socket"
      "The sessions' kitty answers a single instance or a socket others can open, so another program could take its windows."
      (
        lib.hasInfix "--listen-on unix:%t/pi-sessions/kitty.sock" started
        && lib.hasInfix "-o allow_remote_control=socket-only" started
        && !(lib.hasInfix "--instance-group" started)
        && !(lib.hasInfix " -1 " started)
        && (service.RuntimeDirectory or null) == "pi-sessions"
        && (service.RuntimeDirectoryMode or null) == "0700"
      );

  # The sessions' kitty writes its pid beside its socket, which names none,
  # so the shell finds a session's window among those that kitty draws,
  # and then tells the shell it started, as a watch on the file misses one
  # written in a folder made anew, and a kitty started again draws nothing
  # till asked, so a viewer kept for one would wait on what never comes.
  # The write first, failing the start where it fails, as a kitty with no
  # pid beside it would have every window refused; then the call, bounded
  # and only journalled where it fails, as at login the shell may not
  # listen yet and a failed call would stop the kitty; and by the name the
  # shell's "sessions" IpcHandler answers.
  the-sessions-kitty-says-its-pid =
    let
      service = (account graphical).systemd.user.services.sessions-kitty.Service;
      written = lib.concatStringsSep " " (lib.toList (service.ExecStartPost or [ ]));
      inOrder =
        ".*%t/pi-sessions/kitty\\.pid \\|\\| exit 1;"
        + " [^;|]*/timeout [0-9]+ [^;|]* ipc call sessions kittyStarted [^;|]*"
        + "\\|\\| echo [^;|]*>&2'";
      # The "sessions" IpcHandler, from its target to its closing brace.
      handler = lib.splitString "target: \"sessions\"" (builtins.readFile ../Sessions.qml);
      answered =
        builtins.length handler == 2
        && lib.hasInfix "function kittyStarted(): void {\n            root.kittyStarted();" (
          lib.head (lib.splitString "\n    }\n" (lib.last handler))
        );
    in
    holds "the-sessions-kitty-says-its-pid"
      "The sessions' kitty writes no pid beside its socket, or never tells the shell it started, so the shell can't tell which windows it draws, or waits on a kitty come back."
      (lib.hasInfix "$$MAINPID" written && builtins.match inOrder written != null && answered);

  # A session's window is asked of the sessions kitty over its socket, or of
  # the unit's own kitty from before it listened, waiting for one starting
  # again and opening nothing with neither, as the script the shell links
  # and runs does, through each door (modules/quickshell/tests/
  # opens-in-sessions-doors.sh), and the packaged kitten says an unanswered
  # launch, and a socket nothing listens on, in the words the script reads.
  # Each door's run is cut at 20 s, and the whole at twenty minutes, past
  # every run's cut together (35 runs and the packaged kitten's half
  # minute, under twelve minutes), so a wait that runs on fails by name,
  # even in each run at once.
  the-sessions-kitty-is-asked-only-its-own-way =
    let
      linked = (account graphical).xdg.configFile."quickshell/opens-in-sessions.sh".source;
    in
    pkgs.runCommandLocal "the-sessions-kitty-is-asked-only-its-own-way" { } ''
      ${lib.getExe' pkgs.coreutils "timeout"} 1200 ${lib.getExe pkgs.bash} \
        ${../tests/opens-in-sessions-doors.sh} ${pkgs.bash}/bin/sh ${linked} \
        ${lib.getExe pkgs.python3} ${../Sessions.qml} \
        ${lib.getExe' pkgs.kitty "kitten"}
      touch $out
    '';

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
  # only while the machine queue says there's room, as
  # modules/quickshell/tests/places.qml holds Places.js, Reopening.js and
  # Viewers.js to it in Qt's own engine.
  reopening-finds-each-screen =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "reopening-finds-each-screen" { } ''
      cp ${../tests/places.qml} places.qml
      ${copied [
        "Places.js"
        "Reopening.js"
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
  # in (modules/quickshell/tests/sessions.qml), with the node, the agents
  # daemon, niri and kitten stood in over what
  # modules/quickshell/tests/stand-ins/sessions-state.sh lays out.
  a-session-is-known-by-its-own-record =
    let
      # Three windows of the same title, one of kitty 500's and two of 600's,
      # the second opened as pi-session.7 and laid out in workspace 3's
      # second column, each with the app id it opened with kept, as Niri.qml
      # keeps it; the windows daemon tells of the same three
      # (modules/quickshell/tests/sessions.qml).
      niri = singleton "Niri.qml" ''
        property var windows: ({
          "10": { id: 10, pid: 500, title: "\u03c0 - Lighthouse", opened: "kitty" },
          "20": { id: 20, pid: 600, title: "\u03c0 - Lighthouse", opened: "kitty" },
          "21": { id: 21, pid: 600, title: "\u03c0 - Lighthouse", opened: "pi-session.7",
            workspace_id: 3, layout: { pos_in_scrolling_layout: [2, 1] } }
        })
        property int revision: 0
        // Whether niri's own stream stands in for a windows daemon that
        // isn't answering, as Niri.qml says it; a case sets it.
        property bool standIn: false
        property var current: null
        // The window Joel is in, none, which a press's goingTo keeps.
        property var focused: null
        // Workspace 5, where a sleeper's window was, and 8, named for the
        // orchestrator of a session elsewhere, Joel on another.
        property var workspaces: ({
          "5": { id: 5, is_focused: false },
          "8": { id: 8, name: "FARLED", is_focused: false }
        })
        property var acted: null
        property var spawned: []
        property var spawns: []
        // Said by a case as a viewer's window comes, as niri says it.
        signal windowOpenedOrChanged(var window)
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
      cp ${../tests/sessions.qml} shell.qml
      cp ${niri} Niri.qml
      cp ${options} Options.qml
      cp ${quiet} Quiet.qml
      cp ${away} Away.qml
      cp ${notices} Notices.qml
      export XDG_RUNTIME_DIR=$TMPDIR/runtime LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen
      mkdir -m 700 $XDG_RUNTIME_DIR $XDG_RUNTIME_DIR/mycelium $XDG_RUNTIME_DIR/daemons \
        $XDG_RUNTIME_DIR/pi-sessions
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      timeout 60 ${qs} -p shell.qml
      touch $out
    '';

  # A session handed over while the screen is locked, beside its sender in
  # the sessions' kitty, waits once the lock is gone until the node has
  # said which window is the sender's, then goes to the sender's workspace
  # (modules/quickshell/tests/sessions-behind-the-lock.qml), with niri stood in.
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
      cp ${../tests/sessions-behind-the-lock.qml} shell.qml
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
  # back, quietly where the last boot kept it
  # (modules/quickshell/tests/sessions-hosted-reopen.qml), with the node and
  # niri stood in. Its last stages walk plain pis too, since only this case
  # has a realm, a node and a niri that opens windows.
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
      cp ${../tests/sessions-hosted-reopen.qml} shell.qml
      cp ${../../../tests/lib/Expect.js} Expect.js
      cp ${../../../tests/lib/Watch.js} Watch.js
      cp ${niri} Niri.qml
      cp ${options} Options.qml
      cp ${quiet} Quiet.qml
      cp ${away} Away.qml
      cp ${notices} Notices.qml
      export XDG_RUNTIME_DIR=$TMPDIR/runtime LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen
      mkdir -m 700 $XDG_RUNTIME_DIR $XDG_RUNTIME_DIR/mycelium $XDG_RUNTIME_DIR/pi-sessions
      # The sessions' kitty's pid, as its unit writes it, so each viewer is
      # judged by the pid that drew it, as on Joel's desktop.
      echo 4242 > $XDG_RUNTIME_DIR/pi-sessions/kitty.pid
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      # Its walks go at the walk's pace, its named stages follow its led
      # ones, and its last looks a walk's few paces after it asks, so the
      # case runs for over a minute.
      timeout 180 ${qs} -p shell.qml
      touch $out
    '';

  # Go To on a session with no window here brings the window it opens to the
  # front once niri lists it
  # (modules/quickshell/tests/sessions-go-to-focuses.qml), with the node and
  # niri stood in.
  go-to-brings-a-windowless-session-forward =
    let
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
    pkgs.runCommandLocal "go-to-brings-a-windowless-session-forward" { } ''
      export HOME=$TMPDIR XDG_STATE_HOME=$PWD/state XDG_DATA_HOME=$PWD/data
      ${lib.getExe pkgs.bash} ${../tests/stand-ins/sessions-behind-the-lock-state.sh} $XDG_STATE_HOME
      ${copied [
        "Daemon.qml"
        "PiSource.qml"
        "Sessions.qml"
      ]}
      cp ${waiting} WaitingOnYou.qml
      cp ${report} Report.qml
      cp ${../tests/sessions-go-to-focuses.qml} shell.qml
      cp ${../../../tests/lib/Expect.js} Expect.js
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

  # A fresh session the harness hands over is asked of its spawn decider before
  # a window opens (modules/quickshell/tests/sessions-spawn-decides.qml), with
  # niri and the decider stood in.
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
      cp ${../tests/sessions-spawn-decides.qml} shell.qml
      cp ${../../../tests/lib/Expect.js} Expect.js
      cp ${../tests/stand-ins/hosted-reopen-niri.qml} Niri.qml
      cp ${options} Options.qml
      cp ${quiet} Quiet.qml
      cp ${away} Away.qml
      cp ${notices} Notices.qml
      export SPAWN_DECIDE=${../tests/stand-ins/spawn-decide.sh} DECIDED=$PWD/decided
      touch $DECIDED
      mkdir -p $HOME/.config/pi/agentic-harness.pi
      # The harness keeps each extension's settings under sections
      # (lib/internal/config/loader.ts), as Joel's own file does.
      echo '{"version": 1, "sections": {"quest-workflow": {"startHost": "attended"}}}' \
        > $HOME/.config/pi/agentic-harness.pi/config.json
      export PATH=${lib.makeBinPath [ pkgs.bash ]}:$PATH
      export XDG_RUNTIME_DIR=$TMPDIR/runtime LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen
      mkdir -m 700 $XDG_RUNTIME_DIR $XDG_RUNTIME_DIR/mycelium $XDG_RUNTIME_DIR/pi-sessions
      # The sessions' kitty's pid, as its unit writes it, so each viewer is
      # judged by the pid that drew it, as on Joel's desktop.
      echo 4242 > $XDG_RUNTIME_DIR/pi-sessions/kitty.pid
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      timeout 60 ${qs} -p shell.qml
      touch $out
    '';

  # A monitor plugged back in takes back the workspace niri kept on the laptop's
  # screen, even when niri is late to say the monitor is there
  # (modules/quickshell/tests/sessions-screen-comes-home.qml), with niri stood
  # in.
  a-screen-niri-is-late-to-name-takes-its-workspaces-back =
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
    pkgs.runCommandLocal "a-screen-niri-is-late-to-name-takes-its-workspaces-back" { } ''
      export HOME=$TMPDIR XDG_STATE_HOME=$PWD/state XDG_DATA_HOME=$PWD/data
      ${lib.getExe pkgs.bash} ${../tests/stand-ins/sessions-behind-the-lock-state.sh} $XDG_STATE_HOME
      ${copied [
        "Daemon.qml"
        "PiSource.qml"
        "Sessions.qml"
      ]}
      cp ${waiting} WaitingOnYou.qml
      cp ${report} Report.qml
      cp ${../tests/sessions-screen-comes-home.qml} shell.qml
      cp ${../../../tests/lib/Expect.js} Expect.js
      cp ${../tests/stand-ins/hosted-reopen-niri.qml} Niri.qml
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
}
