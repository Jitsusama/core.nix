# The checks of the launcher's timers, and the time they keep, which the
# folder's checks.nix gathers with every other place's; its opening comment says
# why each place keeps its own.
{
  noFonts,
  passesClippy,
  pkgs,
  shellFolderLint,
  ...
}:
{
  # A timer's span and name are read from what's typed after "timer", its
  # time left told as a clock and its span in its largest units, and timers
  # listed rung first, and what Now's Timers row says, as tests/timing.qml
  # holds launcher/Timing.js to it in Qt's own engine.
  a-timer-is-read-from-what-is-typed =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "a-timer-is-read-from-what-is-typed" { } ''
      cp ${../../../tests/timing.qml} timing.qml
      cp ${../launcher/Timing.js} Timing.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml timing.qml
      touch $out
    '';

  # clippy finds nothing in timekeeping, its tests included.
  timekeeping-passes-clippy = passesClippy "timekeeping-passes-clippy" (
    pkgs.callPackage ../timekeeping/package.nix { }
  );

  # The timers daemon's tests run as it builds: a timer started is told to
  # every watch and rings as a critical notice once its end passes, the
  # notice's Again sets it going afresh and putting it away ends it, ending
  # one rung takes its notice back, a ring with no server to take it is
  # said once one comes, what's kept outlasts the daemon, one that ended
  # meanwhile ringing late, and an ask it can't do is refused.
  timers-are-kept-and-rung = pkgs.callPackage ../timers/package.nix { };

  # clippy finds nothing in the timers daemon, its tests included.
  timers-passes-clippy = passesClippy "timers-passes-clippy" (
    pkgs.callPackage ../timers/package.nix { }
  );
}
