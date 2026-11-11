# The checks of the margin's pieces and marks at the top of the screen, which
# the folder's checks.nix gathers with every other place's; its opening comment
# says why each place keeps its own.
{
  account,
  bareNixos,
  holds,
  lib,
  noFonts,
  pkgs,
  self,
  shellFolderLint,
  ...
}:
{
  # A machine shapes what the overview shows in a line a change: a piece
  # moved, cut, shown only while true, a threshold changed and the rest kept,
  # and a piece of its own linked beside the built-in ones.
  the-margin-takes-its-pieces =
    let
      own = pkgs.writeText "wide.qml" "Piece { value: settings.text }";
      machine = bareNixos [
        self.nixosModules.workstation
        self.nixosModules.graphical
        {
          home-manager.users.someone.jitsusama.quickshell.margin.pieces = {
            date.tier = null;
            network.order = 25;
            battery.low = 15;
            volume.tier = "when";
            wide = {
              order = 60;
              source = own;
              label = "wide";
              text = "a piece of the machine's own";
              collapse = 5;
              mark = "on";
            };
          };
        }
      ];
      home = account machine;
      rendered = lib.findFirst (lib.hasInfix "property var inOrder: ") "" (
        lib.splitString "\n" home.xdg.configFile."quickshell/Pieces.qml".text
      );
      drawn = builtins.fromJSON (lib.last (lib.splitString "property var inOrder: " rendered));
      piece = lib.listToAttrs (map (piece: lib.nameValuePair piece.name piece) drawn);
    in
    holds "the-margin-takes-its-pieces"
      "The margin didn't draw a machine's own pieces as it set them: their order, their cuts, their settings or its own file."
      (
        map (piece: piece.name) drawn == [
          "time"
          "network"
          "battery"
          "devices"
          "volume"
          "tokens"
          "spend"
          "pi"
          "notices"
          "forYou"
          "google"
          "meeting"
          "quiet"
          "disk"
          "lag"
          "awake"
          "wide"
          "mail"
          "profile"
          "clearing"
          "microphone"
          "capslock"
          "layer"
          "call"
          "meet"
          "sharing"
          "camera"
          "recording"
          "timers"
          "playing"
        ]
        && piece.battery.low == 15
        && piece.battery.critical == 10
        && piece.volume.tier == "when"
        && piece.wide.mark == "on"
        && piece.wide.file == "wide.qml"
        && "${home.xdg.configFile."quickshell/pieces/wide.qml".source}" == "${own}"
      );

  # Now's own pieces say in their drawers all their source holds, in the
  # order every drawer keeps, as tests/piece-drawers.qml holds
  # pieces/Drawers.js to what the disk guards, Notices, lag-watch, niri,
  # the recorder and Quiet say.
  a-piece-says-all-it-knows =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "a-piece-says-all-it-knows" { } ''
      mkdir pieces
      cp ${../../../tests/piece-drawers.qml} piece-drawers.qml
      cp ${../pieces/Drawers.js} pieces/Drawers.js
      cp ${../FileFacts.js} FileFacts.js
      cp ${../When.js} When.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml piece-drawers.qml
      touch $out
    '';

  # Every glyph in the margin stands the same height on the gap's middle
  # line, however tall its font draws it, and they line up from the corner
  # by how much each asks, in the colour that says so, and For You's tray
  # is lit by what For You holds, as tests/marks.qml holds them.
  the-margins-marks-stand-level =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "the-margins-marks-stand-level" { } ''
      cp ${../../../tests/marks.qml} marks.qml
      cp ${../Marks.js} Marks.js
      cp ${../pieces/Holding.js} Holding.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml marks.qml
      touch $out
    '';

  # Every mark the margin can show, each kind a device running low is drawn
  # as among them, is the theme's drawing of it, standing the grid's full
  # height and centred across it as Qt draws it, so a row of them is level
  # and each has the same room either side (tests/drawings.qml).
  the-margins-marks-are-drawn-level =
    let
      qt = pkgs.qt6.qtdeclarative;
      # With Slack too, whose piece only a machine that brings Slack has.
      home = account (bareNixos [
        self.nixosModules.graphical
        { home-manager.sharedModules = [ self.homeModules.slack ]; }
      ]);
      marks = lib.unique (
        lib.filter (mark: mark != null) (
          lib.mapAttrsToList (_: piece: piece.mark) home.jitsusama.quickshell.margin.pieces
        )
      );
      drawn = lib.mapAttrs (_: mark: mark.drawn) home.jitsusama.theme.marks;
      drawings = pkgs.writeText "Drawings.js" ''
        .pragma library
        var marks = ${builtins.toJSON marks};
        var drawn = ${builtins.toJSON drawn};
      '';
    in
    pkgs.runCommandLocal "the-margins-marks-are-drawn-level" { } ''
      cp ${../../../tests/drawings.qml} drawings.qml
      cp ${drawings} Drawings.js
      cp ${../Devices.js} Devices.js
      cp ${../Crossing.js} Crossing.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml drawings.qml
      touch $out
    '';

  # A piece standing for several things is a fact for each, known by its
  # own mark, as tests/facts.qml holds Facts.js to.
  a-piece-may-stand-for-several =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "a-piece-may-stand-for-several" { } ''
      cp ${../../../tests/facts.qml} facts.qml
      cp ${../Facts.js} Facts.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml facts.qml
      touch $out
    '';
}
