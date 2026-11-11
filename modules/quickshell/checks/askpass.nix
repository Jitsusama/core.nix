# The checks of the prompts beside the launcher: polkit's and SSH's askpass,
# which the folder's checks.nix gathers with every other place's; its opening
# comment says why each place keeps its own.
{
  account,
  graphical,
  lib,
  noFonts,
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
      cp ${../Askpass.js} Askpass.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml ${qt}/bin/qml askpass.qml
      touch $out
    '';

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
