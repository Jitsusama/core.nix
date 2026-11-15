# The checks of the prompts beside the launcher: polkit's and SSH's askpass,
# which the folder's checks.nix gathers with every other place's; its opening
# comment says why each place keeps its own.
{
  account,
  copied,
  graphical,
  lib,
  noFonts,
  passesClippy,
  pkgs,
  shellFolderLint,
  ...
}:
{
  # The askpass prompt says who's asking for what, for each program's words,
  # as tests/askpass.qml says, in Qt's own engine.
  the-askpass-says-who-asks-for-what =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "the-askpass-says-who-asks-for-what" { } ''
      cp ${../../../tests/askpass.qml} askpass.qml
      ${copied [ "Askpass.js" ]}
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml ${qt}/bin/qml askpass.qml
      touch $out
    '';

  # Polkit's prompt names the command asking and the account it would run
  # as, for run0, pkexec and the rest, as tests/polkit.qml says, in Qt's own
  # engine.
  polkits-prompt-says-what-runs =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "polkits-prompt-says-what-runs" { } ''
      cp ${../../../tests/polkit.qml} polkit.qml
      ${copied [ "Polkit.js" ]}
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml ${qt}/bin/qml polkit.qml
      touch $out
    '';

  # asker-says's tests run as it builds: the quest and session a process
  # says of itself, read by those two names alone, from it or its nearest
  # parent that says them, and a secret beside them never said.
  asker-says-only-what-a-process-says = pkgs.callPackage ../asker-says/package.nix { };

  # clippy finds nothing in asker-says, its tests included.
  asker-says-passes-clippy = passesClippy "asker-says-passes-clippy" (
    pkgs.callPackage ../asker-says/package.nix { }
  );

  # The askpass asks a shell that restarted mid-prompt again, and refuses
  # once none has answered for a while, rather than holding ssh-tpm-agent
  # and every key behind it forever, as
  # modules/quickshell/tests/askpass-waits.sh says. The desktop test restarts
  # a real shell, in minutes; this says it in one.
  the-askpass-outlives-its-shell =
    let
      askpass = (account graphical).home.sessionVariables.SSH_ASKPASS;
    in
    pkgs.runCommandLocal "the-askpass-outlives-its-shell" { } ''
      ${lib.getExe' pkgs.coreutils "timeout"} 300 ${lib.getExe pkgs.bash} \
        ${../tests/askpass-waits.sh} ${askpass} ||
        { echo "the askpass is still waiting, or got something wrong"; exit 1; }
      touch $out
    '';

  # A D-Bus call from the account's shells, as run0 asks for polkit's prompt
  # in, waits past systemd's own 25 seconds for an answer, as
  # modules/quickshell/tests/prompt-outlasts-the-bus.py says, on a session
  # bus of its own.
  a-prompt-waits-for-a-person =
    let
      timeout = (account graphical).home.sessionVariables.SYSTEMD_BUS_TIMEOUT or "";
      python = pkgs.python3.withPackages (python: [ python.jeepney ]);
    in
    pkgs.runCommandLocal "a-prompt-waits-for-a-person" { } ''
      export HOME=$TMPDIR
      ${lib.getExe' pkgs.dbus "dbus-run-session"} \
        --dbus-daemon=${lib.getExe' pkgs.dbus "dbus-daemon"} \
        --config-file=${pkgs.dbus}/share/dbus-1/session.conf -- \
        ${lib.getExe python} ${../tests/prompt-outlasts-the-bus.py} \
        ${lib.escapeShellArg timeout} ${lib.getExe' pkgs.systemd "busctl"}
      touch $out
    '';
}
