# The checks of the launcher's Capture: screenshots, what each took, and
# recording a window, which the folder's checks.nix gathers with every other
# place's; its opening comment says why each place keeps its own.
{
  copied,
  lib,
  noFonts,
  passesClippy,
  pkgs,
  programs,
  qs,
  shellFolderLint,
  ...
}:
{
  # What was taken lately reads its folder through the programs daemon,
  # handed a source for it as modules/quickshell/home.nix hands one, and
  # finds screenshots by their words (Taken.qml), asking a stand-in for the
  # screen-text daemon
  # (tests/desktop/stand-in-screen-text.py) over one connection made on the
  # first search and let go at rest, each text once, the last few kept,
  # none for fewer than three letters, a ragged line given as one short
  # one, and finds a few thousand files by name, newest first, as Recent
  # does should the programs daemon not search, as tests/taken.qml holds it
  # to, with what each screenshot says in
  # modules/quickshell/tests/stand-ins/taken-texts.json.
  the-desktop-finds-screenshots-by-their-words =
    let
      standIn = pkgs.writers.writePython3 "stand-in-screen-text" {
        doCheck = false;
      } (builtins.readFile ../../../tests/desktop/stand-in-screen-text.py);
    in
    pkgs.runCommandLocal "the-desktop-finds-screenshots-by-their-words" { } ''
      mkdir -p runtime/daemons shots && chmod 700 runtime
      touch shots/2026-10-{29,30,31}\ 09-00-00.png
      substitute ${../tests/stand-ins/taken-texts.json} texts.json --subst-var-by shots "$PWD/shots"
      printf '[{"name": "taken", "folders": ["%s"]}]' "$PWD/shots" > sources.json
      ${copied [
        "Daemon.qml"
        "Taken.qml"
      ]}
      cp ${../../../tests/taken.qml} shell.qml
      export HOME=$TMPDIR XDG_RUNTIME_DIR=$PWD/runtime LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts} SHOTS=$PWD/shots
      ${standIn} runtime/daemons/screen-text.sock texts.json &
      ${lib.getExe programs} runtime/daemons/programs.sock sources.json &
      ${shellFolderLint}
      timeout 60 ${qs} -p shell.qml
      touch $out
    '';

  # What Capture took says where it is and what kind, when it was taken,
  # its size and a picture's dimensions or a recording's length, as
  # tests/captured.qml holds launcher/Captured.js to what Taken.qml reads
  # and file-preview says.
  a-capture-says-what-it-took =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "a-capture-says-what-it-took" { } ''
      mkdir launcher
      cp ${../../../tests/captured.qml} captured.qml
      ${copied [ "launcher/Captured.js" ]}
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml captured.qml
      touch $out
    '';

  # clippy finds nothing in the window recorder, its tests included.
  record-window-passes-clippy = passesClippy "record-window-passes-clippy" (
    pkgs.callPackage ../record-window/package.nix { }
  );
}
