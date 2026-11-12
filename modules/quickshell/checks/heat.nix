# The checks of the launcher's Heat, which the folder's checks.nix gathers with
# every other place's; its opening comment says why each place keeps its own.
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
  # The launcher's Heat draws its rows and headline as the machine daemon
  # serves them ready, and makes again only the rows a change names
  # (tests/heat.qml), over a stand-in for the daemon, with niri stood in.
  heat-remakes-only-what-moved =
    # Kept in Nix because each line copies a file Nix gives it beside the
    # cases, or sets what Quickshell runs them with.
    pkgs.runCommandLocal "heat-remakes-only-what-moved" { } ''
      mkdir -p launcher runtime/daemons && chmod -R 700 runtime
      cp ${../../../tests/lib/Expect.js} Expect.js
      cp ${../../../tests/lib/Watch.js} Watch.js
      ${copied [
        "Daemon.qml"
        "Words.qml"
        "launcher/Heat.qml"
        "launcher/Provider.qml"
        "launcher/ReadyRows.qml"
        "launcher/words.json"
      ]}
      cp ${(account graphical).xdg.configFile."quickshell/Theme.qml".source} Theme.qml
      cp ${singleton "Niri.qml" ''
        function spawn(command) { }
      ''} Niri.qml
      cp ${../../../tests/heat.qml} shell.qml
      export HOME=$TMPDIR XDG_RUNTIME_DIR=$TMPDIR/runtime LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      timeout 60 ${qs} -p shell.qml
      touch $out
    '';

  # With nothing at the machine daemon's socket, Heat's row stands: held
  # before its watch begins, then saying its daemon isn't answering, so
  # Machine's head never names a row a show can't find (tests/heat-away.qml).
  heat-stands-while-its-daemon-is-away =
    # Kept in Nix because each line copies a file Nix gives it beside the
    # case, or sets what Quickshell runs it with.
    pkgs.runCommandLocal "heat-stands-while-its-daemon-is-away" { } ''
      mkdir -p launcher runtime/daemons && chmod -R 700 runtime
      cp ${../../../tests/lib/Expect.js} Expect.js
      ${copied [
        "Daemon.qml"
        "Words.qml"
        "launcher/Heat.qml"
        "launcher/Provider.qml"
        "launcher/ReadyRows.qml"
        "launcher/words.json"
      ]}
      cp ${(account graphical).xdg.configFile."quickshell/Theme.qml".source} Theme.qml
      cp ${singleton "Niri.qml" ''
        function spawn(command) { }
      ''} Niri.qml
      cp ${../../../tests/heat-away.qml} shell.qml
      export HOME=$TMPDIR XDG_RUNTIME_DIR=$TMPDIR/runtime LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      timeout 60 ${qs} -p shell.qml
      touch $out
    '';
}
