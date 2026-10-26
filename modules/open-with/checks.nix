# The checks of open-with, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  graphical,
  holds,
  lib,
  onLinux,
  passesClippy,
  pkgs,
  ...
}:
onLinux {
  # clippy finds nothing in the portal's chooser of what to open something
  # with, its tests included.
  open-with-passes-clippy = passesClippy "open-with-passes-clippy" (
    pkgs.callPackage ./package.nix { }
  );

  # open-with chooses a file in the shell when the shell answers, and in
  # GTK's portal when it refuses, as a shell with no picker does, each
  # stood in for on a session bus of its own (files-chosen.py).
  files-are-chosen-with-or-without-the-picker =
    let
      python = pkgs.python3.withPackages (python: [ python.jeepney ]);
      picked = "/home/someone/picked.md";
      answers = pkgs.writeShellScript "qs-answers" ''
        [ "$3 $4" = "file-chooser ask" ] || exit 1
        echo '{"paths": ["${picked}"]}' | ${lib.getExe pkgs.socat} - UNIX-CONNECT:"$5"
      '';
    in
    pkgs.runCommandLocal "files-are-chosen-with-or-without-the-picker" { } ''
      export HOME=$TMPDIR XDG_RUNTIME_DIR=$TMPDIR/runtime
      ${lib.getExe' pkgs.coreutils "timeout"} 120 ${lib.getExe' pkgs.dbus "dbus-run-session"} \
        --dbus-daemon=${lib.getExe' pkgs.dbus "dbus-daemon"} \
        --config-file=${pkgs.dbus}/share/dbus-1/session.conf -- \
        ${lib.getExe python} ${./files-chosen.py} \
        ${lib.getExe (pkgs.callPackage ./package.nix { })} ${lib.getExe' pkgs.coreutils "false"} \
        ${answers} ${picked}
      touch $out
    '';

  # A graphical machine's portal asks the launcher for a file to open or
  # save, which hands it to GTK's dialog while the shell has no picker
  # (src/files.rs), and open-with's description says it answers that.
  the-launcher-chooses-files =
    let
      niri = graphical.config.xdg.portal.config.niri;
      portal = lib.findFirst (
        portal: portal.pname or "" == "open-with"
      ) null graphical.config.xdg.portal.extraPortals;
      described = builtins.readFile "${portal}/share/xdg-desktop-portal/portals/launcher.portal";
    in
    holds "the-launcher-chooses-files"
      "A graphical machine's file chooser isn't the launcher's, or open-with doesn't say it answers it."
      (
        niri."org.freedesktop.impl.portal.FileChooser" or null == "launcher"
        && lib.hasInfix "org.freedesktop.impl.portal.FileChooser;" described
      );

  # xdg-open opens a kind by the kinds it's a case of, in its declared app,
  # and a program that runs in a terminal in the terminal named for it:
  # markdown in the editor declared for text, in the terminal, and a
  # picture in its viewer. The apps and the terminal are stand-ins that
  # say what they were given; gio and xdg-terminal-exec are the real ones
  # (tests/xdg-open-opens-each-kind.sh).
  xdg-open-opens-each-kind-in-its-app =
    pkgs.runCommandLocal "xdg-open-opens-each-kind-in-its-app"
      {
        nativeBuildInputs = [
          (pkgs.callPackage ./xdg-open.nix { })
          pkgs.xdg-terminal-exec
        ];
      }
      ''
        ${lib.getExe pkgs.bash} ${../../tests/xdg-open-opens-each-kind.sh} ${pkgs.shared-mime-info}
        touch $out
      '';
}
