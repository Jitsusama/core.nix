# The checks of slack, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  account,
  bareNixos,
  holds,
  lib,
  onLinux,
  passesClippy,
  pkgs,
  self,
  shellFolderLint,
  ...
}:
# slack-people serves the Linux desktop's shell, and builds only on Linux.
onLinux {
  # slack-people's tests run as it builds: who Slack knows by an address is
  # read from its answer, by a search a profile confirms where a grid
  # refuses the look-up, with whether they're active, away or in a huddle
  # and their status while it lasts; one it knows nobody by said so, a
  # sign-in it refuses said once and nobody more asked about, who each
  # address is found once, kept across a restart, and how they are asked
  # each time, their conversation with Joel found in plain words, asking
  # ahead taken only after a step-in, finding their conversation once and
  # never asking how they are, nothing but reads ever asked, and with no sign-in
  # nothing asked at all, said so; and slack-unreads keeps what wants Joel in
  # a file, whole, each conversation named once, signed out kept as such,
  # with no sign-in no file, and over its socket a search says each message
  # found and what's around one, Slack too busy told apart. A stand-in on
  # the machine answers for Slack.
  slack-finds-who-it-knows = pkgs.callPackage ./package.nix { };
  # clippy finds nothing in slack-people, its tests included.
  slack-passes-clippy = passesClippy "slack-passes-clippy" (pkgs.callPackage ./package.nix { });
  # slack-unreads starts once pi has signed in to Slack, comes back as the
  # new one after a switch, is restarted once its watchdog hears nothing,
  # and takes its file with it however it stops, so Now never shows a count
  # nothing is keeping current.
  slack-unreads-go-with-the-service =
    let
      home = account (bareNixos [ { home-manager.sharedModules = [ self.homeModules.slack ]; } ]);
      unit = home.systemd.user.services.slack-unreads;
      credentials = "${home.xdg.configHome}/agentic-harness/slack/credentials.json";
    in
    holds "slack-unreads-go-with-the-service"
      "slack-unreads lacks its start with the sign-in, switch restart, watchdog or file cleanup."
      (
        home.systemd.user.paths.slack-unreads.Path.PathExists == credentials
        && unit.Unit.X-SwitchMethod == "restart"
        && unit.Service.WatchdogSec > 0
        && unit.Service.NotifyAccess == "main"
        && lib.hasInfix "%t/slack-unreads.json" unit.Service.ExecStopPost
        && lib.hasInfix "%t/daemons/slack.sock" unit.Service.ExecStopPost
      );
  # Who the source asks about and when, what stands in Write in Slack's
  # place before Slack answers, how answers that come together are taken,
  # the drawer's facts and a conversation's drawer are Slack.js's, run
  # in Qt's own engine (tests/slack.qml).
  slack-waits-in-its-place =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "slack-waits-in-its-place" { } ''
      cp ${../../tests/slack.qml} slack.qml
      cp ${./Slack.js} Slack.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${pkgs.makeFontsConf { fontDirectories = [ ]; }}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml slack.qml
      touch $out
    '';
}
