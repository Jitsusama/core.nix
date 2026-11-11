# The checks of the launcher's Power, and the charge notices, which the folder's
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
  # The launcher's Power draws its rows and headline as the machine daemon
  # serves them ready, makes again only the rows a change names, and asks
  # a charge limit on the rows' own socket (tests/power.qml), over a
  # stand-in for the daemon, with niri stood in.
  power-remakes-only-what-moved =
    # Kept in Nix because each line copies a file Nix gives it beside the
    # cases, or sets what Quickshell runs them with.
    pkgs.runCommandLocal "power-remakes-only-what-moved" { } ''
      mkdir -p launcher runtime/daemons && chmod -R 700 runtime
      cp ${../../../tests/lib/Expect.js} Expect.js
      cp ${../../../tests/lib/Watch.js} Watch.js
      cp ${../Daemon.qml} Daemon.qml
      cp ${../Hearing.js} Hearing.js
      cp ${../Fuzzy.js} Fuzzy.js
      cp ${../ProgramGlyphs.js} ProgramGlyphs.js
      cp ${../When.js} When.js
      cp ${../launcher/Provider.qml} launcher/Provider.qml
      cp ${../launcher/Power.qml} launcher/Power.qml
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
      cp ${../../../tests/power.qml} shell.qml
      export HOME=$TMPDIR XDG_RUNTIME_DIR=$TMPDIR/runtime LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      timeout 60 ${qs} -p shell.qml
      touch $out
    '';

  # A battery that reports only its charge, as a Bluetooth device's does,
  # is low and then critical once each, as tests/crossing.qml holds.
  a-low-charge-is-said-once =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "a-low-charge-is-said-once" { } ''
      cp ${../../../tests/crossing.qml} crossing.qml
      cp ${../Crossing.js} Crossing.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml crossing.qml
      touch $out
    '';

  # Each Bluetooth device running low is a thing of its own, by its own
  # charge, name and kind, as tests/devices.qml holds Devices.js to with
  # plain objects standing in for BlueZ's devices, since the desktop VM has
  # no Bluetooth.
  a-low-device-is-its-own-notice =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "a-low-device-is-its-own-notice" { } ''
      cp ${../../../tests/devices.qml} devices.qml
      cp ${../Devices.js} Devices.js
      cp ${../Crossing.js} Crossing.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml devices.qml
      touch $out
    '';
}
