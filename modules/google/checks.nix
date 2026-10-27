# The checks of google, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  lib,
  onLinux,
  passesClippy,
  pkgs,
  shellFolderLint,
  ...
}:
# google serves the Linux desktop's shell, and builds only on Linux.
onLinux {
  # google's tests run as it builds: the harness's file is read for its
  # default or first account and never written, pi's fresh token is used
  # and a stale one refreshed in memory, a refused sign-in is signed out
  # and not offered again until a new one, a revoked token is refreshed
  # past once, an unreachable Google keeps what was heard, every page of
  # meetings is joined, each meeting's times, answer and the one link it's
  # joined by are read, an invitation is answered by changing only Joel's
  # own answer, and held at once, one Google refuses changing nothing, the
  # mail that wants Joel is told newest first, each asked for once and kept
  # while Gmail fails, mail marked read or archived takes one label off and
  # wants Joel no more, the shell asks only what's read strictly, and it hears the state on
  # connecting and each change once. A stand-in on the machine answers for
  # Google.
  google-reads-the-calendar = pkgs.callPackage ./package.nix { };
  # clippy finds nothing in google, its tests included.
  google-passes-clippy = passesClippy "google-passes-clippy" (pkgs.callPackage ./package.nix { });

  # Which meeting is next, when it is in words and whether it's about to
  # start are Calendar.js's, run in Qt's own engine over meetings like the
  # program's (tests/calendar.qml), as is reading the socket's lines.
  the-shell-tells-the-next-meeting =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "the-shell-tells-the-next-meeting" { } ''
      cp ${../../tests/calendar.qml} calendar.qml
      cp ${./Calendar.js} Calendar.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${pkgs.makeFontsConf { fontDirectories = [ ]; }}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml calendar.qml
      touch $out
    '';

  # A meeting's link is opened as it's meant to be, by opens.sh with
  # stand-ins for Tuple, the window and the browser
  # (tests/google-opens-its-links.sh).
  google-opens-its-links = pkgs.runCommandLocal "google-opens-its-links" { } ''
    ${lib.getExe pkgs.bash} ${../../tests/google-opens-its-links.sh} ${./opens.sh}
    touch $out
  '';
}
