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
  # Each of Chrome's open pages the programs daemon serves is a row of the
  # launcher's, named by its title and found by its site in the daemon's
  # order, and its Return goes to the Chrome window it's in front of, else
  # Chrome's newest (tests/tabs.qml), through the real ReadyRows.qml and
  # Daemon.qml, with niri stood in.
  a-chrome-tab-is-found-by-its-site-and-goes-to-its-window =
    let
      # Kept in Nix because it only stands in for the niri windows a tab's
      # Return looks among, keeping what it was asked to do.
      niri = singleton "Niri.qml" ''
        property var windows: ({})
        property var byApp: ({ "google-chrome": { id: 9 } })
        property var acted: []
        function act(action) { acted = acted.concat([action]); }
        function spawn(command) {}
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
      export HOME=$TMPDIR XDG_RUNTIME_DIR=$TMPDIR/runtime LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen
      export TZ=UTC FONTCONFIG_FILE=${noFonts}
      mkdir -m 700 $XDG_RUNTIME_DIR
      ${shellFolderLint}
      timeout 60 ${qs} -p shell.qml
      touch $out
    '';
}
