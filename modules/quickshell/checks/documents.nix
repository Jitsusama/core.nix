# The checks of the launcher's Documents, Drive's among them, which the folder's
# checks.nix gathers with every other place's; its opening comment says why each
# place keeps its own.
{
  account,
  graphical,
  noFonts,
  pkgs,
  qs,
  shellFolderLint,
  singleton,
  ...
}:
{
  # The launcher's Documents make no row while it's shut, however the clock
  # and the google daemon's rows move, and make them once as it opens, in
  # the daemon's order (tests/documents.qml), through the real Drive.qml
  # and Daemon.qml, with Google's account, the clock, the mail and the
  # saves stood in.
  a-shut-launcher-makes-no-document-rows =
    let
      # Kept in Nix because it only stands in for what Drive.qml reads of
      # Google's, answering nothing.
      google = singleton "Google.qml" ''
        property var documents: []
        property var drive: ({ places: {}, kept: {}, listings: [], downloads: [] })
        property var account: null
        function list(folder) {}
        function listing(folder, since) { return null; }
        function download(file, how, to) {}
        function downloading(file, since) { return null; }
      '';
      calendar = singleton "Calendar.qml" ''property real now: Date.parse("2026-11-10T12:00:00Z")'';
      mail = singleton "Mail.qml" "property var dates: ({})";
      saves = singleton "Saves.qml" "property var launcher: null";
      niri = singleton "Niri.qml" "function spawn(command) {}";
    in
    # Kept in Nix because each line copies a file Nix gives it beside the
    # cases, or sets what Quickshell runs them with.
    pkgs.runCommandLocal "a-shut-launcher-makes-no-document-rows" { } ''
      mkdir -p launcher
      cp ${../../../tests/lib/Expect.js} Expect.js
      cp ${(account graphical).xdg.configFile."quickshell/Theme.qml".source} Theme.qml
      cp ${../Fuzzy.js} Fuzzy.js
      cp ${../ProgramGlyphs.js} ProgramGlyphs.js
      cp ${../Words.qml} Words.qml
      cp ${../FileFacts.js} FileFacts.js
      cp ${../When.js} When.js
      cp ${../launcher/Provider.qml} launcher/Provider.qml
      cp ${../launcher/Asking.js} launcher/Asking.js
      cp ${../launcher/Numbers.js} launcher/Numbers.js
      cp ${../launcher/words.json} launcher/words.json
      cp ${../Daemon.qml} Daemon.qml
      cp ${../Hearing.js} Hearing.js
      cp ${../launcher/Placed.js} launcher/Placed.js
      cp ${../../google/Documents.qml} launcher/Documents.qml
      cp ${../../google/Drive.qml} Drive.qml
      cp ${../../google/Drive.js} Drive.js
      cp ${../../google/DrivePlaces.js} DrivePlaces.js
      cp ${../../google/Mail.js} Mail.js
      cp ${google} Google.qml
      cp ${calendar} Calendar.qml
      cp ${mail} Mail.qml
      cp ${saves} Saves.qml
      cp ${niri} Niri.qml
      cp ${../../../tests/documents.qml} shell.qml
      export HOME=$TMPDIR XDG_RUNTIME_DIR=$TMPDIR/runtime LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen
      export TZ=UTC FONTCONFIG_FILE=${noFonts}
      mkdir -m 700 $XDG_RUNTIME_DIR
      ${shellFolderLint}
      timeout 60 ${qs} -p shell.qml
      touch $out
    '';
}
