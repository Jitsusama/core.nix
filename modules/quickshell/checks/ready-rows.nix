# The checks of what the shell makes of the rows a daemon serves ready:
# placing them by the order each says, and wording each one's time, side,
# note, drawer and acts, in Qt's engine. They change together when the
# ready-row form does (attachments' ready-rows.md), apart from any place,
# which the folder's checks.nix gathers with every other place's.
{
  copied,
  noFonts,
  pkgs,
  shellFolderLint,
  ...
}:
{
  # The launcher keeps the rows a daemon serves ready in the order each
  # row says, placing each a change names and letting those gone go, as
  # tests/placed.qml holds launcher/Placed.js to rows made up there.
  the-launcher-places-each-row-it-is-served =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "the-launcher-places-each-row-it-is-served" { } ''
      mkdir launcher
      cp ${../../../tests/placed.qml} placed.qml
      ${copied [ "launcher/Placed.js" ]}
      cp ${../../../tests/lib/Expect.js} Expect.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml placed.qml
      touch $out
    '';

  # The shell takes a place's rows as its daemon serves them ready, placing
  # only its rows by their order and wording every time form, side, note
  # and drawer, as tests/ready-rows.qml holds launcher/ReadyRows.js and
  # launcher/Found.js to records made up there.
  the-launcher-words-each-ready-row-it-is-served =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    # Kept in Nix because each line copies a file Nix gives it beside the
    # cases, or sets what Qt's engine runs them with.
    pkgs.runCommandLocal "the-launcher-words-each-ready-row-it-is-served" { } ''
      mkdir launcher
      cp ${../../../tests/ready-rows.qml} ready-rows.qml
      ${copied [
        "launcher/Beneath.js"
        "launcher/Found.js"
        "launcher/Moved.js"
        "launcher/ReadyRows.js"
      ]}
      cp ${../../../tests/lib/Expect.js} Expect.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml ready-rows.qml
      touch $out
    '';
}
