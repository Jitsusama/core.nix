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

  # open-with chooses a file in the shell when the shell answers, and none
  # when it can't be asked, as while it restarts, never asking GTK's
  # portal, each stood in for on a session bus of its own (files-chosen.py).
  files-are-chosen-in-the-launcher-alone =
    let
      python = pkgs.python3.withPackages (python: [ python.jeepney ]);
      picked = "/home/someone/picked.md";
      answers = pkgs.writeShellScript "qs-answers" ''
        [ "$3 $4" = "file-chooser ask" ] || exit 1
        echo '{"paths": ["${picked}"]}' | ${lib.getExe pkgs.socat} - UNIX-CONNECT:"$5"
      '';
    in
    pkgs.runCommandLocal "files-are-chosen-in-the-launcher-alone" { } ''
      export HOME=$TMPDIR XDG_RUNTIME_DIR=$TMPDIR/runtime
      mkdir -m 700 "$XDG_RUNTIME_DIR"
      ${lib.getExe' pkgs.coreutils "timeout"} 120 ${lib.getExe' pkgs.dbus "dbus-run-session"} \
        --dbus-daemon=${lib.getExe' pkgs.dbus "dbus-daemon"} \
        --config-file=${pkgs.dbus}/share/dbus-1/session.conf -- \
        ${lib.getExe python} ${./files-chosen.py} \
        ${lib.getExe (pkgs.callPackage ./package.nix { })} ${lib.getExe' pkgs.coreutils "false"} \
        ${answers} ${picked}
      touch $out
    '';

  # A GTK 3 program's own dialog for choosing a file never shows: the
  # module (dialogs-ask-the-portal.c) asks the portal in its place, and the
  # dialog answers with what the portal chose, or as closed when it's
  # cancelled. The program (dialog-of-its-own.c) runs in a headless cage,
  # with a stand-in for the portal on a session bus of its own
  # (dialogs-asked.py).
  gtk-3-dialogs-ask-the-portal =
    let
      python = pkgs.python3.withPackages (python: [ python.jeepney ]);
      gtk = "${pkgs.gtk3}/share/gsettings-schemas/${pkgs.gtk3.name}";
      dialog =
        pkgs.runCommandCC "dialog-of-its-own"
          {
            nativeBuildInputs = [ pkgs.pkg-config ];
            buildInputs = [ pkgs.gtk3 ];
          }
          ''
            mkdir -p "$out/bin"
            # shellcheck disable=SC2046 # pkg-config's flags are words.
            $CC -O2 -Wall -Wextra -Werror $(pkg-config --cflags gtk+-3.0) ${./dialog-of-its-own.c} \
              -o "$out/bin/dialog-of-its-own" $(pkg-config --libs gtk+-3.0)
          '';
      module = pkgs.callPackage ./dialogs-ask-the-portal.nix { };
    in
    pkgs.runCommandLocal "gtk-3-dialogs-ask-the-portal" { nativeBuildInputs = [ pkgs.cage ]; } ''
      export HOME=$TMPDIR XDG_RUNTIME_DIR=$TMPDIR/runtime XDG_DATA_DIRS=${gtk}
      export GDK_BACKEND=wayland GSETTINGS_BACKEND=memory
      export WLR_BACKENDS=headless WLR_RENDERER=pixman WLR_LIBINPUT_NO_DEVICES=1
      export FONTCONFIG_FILE=${pkgs.makeFontsConf { fontDirectories = [ ]; }}
      export GTK_USE_PORTAL=1 GTK3_MODULES=${module}/lib/libdialogs-ask-the-portal.so
      mkdir -m 700 "$XDG_RUNTIME_DIR"
      mkdir drafts
      # cage ends when the script does, so only a script that got to its end
      # leaves its mark.
      ${lib.getExe' pkgs.coreutils "timeout"} 180 ${lib.getExe' pkgs.dbus "dbus-run-session"} \
        --dbus-daemon=${lib.getExe' pkgs.dbus "dbus-daemon"} \
        --config-file=${pkgs.dbus}/share/dbus-1/session.conf -- \
        cage -- ${lib.getExe pkgs.bash} -c \
        '${lib.getExe python} ${./dialogs-asked.py} ${dialog}/bin/dialog-of-its-own "$PWD/drafts" && touch passed'
      [ -e passed ]
      touch $out
    '';

  # A graphical machine's portal asks the launcher for a file to open or
  # save, and never GTK's dialog (src/files.rs), open-with's description
  # says it answers that, and GTK
  # 3's native chooser is told to ask the portal, its own dialogs asking
  # through the native one (dialogs-ask-the-portal.c).
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
        && graphical.config.environment.sessionVariables.GTK_USE_PORTAL or null == "1"
        && lib.hasInfix "/lib/libdialogs-ask-the-portal.so" (
          graphical.config.environment.sessionVariables.GTK3_MODULES or ""
        )
        && lib.hasInfix "org.freedesktop.impl.portal.FileChooser;" described
      );

  # xdg-open opens a kind by the kinds it's a case of, in its declared
  # program, and a program that runs in a terminal in the terminal named for
  # it: markdown in the editor declared for text, in the terminal, and a
  # picture in its viewer. The programs and the terminal are stand-ins that
  # say what they were given; gio and xdg-terminal-exec are the real ones
  # (tests/xdg-open-opens-each-kind.sh).
  xdg-open-opens-each-kind-in-its-program =
    pkgs.runCommandLocal "xdg-open-opens-each-kind-in-its-program"
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
