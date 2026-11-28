# The checks of On the Selection, the place on Super+X, which the folder's
# checks.nix gathers with every other place's; its opening comment says why
# each place keeps its own.
{
  account,
  copied,
  graphical,
  noFonts,
  pkgs,
  qs,
  shellFolderLint,
  ...
}:
{
  # A pi-verbs record the daemon keys as one of the launcher's own kinds is
  # kept under the node's row:, what its Return posed is held by the record's
  # own key, and the answer standing in its row's place keeps the row's key
  # (modules/quickshell/tests/on-selection-keys.qml), with pi-verbs and the
  # clipboard daemon stood in by one Daemon, answering an ask at once
  # (modules/quickshell/tests/stand-ins/selection-daemon.qml), and Ask's warm
  # pi by the case.
  the-selections-rows-keep-the-records-key =
    pkgs.runCommandLocal "the-selections-rows-keep-the-records-key" { FONTCONFIG_FILE = noFonts; }
      ''
        ${copied [
          "Words.qml"
          "launcher/OnSelection.qml"
          "launcher/Provider.qml"
          "launcher/words.json"
        ]}
        cp ${(account graphical).xdg.configFile."quickshell/Theme.qml".source} Theme.qml
        cp ${../tests/on-selection-keys.qml} shell.qml
        cp ${../../../tests/lib/Expect.js} Expect.js
        cp ${../tests/stand-ins/selection-daemon.qml} Daemon.qml
        export HOME=$TMPDIR XDG_RUNTIME_DIR=$TMPDIR/runtime LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen
        mkdir -m 700 $XDG_RUNTIME_DIR
        ${shellFolderLint}
        timeout 60 ${qs} -p shell.qml
        touch $out
      '';

  # The selected text heads the place on one line, given way at a whole word
  # (VKCF3Z, e684d7ae), an answer is named by its first sentence, and pi-verbs'
  # actions keep the order it served them in, as
  # modules/quickshell/tests/on-selection.qml holds launcher/OnSelection.js to
  # it in Qt's own engine.
  the-launcher-heads-the-selection-at-a-whole-word =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "the-launcher-heads-the-selection-at-a-whole-word" { } ''
      cp ${../tests/on-selection.qml} on-selection.qml
      cp ${../../../tests/lib/Expect.js} Expect.js
      ${copied [ "launcher/OnSelection.js" ]}
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml on-selection.qml
      touch $out
    '';
}
