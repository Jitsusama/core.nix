# The checks of the launcher's Chrome tabs, found from home, which the
# folder's checks.nix gathers with every other place's; its opening comment
# says why each place keeps its own.
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
  # Chrome's open pages the programs daemon serves are the launcher's Tabs
  # collection, each named by its title and found by its site, ranked as the
  # daemon answers what's typed, and its Return goes to the Chrome window
  # it's in front of, else Chrome's newest (tests/tabs.qml), through the real
  # ReadyRows.qml and Daemon.qml, with niri stood in. Each bookmark served is
  # a row of the main search, found the same way, and its Return runs Chrome
  # with its address.
  a-chrome-tab-is-found-by-its-site-and-goes-to-its-window =
    let
      # Chrome's entry, which a bookmark opens through, shown here so the
      # cases can wait for Quickshell to list it.
      chrome = pkgs.writeText "google-chrome.desktop" ''
        [Desktop Entry]
        Type=Application
        Name=Google Chrome
        Exec=google-chrome-stable %U
      '';
      # Kept in Nix because it only stands in for the niri windows a tab's
      # Return looks among, keeping what it was asked to do.
      niri = singleton "Niri.qml" ''
        property var windows: ({})
        property var byApp: ({ "google-chrome": { id: 9 } })
        property var acted: []
        property var spawned: []
        function act(action) { acted = acted.concat([action]); }
        function spawn(command) { spawned = spawned.concat([command]); }
      '';
    in
    # Kept in Nix because each line copies a file Nix gives it beside the
    # cases, or sets what Quickshell runs them with.
    pkgs.runCommandLocal "a-chrome-tab-is-found-by-its-site-and-goes-to-its-window" { } ''
      mkdir -p launcher
      cp ${../../../tests/lib/Expect.js} Expect.js
      cp ${(account graphical).xdg.configFile."quickshell/Theme.qml".source} Theme.qml
      ${copied [
        "Daemon.qml"
        "Words.qml"
        "launcher/Provider.qml"
        "launcher/ReadyRows.qml"
        "launcher/words.json"
      ]}
      cp ${../../chrome/Tabs.qml} launcher/Tabs.qml
      cp ${niri} Niri.qml
      cp ${../../../tests/tabs.qml} shell.qml
      mkdir -p share/applications
      cp ${chrome} share/applications/google-chrome.desktop
      export HOME=$TMPDIR XDG_RUNTIME_DIR=$TMPDIR/runtime LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen
      export XDG_DATA_HOME=$TMPDIR/data XDG_DATA_DIRS=$PWD/share
      export TZ=UTC FONTCONFIG_FILE=${noFonts}
      mkdir -m 700 $XDG_RUNTIME_DIR
      ${shellFolderLint}
      timeout 60 ${qs} -p shell.qml
      touch $out
    '';
}
