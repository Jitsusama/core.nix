# The checks of the desktop's reading of spend-watch, which the folder's
# checks.nix gathers with every other place's; its opening comment says why each
# place keeps its own.
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
  # The AI spend piece and the tokens piece read what spend-watch writes, from a
  # day of a session's turns on two models, and the states
  # modules/quickshell/tests/spend.qml writes by hand, through Quickshell
  # itself, with niri, do not disturb, the shortcuts and the sessions stood in
  # for; the launcher's collection holds the rows `spend-watch serve` serves
  # ready from those files (modules/quickshell/tests/spend-served.qml), its
  # budgets' drawers among them; and what its socket tells, from a stand-in
  # serving those records (modules/quickshell/tests/spend-told.qml), a change
  # remaking only the rows it changed. First, how a row reads while Spend's rows
  # aren't wanted (launcher/SpendNow.js), held in Qt's own engine by
  # modules/quickshell/tests/spend-now.qml.
  the-desktop-reads-the-spend-watch =
    let
      qt = pkgs.qt6.qtdeclarative;
      watch = pkgs.callPackage ../../spend-watch/package.nix { };
      niri = singleton "Niri.qml" ''
        property var spawned: []
        function spawn(command) { spawned = command; }
      '';
      sessions = singleton "Sessions.qml" ''
        property var running: [{
          address: "ADDR1",
          title: "The Launcher's Shape",
          presence: { session: "s-running" }
        }]
        property string steered: ""
        function steer(address, fields) { steered = address + " " + fields.act; }
      '';
      shortcuts = singleton "Shortcuts.qml" ''
        property var actions: [{ opens: "spend", chord: [{ text: "Super" }, { text: "U" }] }]
      '';
    in
    pkgs.runCommandLocal "the-desktop-reads-the-spend-watch" { } ''
      mkdir -p runtime/written runtime/spend-watch runtime/daemons sessions/thing state launcher
      chmod 700 runtime
      ${lib.getExe pkgs.bash} ${../tests/stand-ins/spend-session.sh} sessions/thing/session.jsonl
      cat > config.json <<EOF
      {"sessions": "$PWD/sessions", "runs": "$PWD/runs.db", "registry": "$PWD/registry",
       "quests": "$PWD/quests", "state": "$PWD/state", "runtime": "$PWD/runtime/written",
       "zone": "UTC", "notices": {"enable": false}}
      EOF
      # serve reads where spend-watch writes, worded by the launcher's own
      # vocabulary, as the home module runs it.
      cat > serve.json <<EOF
      {"sessions": "$PWD/sessions", "runs": "$PWD/runs.db", "registry": "$PWD/registry",
       "quests": "$PWD/quests", "state": "$PWD/state", "runtime": "$PWD/runtime/spend-watch",
       "zone": "UTC", "notices": {"enable": false},
       "vocabulary": "${(import ../launcher/words.nix).file}"}
      EOF
      ${lib.getExe watch} --config config.json import
      ${lib.getExe watch} --config config.json show > runtime/written/status.json
      ${lib.getExe watch} --config config.json show collection > runtime/written/collection.json
      ${lib.getExe watch} --config config.json budgets --json > runtime/written/budgets.json
      cp ${../pieces/Piece.qml} Piece.qml
      cp ${../pieces/Spend.qml} Spend.qml
      cp ${../pieces/Tokens.qml} Tokens.qml
      cp ${sessions} Sessions.qml
      cp ${../pieces/StatusFile.qml} StatusFile.qml
      ${copied [
        "Daemon.qml"
        "Options.qml"
        "SpendWatch.qml"
        "Words.qml"
        "launcher/Catalogue.js"
        "launcher/Provider.qml"
        "launcher/ReadyRows.qml"
        "launcher/Spend.qml"
        "launcher/SpendDrawer.js"
        "launcher/words.json"
      ]}
      cp ${(account graphical).xdg.configFile."quickshell/Theme.qml".source} Theme.qml
      cp ${../tests/spend.qml} shell.qml
      cp ${../tests/spend-served.qml} served.qml
      cp ${../tests/spend-told.qml} told.qml
      cp ${../tests/spend-now.qml} spend-now.qml
      cp ${../../../tests/lib/Expect.js} Expect.js
      cp ${../../../tests/lib/Until.qml} Until.qml
      cp ${niri} Niri.qml
      cp ${singleton "Quiet.qml" "property bool sharing: false"} Quiet.qml
      cp ${shortcuts} Shortcuts.qml
      export HOME=$TMPDIR XDG_RUNTIME_DIR=$TMPDIR/runtime LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      # Each program may take longer than all its ten-second waits together,
      # and one stopped says which and why in the check's own words, since a
      # builder may keep no log of a failed try.
      runs() {
        limit=$1
        shift
        timeout "$limit" "$@" || {
          status=$?
          if [ "$status" = 124 ]; then
            echo "FAIL $* ran past $limit s" >&2
          else
            echo "FAIL $* ended with $status" >&2
          fi
          exit "$status"
        }
      }
      runs 180 ${qs} -p shell.qml
      SPEND_WATCH=${lib.getExe watch} SERVE_CONFIG=$PWD/serve.json runs 180 ${qs} -p served.qml
      # Serve ended with the shell, its socket left behind for the stand-in.
      rm -f runtime/spend-watch/socket
      cp runtime/written/collection.json runtime/written/budgets.json runtime/spend-watch/
      runs 180 ${qs} -p told.qml
      touch $out
    '';
}
