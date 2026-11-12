# The checks of the launcher's Copies, the clipboard's history, which the
# folder's checks.nix gathers with every other place's; its opening comment says
# why each place keeps its own.
{
  copied,
  launcherOnNiri,
  lib,
  noFonts,
  passesClippy,
  pkgs,
  shellFolderLint,
  ...
}:
{
  # Super+V's paste lands in the terminal under within 50 ms of Return
  # though the shell is held stopped once it has asked, a stand-in for its
  # thread busy after an opening: the clipboard's history serves the copy
  # and presses the paste keys (tests/pastes-on-niri-test.py).
  a-paste-lands-while-the-shell-is-busy-on-niri = launcherOnNiri {
    name = "a-paste-lands-while-the-shell-is-busy-on-niri";
    tests = ../../../tests/pastes-on-niri-test.py;
    given = [
      "CLIPBOARD=${lib.getExe (pkgs.callPackage ../clipboard/package.nix { })}"
      "CLIPBOARD_WORDS=${(import ../launcher/words.nix).file}"
    ];
  };

  # Copies and Emoji say the clipboard's history isn't answering only once
  # it's been silent a second, so a restart never blinks their counts, and
  # a paste that never landed for a minute, as tests/pasting.qml holds
  # Pasting.js to it in Qt's own engine.
  the-clipboard-says-once-it-is-silent =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "the-clipboard-says-once-it-is-silent" { } ''
      cp ${../../../tests/pasting.qml} pasting.qml
      ${copied [ "Pasting.js" ]}
      cp ${../../../tests/lib/Expect.js} Expect.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml pasting.qml
      touch $out
    '';

  # clippy finds nothing in the clipboard's history, its tests included.
  clipboard-history-passes-clippy = passesClippy "clipboard-history-passes-clippy" (
    pkgs.callPackage ../clipboard/package.nix { }
  );
}
