# The checks of the margin's cat, which checks.nix gathers with every other
# folder's; its opening comment says what each kind of check proves.
{
  onLinux,
  pkgs,
  shellFolderLint,
  ...
}:
# The cat lives in the shell, only on Linux, so a Mac doesn't evaluate
# these.
onLinux {
  # Where the cat sits, over its column and off the margin's marks, how big
  # it's drawn, how it walks and plays, as tests/margin-cat.qml holds them,
  # in Qt's own engine.
  the-margin-cat-sits-above-its-column =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "the-margin-cat-sits-above-its-column" { } ''
      cp ${./tests/margin-cat.qml} margin-cat.qml
      cp ${./MarginCat.js} MarginCat.js
      cp ${./Play.js} Play.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${pkgs.makeFontsConf { fontDirectories = [ ]; }}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml margin-cat.qml
      touch $out
    '';
}
