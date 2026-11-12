# The checks of the launcher's Quests, which the folder's checks.nix gathers
# with every other place's; its opening comment says why each place keeps its
# own.
{
  account,
  copied,
  graphical,
  lib,
  noFonts,
  pkgs,
  qs,
  shellFolderLint,
  singleton,
  ...
}:
{
  # The launcher's quests find pi's quests in the harness's two lists, open
  # and ended, as the quests daemon makes their rows from a stand-in for
  # the realm's node, and open pi on one or go to its session
  # (tests/quests.qml), with niri and pi's sessions stood in.
  the-launcher-finds-the-quests =
    let
      quests = lib.getExe (pkgs.callPackage ../../quests/package.nix { });
      niri = singleton "Niri.qml" ''
        property var spawned: []
        property var acted: null
        function spawn(command) { spawned = command; }
        function act(request) { acted = request; }
      '';
      sessions = singleton "Sessions.qml" ''
        readonly property string home: Quickshell.env("HOME")
        readonly property string nodePath: Quickshell.env("XDG_RUNTIME_DIR")
          + "/mycelium/harbour.sock"
        property var running: []
        property var parked: []
        function isOf(session) { return `window:''${session.window.id}`; }
        function goTo(session) { Niri.act({ FocusWindow: { id: session.window.id } }); }
        property var started: null
        function start(quest, folder, orchestrates) {
          started = orchestrates ? [quest, folder, "orchestrating"] : [quest, folder];
        }
        readonly property string machine: "alpha"
        property var places: []
        property string fits: ""
        readonly property bool choosesMachine: places.length > 1
        property int lookedAround: 0
        function lookAround() { lookedAround += 1; }
        property var startedOn: null
        function startOn(on, quest, folder) { startedOn = [on, quest, folder]; }
      '';
    in
    pkgs.runCommandLocal "the-launcher-finds-the-quests" { } ''
      mkdir launcher
      cp ${(account graphical).xdg.configFile."quickshell/Theme.qml".source} Theme.qml
      ${copied [
        "Daemon.qml"
        "Words.qml"
        "launcher/Provider.qml"
        "launcher/Quests.qml"
        "launcher/ReadyRows.qml"
        "launcher/words.json"
      ]}
      cp ${../../../tests/quests.qml} shell.qml
      cp ${niri} Niri.qml
      cp ${sessions} Sessions.qml
      export XDG_RUNTIME_DIR=$TMPDIR/runtime LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen
      # At home here, so a folder here reads from ~.
      export HOME=$PWD
      mkdir -m 700 $XDG_RUNTIME_DIR $XDG_RUNTIME_DIR/mycelium $XDG_RUNTIME_DIR/daemons
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      # The daemon follows the node the shell stands in, trying again
      # until it listens.
      ${quests} --vocabulary ${(import ../launcher/words.nix).file} \
        --node $XDG_RUNTIME_DIR/mycelium/harbour.sock \
        --socket $XDG_RUNTIME_DIR/daemons/quests.sock &
      timeout 60 ${qs} -p shell.qml
      touch $out
    '';
}
