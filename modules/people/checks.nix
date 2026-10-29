# The checks of people, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  onLinux,
  pkgs,
  shellFolderLint,
  ...
}:
# People serves the Linux desktop's shell, and builds only on Linux.
onLinux {
  # One person per email address across every source, how each stands, who's
  # listed and in what order, the drawer's details and the order of their
  # acts are Person.js's, run in Qt's own engine (tests/person.qml).
  the-people-are-one-person-each =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "the-people-are-one-person-each" { } ''
      cp ${../../tests/person.qml} person.qml
      cp ${./Person.js} Person.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${pkgs.makeFontsConf { fontDirectories = [ ]; }}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml person.qml
      touch $out
    '';
}
