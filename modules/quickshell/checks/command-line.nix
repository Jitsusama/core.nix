# The checks of the launcher's command line and query field, and the keys they
# take as Vim's, which the folder's checks.nix gathers with every other place's;
# its opening comment says why each place keeps its own.
{
  copied,
  noFonts,
  pkgs,
  shellFolderLint,
  ...
}:
{
  # The region picker's keys start, move and size its box, and letters name
  # the grid's cells, as tests/picking.qml says, in Qt's own engine.
  the-picker-picks-what-the-keys-say =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "the-picker-picks-what-the-keys-say" { } ''
      cp ${../../../tests/picking.qml} picking.qml
      ${copied [ "Picking.js" ]}
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml ${qt}/bin/qml picking.qml
      touch $out
    '';

  # Log Out, Reboot and Power Off arm on a first Return and end on a second
  # with no key between, as tests/arming.qml holds Arming.js to.
  an-ending-row-arms-before-it-ends =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "an-ending-row-arms-before-it-ends" { } ''
      cp ${../../../tests/arming.qml} arming.qml
      ${copied [ "Arming.js" ]}
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml arming.qml
      touch $out
    '';

  # Vim's command line is read as Vim reads it, as tests/ex.qml holds Ex.js
  # to: ranges, names by their short forms, a bang, :g and :v, the rows a
  # range names, and Vim's own errors.
  ex-reads-vims-command-lines =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "ex-reads-vims-command-lines" { } ''
      cp ${../../../tests/ex.qml} ex.qml
      ${copied [ "Ex.js" ]}
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml ex.qml
      touch $out
    '';

  # The launcher's command line lists every command it can take, the cursor
  # row's acts first, narrows as it's typed, chooses as his cmp does and
  # runs exactly what's typed, with Vim's ranges and :g, and reads :set as
  # Vim does, as tests/command-line.qml holds Commands.js and SetWords.js
  # to, with the Ex.js they read ranges by and CommandBinding.js, which
  # binds a line to things and asks by d's rule, in Operators.js.
  the-command-line-lists-every-command =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "the-command-line-lists-every-command" { } ''
      cp ${../../../tests/command-line.qml} command-line.qml
      ${copied [
        "CommandBinding.js"
        "SetWords.js"
      ]}
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml command-line.qml
      touch $out
    '';

  # The launcher's query line takes vi's two modes from one table, as
  # tests/query-keys.qml holds QueryKeys.js and Editing.js to: insert's
  # keys, normal's ways back into insert, its moves through the rows with counts and
  # where each lands, its closing keys and ex line, its place used last,
  # sequences, and where the caret goes, which Typing.js says; ListedKeys.js
  # to the keys Space ? lists; the drawer as a window, Space and the arrows
  # into it and back, with DrawerPage.js to where its keys take its cursor
  # and view, DrawerFigure.js to where a fact's figure stands, DrawerProse.js to where
  # its prose is cut and DrawerValue.js to how its value is laid on lines; Operators.js to what d and y do to the rows they take; and
  # Confirm.js to how the launcher asks once, answered by Return as an armed row is.
  the-query-field-takes-vi-modes =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "the-query-field-takes-vi-modes" { } ''
      cp ${../../../tests/query-keys.qml} query-keys.qml
      ${copied [
        "Confirm.js"
        "DrawerFigure.js"
        "DrawerPage.js"
        "DrawerProse.js"
        "DrawerValue.js"
        "Editing.js"
        "KeyNames.js"
        "ListedKeys.js"
        "ModeMarks.js"
      ]}
      cp ${(import ../launcher/words.nix).file} words.json
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts} QML_XHR_ALLOW_FILE_READ=1
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml query-keys.qml
      touch $out
    '';
}
