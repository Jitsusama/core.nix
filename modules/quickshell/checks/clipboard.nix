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
  # Super+V's paste lands in the terminal under within 50 ms of Return though the
  # shell is held stopped once it has asked, a stand-in for its thread busy after
  # an opening: the clipboard's history serves the copy and presses the paste
  # keys; and it reads the text highlighted as a get asks for it, never keeping
  # it; and it keeps no copy a password manager marks as secret, but the ordinary
  # one after it (modules/quickshell/tests/pastes-on-niri-test.py).
  a-paste-lands-while-the-shell-is-busy-on-niri = launcherOnNiri {
    name = "a-paste-lands-while-the-shell-is-busy-on-niri";
    tests = ../tests/pastes-on-niri-test.py;
    given = [
      "CLIPBOARD=${lib.getExe (pkgs.callPackage ../clipboard/package.nix { })}"
      "CLIPBOARD_WORDS=${(import ../launcher/words.nix).file}"
    ];
  };

  # Copies and Emoji say the clipboard's history isn't answering only once it's
  # been silent a second, so a restart never blinks their counts, and a paste
  # that never landed for a minute, as modules/quickshell/tests/pasting.qml
  # holds Pasting.js to it in Qt's own engine.
  the-clipboard-says-once-it-is-silent =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "the-clipboard-says-once-it-is-silent" { } ''
      cp ${../tests/pasting.qml} pasting.qml
      ${copied [ "Pasting.js" ]}
      cp ${../../../tests/lib/Expect.js} Expect.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml pasting.qml
      touch $out
    '';

  # Copies hide what each copy holds while niri says any cast, a call's
  # share through the portal or a recorder's screencopy, and until niri has
  # said its casts, but not for the shell's own recording of one window, and
  # what a hidden copy's row is made of says only its kind and when it came,
  # and a row made showing names what hides it, as
  # modules/quickshell/tests/copies-hidden.qml holds launcher/Shared.js
  # to it in Qt's own engine.
  copies-hide-while-the-screen-is-shared =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "copies-hide-while-the-screen-is-shared" { } ''
      cp ${../tests/copies-hidden.qml} copies-hidden.qml
      cp ${../launcher/Shared.js} Shared.js
      cp ${../../../tests/lib/Expect.js} Expect.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml copies-hidden.qml
      touch $out
    '';

  # clippy finds nothing in the clipboard's history, its tests included.
  clipboard-history-passes-clippy = passesClippy "clipboard-history-passes-clippy" (
    pkgs.callPackage ../clipboard/package.nix { }
  );
}
