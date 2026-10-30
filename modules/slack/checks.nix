# The checks of slack, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  onLinux,
  passesClippy,
  pkgs,
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
  # each time, asking ahead taken only after a step-in and never asking
  # how they are, nothing but reads ever asked, and with no sign-in
  # nothing asked at all, said so. A stand-in on the machine answers for
  # Slack.
  slack-finds-who-it-knows = pkgs.callPackage ./package.nix { };
  # clippy finds nothing in slack-people, its tests included.
  slack-passes-clippy = passesClippy "slack-passes-clippy" (pkgs.callPackage ./package.nix { });
  # Who the source asks about and when, what stands in Write in Slack's
  # place before Slack answers, and the drawer's facts are Slack.js's, run
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
