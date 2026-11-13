# The checks of On the Selection, the place on Super+X, which the folder's
# checks.nix gathers with every other place's; its opening comment says why
# each place keeps its own.
{
  copied,
  noFonts,
  pkgs,
  shellFolderLint,
  ...
}:
{
  # The selected text heads the place on one line, given way at a whole
  # word (VKCF3Z, e684d7ae), an answer is named by its first sentence, and
  # pi-verbs' actions keep the order it served them in, as
  # tests/on-selection.qml holds launcher/OnSelection.js to it in Qt's own
  # engine.
  the-launcher-heads-the-selection-at-a-whole-word =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "the-launcher-heads-the-selection-at-a-whole-word" { } ''
      cp ${../../../tests/on-selection.qml} on-selection.qml
      cp ${../../../tests/lib/Expect.js} Expect.js
      ${copied [ "launcher/OnSelection.js" ]}
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml on-selection.qml
      touch $out
    '';
}
