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
  # generated layouts keep, as tests/margin-cat-journeys.qml holds them, how
  # it rides and falls as the windows under it change, as
  # tests/margin-cat-rides.qml holds it, its way down a workspace as one
  # screen, as tests/margin-cat-one-screen.qml holds it, how it reads
  # niri's layout, as tests/margin-cat-tiles.qml holds it, and the plays
  # that move it, as tests/margin-cat-games.qml holds them, in Qt's own
  # engine.
  the-margin-cat-sits-above-its-column =
    let
      qt = pkgs.qt6.qtdeclarative;
      sources = pkgs.lib.fileset.toSource {
        root = ./.;
        fileset = pkgs.lib.fileset.unions [
          ./MarginCat.js
          ./MarginCatFalls.js
          ./MarginCatFloats.js
          ./MarginCatGames.js
          ./MarginCatPaths.js
          ./MarginCatPlays.js
          ./MarginCatRoutes.js
          ./MarginCatStacked.js
          ./MarginCatTiles.js
          ./MarginCatWays.js
          ./tests/margin-cat.qml
          ./tests/margin-cat-floats.qml
          ./tests/margin-cat-games.qml
          ./tests/margin-cat-journeys.qml
          ./tests/margin-cat-one-screen.qml
          ./tests/margin-cat-pets.qml
          ./tests/margin-cat-rides.qml
          ./tests/margin-cat-tiles.qml
        ];
      };
      # The theme's drawings of the yarn, which margin-cat-games.qml draws
      # as the screen does, and of the cat sitting and leaning, whose cells
      # margin-cat-pets.qml holds a lean either way to.
      glyphs = import ../theme/glyphs.nix { inherit (pkgs) lib; };
      drawings = pkgs.writeText "MarginCatDrawings.js" ''
        .pragma library
        // The yarn's drawings and the lean's, from modules/theme/glyphs.nix.
        var pixels = ${
          builtins.toJSON (
            pkgs.lib.genAttrs [ "yarn" "yarnRollingA" "yarnRollingB" ] (name: glyphs.${name}.pixels)
          )
        };
        var poses = ${
          builtins.toJSON (pkgs.lib.genAttrs [ "leaning" "nothingHere" ] (name: glyphs.${name}.pixels))
        };
      '';
    in
    pkgs.runCommandLocal "the-margin-cat-sits-above-its-column" { } ''
      cp ${sources}/*.js ${sources}/tests/*.qml .
      cp ${drawings} MarginCatDrawings.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${pkgs.makeFontsConf { fontDirectories = [ ]; }}
      ${shellFolderLint}
      for case in *.qml; do
        QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml "$case"
      done
      touch $out
    '';

  # Every design Joel stated for the cat, under the guide's "Joel's Words",
  # is held by a phrase a case says, or owed with the date it was asked for,
  # so a ruling that would drop one fails a named case rather than going
  # unseen.
  the-cat-keeps-joels-words = pkgs.runCommandLocal "the-cat-keeps-joels-words" { } ''
    ${pkgs.lib.getExe pkgs.python3} ${./tests/keeps-joels-words.py} ${./AGENTS.md} ${./tests}
    touch $out
  '';
}
