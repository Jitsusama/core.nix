# The checks of how the shell looks: its type, space, shape and cards, held to
# the theme, which the folder's checks.nix gathers with every other place's; its
# opening comment says why each place keeps its own.
{
  account,
  copied,
  everyShellFile,
  graphical,
  holds,
  lib,
  noFonts,
  pkgs,
  qs,
  shellFolderLint,
  ...
}:
{
  # Every run of text the shell sets names its type's level, never a step, a
  # weight or a spacing of its own (type-is-set-by-level.py). So the check
  # can't pass by seeing nothing, each way of straying is shown to fail
  # first: a step, a level mixed with a step, a line carried on and a
  # script's assignment. The reader's page and its foot, Typst
  # held in Rust, name a level too, and a size, a weight, a tracking, a
  # strong's delta or an axis said by number fails, as a size named with one
  # does, while a comment saying one doesn't.
  type-is-set-by-level =
    let
      check = "${lib.getExe pkgs.python3} ${../type-is-set-by-level.py}";
      shells = lib.fileset.toSource {
        root = ../../.;
        fileset = lib.fileset.fileFilter (file: file.hasExt "qml") ../../.;
      };
      pages = [
        ../../reader/src/page.rs
        ../../reader/src/prompt.rs
      ];
    in
    pkgs.runCommandLocal "type-is-set-by-level" { } ''
      # A file that sets its type as given, and what the check must say of it.
      strays() {
        rm -rf modules
        mkdir -p modules/shell
        printf '%s\n' "$1" > modules/shell/Surface.qml
        if ${check} modules > said; then
          echo "The type check passed on: $1"
          exit 1
        fi
        if ! grep -qF -- "$2" said; then
          echo "Of $1 the type check didn't say: $2"
          cat said
          exit 1
        fi
      }
      # A page's template as given, and what the check must say of it.
      sets() {
        rm -rf modules
        mkdir -p modules
        printf '%s\n' "$1" > page.rs
        if ${check} modules page.rs > said; then
          echo "The type check passed on the page: $1"
          exit 1
        fi
        if ! grep -qF -- "$2" said; then
          echo "Of the page $1 the type check didn't say: $2"
          cat said
          exit 1
        fi
      }
      strays 'font.pointSize: Theme.mono.large' 'Surface.qml:1: font.pointSize'
      strays 'font.pointSize: row.big ? Theme.level.title.pointSize : Theme.mono.large' \
        'font.pointSize is row.big'
      strays $'font.weight: row.selected\n    ? Font.Medium : Theme.level.name.weight' \
        'font.weight is row.selected ? Font.Medium'
      strays 'label.font.letterSpacing = 2' 'font.letterSpacing is 2'
      sets '#set text(..level.words.text, size: 0.93 * body-size)' 'page.rs:1: size is 0.93'
      sets '#show heading: set text(weight: 540, tracking: -0.02em)' 'weight is 540'
      sets '#show heading: set text(weight: 540, tracking: -0.02em)' 'tracking is -0.02em'
      sets '#set strong(delta: 100)' 'delta is 100'
      sets '#let display-axes = (opsz: 144, SOFT: 100, WONK: 1)' 'opsz is 144'
      sets '#let code-size = 10.5pt' 'code-size is 10.5pt'
      sets '#let heading-weight = 540' 'heading-weight is 540'
      sets $'#[cfg(test)]\nconst ROWS: u8 = 3;\n#let lede-size = 12pt' 'lede-size is 12pt'
      # A level, read from the theme or a property holding one, passes, as
      # a comment saying a size and the test module after the page's own
      # lines do.
      mkdir -p modules/shell
      printf '%s\n' 'font.pointSize: Theme.level.note.pointSize * set.scale' \
        'font.weight: chord.level.weight' 'font: other.font' > modules/shell/Surface.qml
      printf '%s\n' '#set text(..level.note.text, fill: ink.on_accent)' \
        '/// The words at 13.69 pt, weight: 380.' '// size: 12pt, as a note says' \
        '#[cfg(test)]' 'mod tests {' 'size: 11.5pt' > page.rs
      ${check} modules page.rs
      ${check} ${shells} ${lib.concatStringsSep " " pages}
      touch $out
    '';

  # Every space the shell sets names the rung of the ladder it sits between,
  # the beat or a level's pitch, never a step of the size scale
  # (space-is-set-by-rung.py). So it can't pass by seeing nothing, a step and
  # a step summed with a rung are each shown to fail first, while a rung, the
  # beat, a pitch and a comment naming a step pass.
  space-is-set-by-rung =
    let
      check = "${lib.getExe pkgs.python3} ${../space-is-set-by-rung.py}";
      shells = lib.fileset.toSource {
        root = ../../.;
        fileset = lib.fileset.fileFilter (file: file.hasExt "qml") ../../.;
      };
    in
    pkgs.runCommandLocal "space-is-set-by-rung" { } ''
      # A file that sets its space as given, and what the check must say of it.
      strays() {
        rm -rf modules
        mkdir -p modules/shell
        printf '%s\n' "$1" > modules/shell/Surface.qml
        if ${check} modules > said; then
          echo "The space check passed on: $1"
          exit 1
        fi
        if ! grep -qF -- "$2" said; then
          echo "Of $1 the space check didn't say: $2"
          cat said
          exit 1
        fi
      }
      strays 'leftPadding: Theme.space.l' 'Surface.qml:1: leftPadding: Theme.space.l'
      strays 'spacing: Theme.space.rung.items + Theme.space.xs' 'Theme.space.xs'
      # A rung, the beat and a pitch pass, as a comment naming a step does.
      rm -rf modules
      mkdir -p modules/shell
      printf '%s\n' 'spacing: Theme.space.rung.items' 'height: Theme.space.beat * 2' \
        'lineHeight: Theme.level.note.pitch' '// once Theme.space.l, now a rung' \
        > modules/shell/Surface.qml
      ${check} modules
      ${check} ${shells}
      touch $out
    '';

  # The launcher, the held notices and a notice are one card: each takes
  # its frame from Card.qml and stands its ink on the card's content, the
  # inset from its edge, and on its columns (Columns.js), never drawing a
  # frame, reading the inset, placing ink across by hand or tinting a row's
  # ground itself (every-card-is-the-one-card.py). So it can't pass by
  # seeing nothing, a border, the whole thing's radius, a shadow, a frame
  # from Floating, a read of the inset, an x or a margin from a rung or a
  # pitch, and a ground's tint are each shown to fail first, while a card, a
  # part's radius, a column, the card's own inset and a comment pass.
  every-card-is-the-one-card =
    let
      check = "${lib.getExe pkgs.python3} ${../every-card-is-the-one-card.py}";
      cards = [
        "HeldNotices.qml"
        "Launcher.qml"
        "NoticeRow.qml"
        "Notifications.qml"
      ];
      surfaces = lib.fileset.toSource {
        root = ../.;
        fileset = lib.fileset.unions (map (name: ../. + "/${name}") cards);
      };
    in
    pkgs.runCommandLocal "every-card-is-the-one-card" { } ''
      # A card surface drawn as given, and what the check must say of it.
      strays() {
        rm -rf shell
        mkdir shell
        printf '%s\n' "$1" > shell/Surface.qml
        if ${check} shell Surface.qml > said; then
          echo "The card check passed on: $1"
          exit 1
        fi
        if ! grep -qF -- "$2" said; then
          echo "Of $1 the card check didn't say: $2"
          cat said
          exit 1
        fi
      }
      strays 'border.width: Theme.border' 'Surface.qml:1: border.width: Theme.border'
      strays 'radius: Theme.radius' 'a frame of their own'
      strays 'RectangularShadow {}' 'a frame of their own'
      strays 'Floating {' 'take Card'
      strays 'x: Theme.space.rung.inset' 'read the inset'
      strays 'x: Theme.space.rung.mark' 'Surface.qml:1: x: Theme.space.rung.mark'
      strays 'x: gutter + Theme.level.name.pitch' 'by hand'
      strays 'anchors.leftMargin: Theme.space.rung.apart' 'by hand'
      strays 'rightPadding: Theme.space.rung.items' 'by hand'
      strays 'color: Qt.alpha(Theme.selection, Theme.floating.selection)' "a row's ground"
      # A card, a part's radius, a column, the card's own inset and the inset
      # said in a comment pass.
      rm -rf shell
      mkdir shell
      printf '%s\n' 'Card {' 'radius: Theme.inner' 'x: row.columns.text' 'x: -card.inset' \
        '// once at Theme.space.rung.inset' > shell/Surface.qml
      ${check} shell Surface.qml
      ${check} ${surfaces} ${lib.concatStringsSep " " cards}
      touch $out
    '';

  # A row's graph is ten braille characters over a dim track of every dot,
  # a bar filling from the left, a spark from the right with the newest last
  # and a strip a reading for each member, in the warning colour at 95% or
  # when it says it warns; rows' graphs stand in one column, and a figure
  # wears no glyph, as tests/graphs.qml draws them through Quickshell in the
  # theme's own face and sizes, at a panel's width.
  the-launchers-graphs-draw-in-braille =
    let
      machine = graphical.config;
    in
    pkgs.runCommandLocal "the-launchers-graphs-draw-in-braille" { } ''
      cp ${(account graphical).xdg.configFile."quickshell/Theme.qml".source} Theme.qml
      ${copied [
        "Choice.qml"
        "Chord.qml"
        "Graph.qml"
        "Ground.qml"
        "Mark.qml"
        "ReadsOn.qml"
        "Wash.qml"
      ]}
      cp ${../../../tests/graphs.qml} shell.qml
      export HOME=$TMPDIR XDG_RUNTIME_DIR=$TMPDIR/runtime LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen
      mkdir -m 700 $XDG_RUNTIME_DIR
      export FONTCONFIG_FILE=${pkgs.makeFontsConf { fontDirectories = machine.fonts.packages; }}
      ${shellFolderLint}
      timeout 60 ${qs} -p shell.qml
      touch $out
    '';

  # The empty places' cat is laid in whole pixels of the screen at 1, 1.5
  # and 2, starting on one, so it's as sharp at the laptop's fractional
  # scale as at 2, as tests/pixels.qml holds Pixels.js to in Qt's own engine.
  the-cat-is-drawn-in-whole-pixels =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "the-cat-is-drawn-in-whole-pixels" { } ''
      cp ${../../../tests/pixels.qml} pixels.qml
      ${copied [ "Pixels.js" ]}
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml pixels.qml
      touch $out
    '';

  # A floating surface standing over another, as the launcher's drawer over
  # its card on a narrow screen, meets it with a square foot, its region
  # alike, its top and the card's foot staying round, as tests/floating.qml
  # reads them back from what Quickshell draws.
  the-drawer-over-the-card-meets-it-square =
    pkgs.runCommandLocal "the-drawer-over-the-card-meets-it-square" { }
      ''
        cp ${(account graphical).xdg.configFile."quickshell/Theme.qml".source} Theme.qml
        ${copied [ "Floating.qml" ]}
        cp ${../../../tests/floating.qml} shell.qml
        export HOME=$TMPDIR XDG_RUNTIME_DIR=$TMPDIR/runtime LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen
        export FONTCONFIG_FILE=${noFonts}
        mkdir -m 700 $XDG_RUNTIME_DIR
        ${shellFolderLint}
        timeout 60 ${qs} -p shell.qml
        touch $out
      '';

  # A palette's row and its field keep their words apart and inside their
  # edges however long they run, ending what doesn't fit in an ellipsis, as
  # tests/rows.qml draws them through Quickshell in the theme's own face and
  # sizes, at a panel's width.
  the-launchers-rows-keep-their-words-apart =
    let
      machine = graphical.config;
      lights = pkgs.writeText "Lights.qml" ''
        pragma Singleton
        import Quickshell
        import QtQuick
        Singleton { property bool caps: false }
      '';
    in
    pkgs.runCommandLocal "the-launchers-rows-keep-their-words-apart" { } ''
      cp ${(account graphical).xdg.configFile."quickshell/Theme.qml".source} Theme.qml
      ${copied [
        "Choice.qml"
        "Chord.qml"
        "Graph.qml"
        "Ground.qml"
        "Gutter.qml"
        "Hairline.qml"
        "KeyHints.qml"
        "Mark.qml"
        "Prompt.qml"
        "PromptFoot.qml"
        "PromptHead.qml"
        "ReadsOn.qml"
        "StatusLine.qml"
        "Wash.qml"
      ]}
      cp ${lights} Lights.qml
      cp ${../../../tests/rows.qml} shell.qml
      export HOME=$TMPDIR XDG_RUNTIME_DIR=$TMPDIR/runtime LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen
      mkdir -m 700 $XDG_RUNTIME_DIR
      export FONTCONFIG_FILE=${pkgs.makeFontsConf { fontDirectories = machine.fonts.packages; }}
      ${shellFolderLint}
      timeout 60 ${qs} -p shell.qml > said 2>&1 || { cat said; exit 1; }
      cat said
      # A row reused for another lays itself out again, and a binding loop
      # in a row's parts is warned of on Joel's desktop on every step that
      # meets it, so one warned of here fails, as the desktop's own warnings
      # case does. The prompt's asideRoom loop is the prompt's own, and its
      # case is the prompt's.
      if grep -E '@(Choice|Chord|Gutter|Mark|ReadsOn|Wash)\.qml.*Binding loop' said; then
        exit 1
      fi
      touch $out
    '';

  # Shape is a language (docs/design.md, Shape and Kinds): a border's width
  # says "this is a thing" and a radius says whole or part, so no surface
  # picks either for itself. Every corner is the theme's radius, or a part's
  # radius the theme derives from it, or a floating surface's foot or top,
  # which is the theme's radius or square where it meets another
  # (Floating.qml's footRadius and topRadius), and every border is the
  # theme's.
  quickshell-takes-its-shape-from-the-theme =
    let
      files = everyShellFile;
      allowed = {
        radius = [
          "Theme.radius"
          "Theme.inner"
          "floating.footRadius"
          "floating.topRadius"
        ];
        width = [ "Theme.border" ];
      };
      shape =
        line:
        let
          space = "[[:space:]]*";
          property = "(border\\.width|[a-zA-Z]*[rR]adius)";
          value = "([^;]*[^;[:space:]])";
          found = builtins.match (space + property + space + ":" + space + value + space + ";?" + space) line;
        in
        if found == null then
          null
        else
          {
            kind = if lib.head found == "border.width" then "width" else "radius";
            value = lib.elemAt found 1;
          };
      culprits = lib.concatMap (
        file:
        lib.concatMap (
          line:
          let
            found = shape line;
          in
          lib.optional (
            found != null && !(lib.elem found.value allowed.${found.kind})
          ) "${file.name}: ${lib.trim line}"
        ) (lib.splitString "\n" (builtins.readFile file.path))
      ) files;
    in
    holds "quickshell-takes-its-shape-from-the-theme" ''
      These pick a radius or a border width of their own rather than the theme's:
      ${lib.concatStringsSep "\n" culprits}'' (files != [ ] && culprits == [ ]);

  # Space is one scale (docs/design.md, Space): a padding, a margin, a gap or
  # a line names a step, or sums and multiples of steps, so every surface
  # lands on the same few values. A raw number added to a step, a step
  # divided, or a raw padding, margin or spacing is a nudge off it, the way
  # the ink was once evened by hand. A count taken from (`length - 1`) and a
  # share of the screen (`parent.height / 12`) aren't steps, so they pass.
  quickshell-keeps-to-the-space-scale =
    let
      step = "Theme\\.(space\\.[a-z]+|inset|border|gap)";
      spacing = "([pP]adding|[mM]argins?|spacing)[[:space:]]*:";
      # Each raw number taken from or added to something, with what comes
      # before it and whether a step multiplies it.
      added = "([a-zA-Z_]*)[[:space:]]*[-+][[:space:]]*([0-9]+(\\.[0-9]+)?)([[:space:]]*[*])?";
      divided = "(${step}|[0-9])[[:space:]]*/[[:space:]]*[0-9]+";
      nudged =
        line:
        let
          adds = lib.filter lib.isList (builtins.split added line);
          raw = lib.any (
            found:
            lib.elemAt found 3 == null
            && !(lib.elem (lib.head found) [
              "length"
              "count"
              "index"
            ])
          ) adds;
        in
        builtins.match "[[:space:]]*//.*" line == null
        && (builtins.match ".*(${step}|${spacing}).*" line != null)
        && (
          raw
          || builtins.match ".*${divided}.*" line != null
          || builtins.match ".*${spacing}[[:space:]]*-?[1-9][0-9]*[[:space:]]*;?[[:space:]]*" line != null
        );
      culprits = lib.concatMap (
        file:
        map (line: "${file.name}: ${lib.trim line}") (
          lib.filter nudged (lib.splitString "\n" (builtins.readFile file.path))
        )
      ) everyShellFile;
    in
    holds "quickshell-keeps-to-the-space-scale" ''
      These nudge a step, or set a padding, margin or spacing, by a number off the scale:
      ${lib.concatStringsSep "\n" culprits}'' (everyShellFile != [ ] && culprits == [ ]);
}
