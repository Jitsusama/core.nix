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
  # it's drawn, and its ways about the windows, as tests/margin-cat.qml holds
  # them, its ways on and off floating windows, as
  # tests/margin-cat-floats.qml holds them, the rules its journeys over
  # generated layouts keep, as tests/margin-cat-journeys.qml holds them, and
  # how it reads niri's layout, as tests/margin-cat-tiles.qml holds it, in
  # Qt's own engine.
  the-margin-cat-sits-above-its-column =
    let
      qt = pkgs.qt6.qtdeclarative;
      sources = pkgs.lib.fileset.toSource {
        root = ./.;
        fileset = pkgs.lib.fileset.unions [
          ./MarginCat.js
          ./MarginCatFalls.js
          ./MarginCatFloats.js
          ./MarginCatPaths.js
          ./MarginCatPlays.js
          ./MarginCatRoutes.js
          ./MarginCatTiles.js
          ./MarginCatWays.js
          ./tests/margin-cat.qml
          ./tests/margin-cat-floats.qml
          ./tests/margin-cat-journeys.qml
          ./tests/margin-cat-tiles.qml
        ];
      };
    in
    pkgs.runCommandLocal "the-margin-cat-sits-above-its-column" { } ''
      cp ${sources}/*.js ${sources}/tests/*.qml .
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${pkgs.makeFontsConf { fontDirectories = [ ]; }}
      ${shellFolderLint}
      for case in *.qml; do
        QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml "$case"
      done
      touch $out
    '';
}
