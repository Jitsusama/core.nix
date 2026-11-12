# The checks of the launcher's Agents: pi's sessions, the talks, runs and queue
# the realm's node serves, and asking a fresh pi, which the folder's checks.nix
# gathers with every other place's; its opening comment says why each place
# keeps its own.
{
  account,
  copied,
  graphical,
  launcherOnNiri,
  lib,
  noFonts,
  pkgs,
  qs,
  self,
  shellFolderLint,
  singleton,
  ...
}:
{
  # An agent's row does, on Return, the act its state makes harmless, its
  # rows the agents daemon's, made from the realm's own node's sessions, as
  # a workstation's are (tests/agents-on-niri-test.py).
  an-agents-return-does-its-harmless-act-on-niri = launcherOnNiri {
    name = "an-agents-return-does-its-harmless-act-on-niri";
    tests = ../../../tests/agents-on-niri-test.py;
    given = [
      "MYCELIUM=${lib.getExe self.inputs.mycelium.packages.x86_64-linux.default}"
      "AGENTS=${lib.getExe (pkgs.callPackage ../../agents/package.nix { })}"
      "WORDS=${(import ../launcher/words.nix).file}"
    ];
  };

  # pi, as an agent source, sorts the node's sessions into those running,
  # parked and closed this boot, and says each as the agent source
  # contract's record, leaving out what pi didn't say, as
  # tests/pi-source.qml holds PiSource.js to it in Qt's own engine.
  pi-tells-of-its-sessions-as-an-agent-source =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "pi-tells-of-its-sessions-as-an-agent-source" { } ''
      cp ${../../../tests/pi-source.qml} pi-source.qml
      ${copied [ "PiSource.js" ]}
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml pi-source.qml
      touch $out
    '';

  # pi's sessions, as many and as large as Joel's, each changing its step
  # with the launcher shut and then showing them, sorted and lived again as
  # Sessions.qml does, start a garbage collection in well under one change
  # in five, as QV4's own report of each collection counts them. Told apart
  # as text they had started one nearly every change (99 in 100), each a
  # sweep that stalls the shell's main thread.
  pi-sorts-without-collecting =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "pi-sorts-without-collecting" { } ''
      cp ${../../../tests/pi-source-collects.qml} pi-source-collects.qml
      ${copied [ "PiSource.js" ]}
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      export QML_IMPORT_PATH=${qt}/lib/qt-6/qml QT_LOGGING_RULES=qt.qml.gc.allocatorStats=true
      ${shellFolderLint}
      timeout 120 ${qt}/bin/qml pi-source-collects.qml > said 2>&1 || { cat said; exit 1; }
      awk '/triggered by unmanaged heap:/ { n++ } END { print n + 0, "started"; exit n >= 20 }' said
      touch $out
    '';

  # A pi session says how it stands and its acts, and its drawer's times
  # are worded, as tests/session-row.qml holds launcher/SessionRow.js and
  # SessionDrawer.js to them in Qt's own engine, and Landed what they
  # landed, kept past their end (Landed.js). How Agents lays sessions out,
  # and what its drawer says, is the agents daemon's, proven in its crate.
  the-launcher-says-how-each-session-stands =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "the-launcher-says-how-each-session-stands" { } ''
      cp ${../../../tests/session-row.qml} session-row.qml
      cp ${../launcher/Holders.js} Holders.js
      cp ${../launcher/SessionRow.js} SessionRow.js
      cp ${../launcher/Order.js} Order.js
      cp ${../launcher/Landed.js} Landed.js
      cp ${../launcher/SessionDrawer.js} SessionDrawer.js
      ${copied [ "Reading.js" ]}
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml session-row.qml
      touch $out
    '';

  # Joel's line to the one standing at Agents' top goes out as one quiet
  # card, sent, then answered in its place with the reply cut to whole
  # words, as tests/told.qml holds Told.js to in Qt's own engine.
  the-void-answers-on-one-card =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "the-void-answers-on-one-card" { } ''
      cp ${../../../tests/told.qml} told.qml
      ${copied [ "Told.js" ]}
      cp ${../../../tests/lib/Expect.js} Expect.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml told.qml
      touch $out
    '';

  # The machine queue is the harness's file as the node's record holds it,
  # and a write of only when it was written says nothing new, as
  # tests/jobs.qml holds Jobs.js to it in Qt's own engine.
  the-queue-is-heard-from-the-node =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "the-queue-is-heard-from-the-node" { } ''
      cp ${../../../tests/jobs.qml} jobs.qml
      ${copied [ "Jobs.js" ]}
      cp ${../../../tests/lib/Expect.js} Expect.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml jobs.qml
      touch $out
    '';

  # The talks Agents shows (tests/talks.qml), heard from a stand-in for the
  # realm's node serving each talk as a record of its lines: by its
  # session's address, newest first, moved on by each line told, one made
  # since heard and one let go gone, with no file of the talks read here.
  the-launcher-hears-talks-from-the-node =
    let
      niri = singleton "Niri.qml" ''
        property int revision: 0
        function spawn(command) {}
        function act(request) {}
      '';
      sessions = singleton "Sessions.qml" ''
        readonly property string home: Quickshell.env("HOME")
        readonly property string state: Quickshell.env("PWD") + "/state"
        readonly property string nodePath: Quickshell.env("XDG_RUNTIME_DIR")
          + "/mycelium/harbour.sock"
        property var running: [{ address: "A003" }, { address: "A117" }]
        property var parked: []
        property var present: ({})
        function asks(session) { return false; }
        function resume(id, quest, folder, gone) {}
        function start(quest, folder) {}
      '';
      waiting = singleton "WaitingOnYou.qml" "readonly property var standing: ({})";
      jobs = singleton "Jobs.qml" "readonly property var jobs: []";
    in
    pkgs.runCommandLocal "the-launcher-hears-talks-from-the-node" { } ''
      mkdir -p launcher state/quest-workflow
      echo '{"orchestrators":[{"address":"A003","quest":"QEST-A","title":"A"}]}' \
        > state/quest-workflow/orchestrators.json
      cp ${(account graphical).xdg.configFile."quickshell/Theme.qml".source} Theme.qml
      ${copied [
        "AgentSources.qml"
        "Daemon.qml"
        "PiSource.qml"
        "Words.qml"
        "launcher/Orchestrators.qml"
        "launcher/Provider.qml"
        "launcher/words.json"
      ]}
      cp ${../../../tests/talks.qml} shell.qml
      cp ${niri} Niri.qml
      cp ${sessions} Sessions.qml
      cp ${waiting} WaitingOnYou.qml
      cp ${jobs} Jobs.qml
      export HOME=$TMPDIR XDG_RUNTIME_DIR=$TMPDIR/runtime LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen
      mkdir -m 700 $XDG_RUNTIME_DIR $XDG_RUNTIME_DIR/mycelium
      # pi's source finds the node by the realm the node keeps.
      mkdir -p $HOME/.local/state/mycelium && echo harbour > $HOME/.local/state/mycelium/realm
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      timeout 60 ${qs} -p shell.qml
      touch $out
    '';

  # The launcher's Ask row says why when pi-ask can't start, and gives each
  # question a pi of its own (tests/ask.qml), over a pi-ask that ends with
  # an error and the desktop tests' stand-in, which speaks pi's RPC as the
  # work machine's pi does.
  the-launcher-asks-a-fresh-pi =
    let
      broken = pkgs.writeShellScript "pi-ask-broken" ''
        echo "Error: Failed to get a token." >&2
        exit 1
      '';
      standIn = pkgs.writers.writePython3 "stand-in-ask" {
        doCheck = false;
      } (builtins.readFile ../../../tests/desktop/stand-in-ask.py);
      niri = singleton "Niri.qml" "function spawn(command) {}";
      sessions = singleton "Sessions.qml" ''
        readonly property string home: Quickshell.env("HOME")
        property var running: []
        function tell(words, folder, program) {}
        function talk(address, words) { return true; }
      '';
    in
    pkgs.runCommandLocal "the-launcher-asks-a-fresh-pi" { } ''
      mkdir -p launcher runtime && chmod 700 runtime
      cp ${(account graphical).xdg.configFile."quickshell/Theme.qml".source} Theme.qml
      ${copied [
        "Words.qml"
        "launcher/Ask.qml"
        "launcher/Order.js"
        "launcher/Provider.qml"
        "launcher/words.json"
      ]}
      cp ${niri} Niri.qml
      cp ${sessions} Sessions.qml
      cp ${../../../tests/ask.qml} shell.qml
      export HOME=$TMPDIR XDG_RUNTIME_DIR=$PWD/runtime LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts} ASK_BROKEN=${broken} ASK_STAND_IN=${standIn}
      ${shellFolderLint}
      timeout 60 ${qs} -p shell.qml
      touch $out
    '';

  # The launcher's runs find pi's fleets and review rounds in their records
  # as the realm's mycelium node serves them, and go to the session that
  # started one or read a participant's answer (tests/runs.qml), with niri,
  # pi's sessions and the screen's sharing stood in, and no node, so the
  # case gives the records itself.
  the-launcher-finds-the-runs =
    let
      niri = singleton "Niri.qml" ''
        property var spawned: []
        property var acted: null
        function spawn(command) { spawned = command; }
        function act(request) { acted = request; }
      '';
      sessions = singleton "Sessions.qml" ''
        readonly property string nodePath: ""
        property var running: []
        property var parked: []
        function goTo(session) { Niri.act({ FocusWindow: { id: session.window.id } }); }
      '';
    in
    pkgs.runCommandLocal "the-launcher-finds-the-runs" { } ''
      mkdir launcher
      ${copied [
        "Daemon.qml"
        "Words.qml"
        "launcher/Provider.qml"
        "launcher/Runs.qml"
        "launcher/words.json"
      ]}
      cp ${(account graphical).xdg.configFile."quickshell/Theme.qml".source} Theme.qml
      cp ${../../../tests/runs.qml} shell.qml
      cp ${niri} Niri.qml
      cp ${sessions} Sessions.qml
      cp ${singleton "AgentSources.qml" "readonly property var runs: []"} AgentSources.qml
      cp ${singleton "Quiet.qml" "property bool sharing: false"} Quiet.qml
      export HOME=$TMPDIR XDG_RUNTIME_DIR=$TMPDIR/runtime LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen
      mkdir -m 700 $XDG_RUNTIME_DIR
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      timeout 60 ${qs} -p shell.qml
      touch $out
    '';

  # The launcher's queue, Agents' place for the machine's heavy work, says
  # what each job is from the queue pi's harness writes (tests/queue.qml),
  # with the queue, pi's sessions, niri, Quests and a slice's memory stood
  # in: names in words, times against the last run, what a waiting job
  # waits for, and where Return goes.
  the-launcher-finds-the-queue =
    let
      niri = singleton "Niri.qml" ''
        property var acted: null
        function act(request) { acted = request; }
      '';
      sessions = singleton "Sessions.qml" ''
        property var running: [{ presence: { session: "s1" }, window: { id: 7 }, address: "QUEUE1" }]
        function goTo(session) { Niri.act({ FocusWindow: { id: session.window.id } }); }
      '';
      jobs = singleton "Jobs.qml" ''
        property var said: null
        readonly property var jobs: said?.jobs ?? []
        readonly property var running: jobs.filter(job => job.state === "running")
        readonly property var waiting: jobs.filter(job => job.state === "waiting")
      '';
    in
    pkgs.runCommandLocal "the-launcher-finds-the-queue" { } ''
      mkdir -p launcher cgroups/work.scope
      # 5.8 GiB, held by the running job's slice.
      echo 6227702579 > cgroups/work.scope/memory.current
      cp ${(account graphical).xdg.configFile."quickshell/Theme.qml".source} Theme.qml
      ${copied [
        "Words.qml"
        "launcher/Provider.qml"
        "launcher/Queue.qml"
        "launcher/words.json"
      ]}
      cp ${../../../tests/queue.qml} shell.qml
      cp ${niri} Niri.qml
      cp ${sessions} Sessions.qml
      cp ${jobs} Jobs.qml
      export HOME=$TMPDIR XDG_RUNTIME_DIR=$TMPDIR/runtime LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen
      export CGROUPS=$PWD/cgroups
      mkdir -m 700 $XDG_RUNTIME_DIR
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      timeout 60 ${qs} -p shell.qml
      touch $out
    '';
}
