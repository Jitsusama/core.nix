# The checks of the desktop's reading of spend-watch, which the folder's
# checks.nix gathers with every other place's; its opening comment says why each
# place keeps its own.
{
  account,
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
  # The AI spend piece and the tokens piece read what spend-watch writes,
  # from a day of a session's turns on two models, and the states
  # tests/spend.qml writes by hand, through Quickshell itself, with niri, do
  # not disturb, the shortcuts and the sessions stood in for; the
  # launcher's collection holds the rows `spend-watch serve` serves ready
  # from those files (tests/spend-served.qml), its budgets' drawers among
  # them; and what its socket tells, from a stand-in serving those records
  # (tests/spend-told.qml), a change remaking only the rows it changed.
  the-desktop-reads-the-spend-watch =
    let
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
       "zone": "UTC", "notices": {"enable": false}, "vocabulary": "${../launcher/words.json}"}
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
      cp ${../Fuzzy.js} Fuzzy.js
      cp ${../launcher/Provider.qml} launcher/Provider.qml
      cp ${../ProgramGlyphs.js} ProgramGlyphs.js
      cp ${../launcher/Asking.js} launcher/Asking.js
      cp ${../launcher/Numbers.js} launcher/Numbers.js
      cp ${../launcher/Spend.qml} launcher/Spend.qml
      cp ${../launcher/SpendDrawer.js} launcher/SpendDrawer.js
      cp ${../launcher/ReadyRows.qml} launcher/ReadyRows.qml
      cp ${../launcher/ReadyRows.js} launcher/ReadyRows.js
      cp ${../launcher/Ago.js} launcher/Ago.js
      cp ${../launcher/Interfaces.js} launcher/Interfaces.js
      cp ${../When.js} When.js
      cp ${(account graphical).xdg.configFile."quickshell/Theme.qml".source} Theme.qml
      cp ${../Words.qml} Words.qml
      cp ${../launcher/words.json} launcher/words.json
      cp ${../SpendWatch.qml} SpendWatch.qml
      cp ${../SpendWatch.js} SpendWatch.js
      cp ${../Daemon.qml} Daemon.qml
      cp ${../Hearing.js} Hearing.js
      cp ${../../../tests/spend.qml} shell.qml
      cp ${../../../tests/spend-served.qml} served.qml
      cp ${../../../tests/spend-told.qml} told.qml
      cp ${../../../tests/lib/Expect.js} Expect.js
      cp ${niri} Niri.qml
      cp ${singleton "Quiet.qml" "property bool sharing: false"} Quiet.qml
      cp ${shortcuts} Shortcuts.qml
      export HOME=$TMPDIR XDG_RUNTIME_DIR=$TMPDIR/runtime LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      timeout 60 ${qs} -p shell.qml
      SPEND_WATCH=${lib.getExe watch} SERVE_CONFIG=$PWD/serve.json timeout 90 ${qs} -p served.qml
      # Serve ended with the shell, its socket left behind for the stand-in.
      rm -f runtime/spend-watch/socket
      cp runtime/written/collection.json runtime/written/budgets.json runtime/spend-watch/
      timeout 60 ${qs} -p told.qml
      touch $out
    '';
}
