# The checks of the margin's pieces and marks at the top of the screen, which
# the folder's checks.nix gathers with every other place's; its opening comment
# says why each place keeps its own.
{
  account,
  bareNixos,
  copied,
  holds,
  launcherOnNiri,
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
          "homeNetwork"
          "workNetwork"
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
      ${copied [ "pieces/Drawers.js" ]}
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml piece-drawers.qml
      touch $out
    '';

  # Now's Home Network piece says what ails the home network, worst first,
  # and nothing while eero is out of reach, as tests/home-network-ails.qml
  # holds pieces/HomeNetwork.js to records shaped as the eero daemon keeps.
  now-says-what-ails-the-home-network =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "now-says-what-ails-the-home-network" { } ''
      cp ${../../../tests/home-network-ails.qml} home-network-ails.qml
      cp ${../../../tests/lib/Expect.js} Expect.js
      ${copied [ "pieces/HomeNetwork.js" ]}
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml home-network-ails.qml
      touch $out
    '';

  # Now's Work Network piece says what ails the link to work, and nothing
  # while Joel took it down himself or it's only coming up, as
  # tests/work-network-ails.qml holds pieces/WorkNetwork.js to the states
  # the warp daemon tells.
  now-says-what-ails-the-work-network =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "now-says-what-ails-the-work-network" { } ''
      cp ${../../../tests/work-network-ails.qml} work-network-ails.qml
      cp ${../../../tests/lib/Expect.js} Expect.js
      ${copied [ "pieces/WorkNetwork.js" ]}
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml work-network-ails.qml
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
      ${copied [ "Marks.js" ]}
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
      ${copied [ "Devices.js" ]}
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
      ${copied [ "Facts.js" ]}
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml facts.qml
      touch $out
    '';

  # The margin's cat (modules/margin-cat), which the graphical account's
  # shell carries, sits over the column the keys are in on a real niri and
  # follows them, on a layer taking no keys, and goes in fullscreen
  # (tests/margin-cat-on-niri-test.py). Where over a column it sits is held
  # in Qt's engine, beside its module. Its shell draws through Qt's GPU
  # path, on Mesa's software GL as niri here does, rather than Qt's software
  # renderer: the laptop's shell draws on its GPU, and the software renderer
  # widens a cell of 1.333 logical pixels to three screen pixels at 1.5,
  # which a GPU, filling the pixels whose middles it covers, never does.
  the-margin-cat-follows-the-keys-on-niri = launcherOnNiri {
    name = "the-margin-cat-follows-the-keys-on-niri";
    tests = ../../../tests/margin-cat-on-niri-test.py;
    # Its doze in 45 s rather than the laptop's ten minutes, so a case
    # sees it fall asleep; the tests wake it with a key before each case.
    shell.cat.dozes = 45;
    given = [
      "QT_QUICK_BACKEND=rhi"
      "QSG_RHI_BACKEND=opengl"
    ];
  };
}
