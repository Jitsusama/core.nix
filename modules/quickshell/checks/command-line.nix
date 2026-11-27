# The checks of the launcher's command line and query field, and the keys they
# take as Vim's, which the folder's checks.nix gathers with every other place's;
# its opening comment says why each place keeps its own.
{
  account,
  copied,
  graphical,
  noFonts,
  pkgs,
  shellFolderLint,
  timedQml,
  ...
}:
{
  # The region picker's keys start, move and size its box, and letters name the
  # grid's cells, as modules/quickshell/tests/picking.qml says, in Qt's own
  # engine.
  the-picker-picks-what-the-keys-say =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "the-picker-picks-what-the-keys-say" { } ''
      cp ${../tests/picking.qml} picking.qml
      ${copied [ "Picking.js" ]}
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml ${qt}/bin/qml picking.qml
      touch $out
    '';

  # Log Out, Reboot and Power Off arm on a first Return and end on a second with
  # no key between, as modules/quickshell/tests/arming.qml holds Arming.js to.
  an-ending-row-arms-before-it-ends =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "an-ending-row-arms-before-it-ends" { } ''
      cp ${../tests/arming.qml} arming.qml
      ${copied [ "Arming.js" ]}
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml arming.qml
      touch $out
    '';

  # Vim's command line is read as Vim reads it, as
  # modules/quickshell/tests/ex.qml holds Ex.js to: ranges, names by their short
  # forms, a bang, :g and :v, the rows a range names, and Vim's own errors.
  ex-reads-vims-command-lines =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "ex-reads-vims-command-lines" { } ''
      cp ${../tests/ex.qml} ex.qml
      ${copied [ "Ex.js" ]}
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml ex.qml
      touch $out
    '';

  # The launcher's command line lists every command it can take, the cursor
  # row's acts first, narrows as it's typed, chooses as his cmp does and runs
  # exactly what's typed, with Vim's ranges and :g, and reads :set as Vim does,
  # as modules/quickshell/tests/command-line.qml holds Commands.js and
  # SetWords.js to, with the Ex.js they read ranges by and CommandBinding.js,
  # which binds a line to things and asks by d's rule, in Operators.js.
  the-command-line-lists-every-command =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "the-command-line-lists-every-command" { } ''
      cp ${../tests/command-line.qml} command-line.qml
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
  # modules/quickshell/tests/query-keys.qml holds QueryKeys.js and Editing.js
  # to: insert's keys, normal's ways back into insert, its moves through the
  # rows with counts and where each lands, its closing keys and ex line, its
  # place used last, sequences, and where the caret goes, which Typing.js says;
  # ListedKeys.js to the keys Space ? lists; the drawer as a window, Space and
  # the arrows into it and back, with DrawerPage.js to where its keys take its
  # cursor and view, DrawerRun.js to what a key and a page shown lay of a long
  # run, DrawerLines.js to where its laid lines end or run on into the next,
  # DrawerFigure.js to where a fact's figure stands,
  # DrawerProse.js to where its prose is cut and DrawerValue.js to how its value
  # is laid on lines, with Identifier.js to keeping its cuts bounded and whole;
  # Operators.js to what d and y do to the rows they take; and Confirm.js to how
  # the launcher asks once, answered by Return as an armed row is; and
  # Changes.js to what u and Ctrl+R take back, with Search.js to how a page's
  # search matches; and launcher/Headings.js to how the rows' moves count past a
  # list's headings, and to which lists rest on a whole row; and KeyActs.js to
  # which keys act, the rest leaving the launcher's clock at the key before.
  the-query-field-takes-vi-modes =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "the-query-field-takes-vi-modes" { } ''
      cp ${../tests/query-keys.qml} query-keys.qml
      ${copied [
        "Changes.js"
        "Confirm.js"
        "DrawerFigure.js"
        "DrawerLines.js"
        "DrawerPage.js"
        "DrawerProse.js"
        "DrawerRun.js"
        "DrawerValue.js"
        "Editing.js"
        "KeyActs.js"
        "KeyNames.js"
        "ListedKeys.js"
        "ModeMarks.js"
        "launcher/Headings.js"
      ]}
      cp ${(import ../launcher/words.nix).file} words.json
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts} QML_XHR_ALLOW_FILE_READ=1
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml query-keys.qml
      touch $out
    '';

  # A modal Prompt.qml says a key acted exactly where its own handlers did
  # something with it, keys sent through Qt as the launcher's field takes them
  # (modules/quickshell/tests/prompt-keys.qml), and the command line names the
  # key that let it go, which the launcher times as acting
  # (modules/quickshell/tests/command-line-keys.qml), in Qt's test runner. The
  # account's Theme.qml is made a plain singleton, as Quickshell's isn't
  # there, Lights.qml a stand-in with caps lock off, and Quickshell a module
  # outside the linted folder holding only a clipboard; the lint reads
  # Quickshell's own types.
  the-launchers-field-says-which-keys-acted =
    let
      qt = pkgs.qt6.qtdeclarative;
      theme = (account graphical).xdg.configFile."quickshell/Theme.qml".source;
      lights = pkgs.writeText "Lights.qml" ''
        pragma Singleton
        import QtQuick
        QtObject { property bool caps: false }
      '';
      clipboard = pkgs.writeText "Quickshell.qml" ''
        pragma Singleton
        import QtQuick
        QtObject { property string clipboardText: "pasted\nnot this" }
      '';
      quickshell = pkgs.runCommandLocal "a-clipboard-quickshell" { } ''
        mkdir -p $out/Quickshell
        printf '%s\n' 'module Quickshell' 'singleton Quickshell 1.0 Quickshell.qml' \
          > $out/Quickshell/qmldir
        cp ${clipboard} $out/Quickshell/Quickshell.qml
      '';
    in
    # Kept in Nix because each line copies, or names in a variable, a file
    # Nix gives it, and the cases it runs are QML files of their own
    # (modules/quickshell/tests/prompt-keys.qml and command-line-keys.qml).
    pkgs.runCommandLocal "the-launchers-field-says-which-keys-acted" { } ''
      cp ${../tests/prompt-keys.qml} prompt-keys.qml
      cp ${../tests/command-line-keys.qml} command-line-keys.qml
      ${copied [
        "CommandLine.qml"
        "Prompt.qml"
        "TextMeasure.qml"
      ]}
      sed -e '/^import Quickshell$/d' -e 's/^Singleton {$/QtObject {/' ${theme} > Theme.qml
      grep -q '^QtObject {$' Theme.qml || { echo "Theme.qml is no Singleton" >&2; exit 1; }
      cp ${lights} Lights.qml
      printf '%s\n' 'singleton Theme Theme.qml' 'singleton Lights Lights.qml' \
        'CommandLine CommandLine.qml' 'Prompt Prompt.qml' 'TextMeasure TextMeasure.qml' > qmldir
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qmltestrunner -input prompt-keys.qml
      QML_IMPORT_PATH=${quickshell}:${qt}/lib/qt-6/qml timeout 60 \
        ${qt}/bin/qmltestrunner -input command-line-keys.qml
      touch $out
    '';

  # A drawer's search over hyphenated names, as Emoji's every part is, keeps
  # within a multiple of the same words with no joiners, and that within one of
  # the plainest search there is, timed as the drawer runs it, DrawerPage.js's
  # matchesIn, in at least one of five processes of Qt's own engine
  # (modules/quickshell/tests/drawer-search-pace.qml).
  the-drawer-search-keeps-pace = pkgs.runCommandLocal "the-drawer-search-keeps-pace" { } ''
    cp ${../tests/drawer-search-pace.qml} drawer-search-pace.qml
    ${copied [ "DrawerPage.js" ]}
    export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
    export FONTCONFIG_FILE=${noFonts}
    ${timedQml "drawer-search-pace.qml"}
    touch $out
  '';
}
