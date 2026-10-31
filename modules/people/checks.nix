# The checks of people, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  account,
  bareNixos,
  graphical,
  holds,
  lib,
  onLinux,
  passesClippy,
  pkgs,
  self,
  shellFolderLint,
  ...
}:
# People serves the Linux desktop's shell, and builds only on Linux.
onLinux {
  # The people daemon's tests run as it builds: a get is a step-in answered
  # by every source within its patience, a watch asks ahead, says what's
  # held and gathers a burst of changes into one, a source signed out is
  # never waited on, one that stops is started again and asked again about
  # everyone watched, a get nothing answers in time is not ready, and an
  # ask or a kind it doesn't know is refused. Stand-in sources in the
  # shell answer for the real ones.
  people-holds-everyone-ready = pkgs.callPackage ./package.nix { };
  # clippy finds nothing in the people daemon, its tests included.
  people-passes-clippy = passesClippy "people-passes-clippy" (pkgs.callPackage ./package.nix { });
  # The daemon runs wherever a source names a program for it, as Slack's
  # does, on the socket the shell's daemons share, and not where none
  # does, as on the example desktop, whose sources are QML alone.
  people-runs-where-a-source-names-a-program =
    let
      slack = account (bareNixos [ { home-manager.sharedModules = [ self.homeModules.slack ]; } ]);
      started = slack.systemd.user.services.people.Service.ExecStart or "";
    in
    holds "people-runs-where-a-source-names-a-program"
      "The people daemon doesn't run for Slack's source, or runs where no source names one."
      (
        lib.hasInfix "%t/daemons/people.sock" started
        && slack.systemd.user.services.people.Unit.X-SwitchMethod == "restart"
        && !((account graphical).systemd.user.services ? people)
      );
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
