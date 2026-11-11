# The checks of the launcher's Heat, which the folder's checks.nix gathers with
# every other place's; its opening comment says why each place keeps its own.
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
      cp ${../Daemon.qml} Daemon.qml
      cp ${../Hearing.js} Hearing.js
      cp ${../Fuzzy.js} Fuzzy.js
      cp ${../ProgramGlyphs.js} ProgramGlyphs.js
      cp ${../When.js} When.js
      cp ${../launcher/Provider.qml} launcher/Provider.qml
      cp ${../launcher/Heat.qml} launcher/Heat.qml
      cp ${../launcher/ReadyRows.qml} launcher/ReadyRows.qml
      cp ${../launcher/ReadyRows.js} launcher/ReadyRows.js
      cp ${../launcher/Ago.js} launcher/Ago.js
      cp ${../launcher/Interfaces.js} launcher/Interfaces.js
      cp ${../launcher/Asking.js} launcher/Asking.js
      cp ${../launcher/Numbers.js} launcher/Numbers.js
      cp ${../launcher/Seen.js} launcher/Seen.js
      cp ${(account graphical).xdg.configFile."quickshell/Theme.qml".source} Theme.qml
      cp ${../Words.qml} Words.qml
      cp ${../launcher/words.json} launcher/words.json
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
}
