# The checks of the launcher's Documents, Drive's among them, which the folder's
# checks.nix gathers with every other place's; its opening comment says why each
# place keeps its own.
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
  # The launcher's Documents make no row while it's shut, however the clock
  # and the google daemon's rows move, and make them once as it opens, in
  # the daemon's order (tests/documents.qml), through the real
  # ReadyRows.qml and Daemon.qml, with Google's account, the clock, the
  # mail, the saves and the entry its links open through stood in.
  a-shut-launcher-makes-no-document-rows =
    let
      # The entry every Google link opens through, as google's home.nix
      # gives it, shown here so the cases can wait for Quickshell to list it.
      opens = pkgs.writeText "google-opens.desktop" ''
        [Desktop Entry]
        Type=Application
        Name=Google
        Exec=google-opens %u
      '';
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
      ${copied [
        "Daemon.qml"
        "FileFacts.js"
        "Words.qml"
        "launcher/Provider.qml"
        "launcher/ReadyRows.qml"
        "launcher/words.json"
      ]}
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
      mkdir -p share/applications
      cp ${opens} share/applications/google-opens.desktop
      export HOME=$TMPDIR XDG_RUNTIME_DIR=$TMPDIR/runtime LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen
      export XDG_DATA_HOME=$TMPDIR/data XDG_DATA_DIRS=$PWD/share
      export TZ=UTC FONTCONFIG_FILE=${noFonts}
      mkdir -m 700 $XDG_RUNTIME_DIR
      ${shellFolderLint}
      timeout 60 ${qs} -p shell.qml
      touch $out
    '';
}
