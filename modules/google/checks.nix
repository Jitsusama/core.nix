# The checks of google, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  account,
  graphical,
  holds,
  lib,
  onLinux,
  passesClippy,
  pkgs,
  shellFolderLint,
  ...
}:
# google serves the Linux desktop's shell, and builds only on Linux.
onLinux {
  # A switch restarts google after it reloads the units, so the program the
  # shell reads is never the one from before the switch.
  google-switches-to-the-new-one =
    holds "google-switches-to-the-new-one"
      "A switch stops google before reloading, so google.path starts the old one again."
      (((account graphical).systemd.user.services.google.Unit.X-SwitchMethod or null) == "restart");

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
  # wants Joel no more, mail read here and marked unread again wants him
  # again at once, the documents of each kind are told, Drive asked again
  # only after a while and kept while it fails, the shell asks only what's
  # read strictly, and it hears the state on connecting and each change
  # once. A stand-in on the machine answers for Google.
  google-reads-the-calendar = pkgs.callPackage ./package.nix { };
  # clippy finds nothing in google, its tests included.
  google-passes-clippy = passesClippy "google-passes-clippy" (pkgs.callPackage ./package.nix { });

  # Which meeting is next, when it is in words and whether it's about to
  # start are Calendar.js's, and which Meet call a window is Meet.js's, run
  # in Qt's own engine over meetings like the program's (tests/calendar.qml).
  the-shell-tells-the-next-meeting =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "the-shell-tells-the-next-meeting" { } ''
      cp ${../../tests/calendar.qml} calendar.qml
      cp ${./Calendar.js} Calendar.js
      cp ${./Meet.js} Meet.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${pkgs.makeFontsConf { fontDirectories = [ ]; }}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml calendar.qml
      touch $out
    '';

  # The documents read from the program's file, which program opens each
  # kind, what a document's row says and its details are Drive.js's, run in
  # Qt's own engine (tests/drive.qml).
  the-shell-reads-the-documents =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "the-shell-reads-the-documents" { } ''
      cp ${../../tests/drive.qml} drive.qml
      cp ${./Drive.js} Drive.js
      cp ${./Mail.js} Mail.js
      cp ${./Calendar.js} Calendar.js
      cp ${./Google.js} Google.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${pkgs.makeFontsConf { fontDirectories = [ ]; }}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml drive.qml
      touch $out
    '';

  # Reading the program's file is Google.js's, and when mail came and the
  # lines marking it read or archiving it Mail.js's, run in Qt's own engine
  # (tests/mail.qml).
  the-shell-reads-the-mail =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "the-shell-reads-the-mail" { } ''
      cp ${../../tests/mail.qml} mail.qml
      cp ${./Google.js} Google.js
      cp ${./Mail.js} Mail.js
      cp ${./Calendar.js} Calendar.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${pkgs.makeFontsConf { fontDirectories = [ ]; }}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml mail.qml
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
