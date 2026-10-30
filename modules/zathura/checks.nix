# The checks of zathura, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  account,
  graphical,
  lib,
  onLinux,
  pkgs,
  ...
}:
onLinux {
  # zathura opens a page with the account's settings. It names a setting
  # or a colour it can't take and goes on without it, and stays open until
  # it's closed, so it's given a few seconds to say so. Documents open in
  # zathura.
  zathura-accepts-its-configuration =
    let
      home = account graphical;
      opens = graphical.config.xdg.mime.defaultApplications."application/pdf" or "";
      page = import ../../tests/page.nix { inherit lib pkgs; };
    in
    assert opens == "org.pwmt.zathura-pdf-mupdf.desktop";
    pkgs.runCommandLocal "zathura-accepts-its-configuration"
      {
        nativeBuildInputs = [
          home.programs.zathura.package
          pkgs.cage
        ];
      }
      ''
        export HOME=$PWD XDG_CONFIG_HOME=$PWD/config XDG_DATA_HOME=$PWD/data
        export XDG_RUNTIME_DIR=$PWD/run GDK_BACKEND=wayland
        export FONTCONFIG_FILE=${pkgs.makeFontsConf { fontDirectories = [ ]; }}
        export WLR_BACKENDS=headless WLR_RENDERER=pixman WLR_LIBINPUT_NO_DEVICES=1
        mkdir -p config/zathura data run
        chmod 700 run
        cp ${home.xdg.configFile."zathura/zathurarc".source} config/zathura/zathurarc
        # cage ends when zathura does, so only a zathura still open at the
        # end of the wait opened.
        status=0
        timeout 5 cage -- zathura ${page} > said 2>&1 || status=$?
        if [ $status -ne 124 ]; then cat said; exit 1; fi
        if grep -E '^(warning|error):' said; then exit 1; fi
        touch $out
      '';

  # Its keys go where the launcher says they go: Left and Right turn the
  # pages, Tab goes back to the place used last and again returns, and ?
  # asks the launcher for the keys, which keys-test.sh reads back from
  # zathura itself over its bus, in a compositor of its own.
  zathura-keys-go-where-they-say =
    let
      home = account graphical;
      pages = import ../../tests/page.nix {
        inherit lib pkgs;
        pages = 3;
      };
      # The launcher, which only has to say it was asked.
      launcher = pkgs.writeShellScriptBin "launcher-say" ''echo "$*" > asked'';
    in
    pkgs.runCommandLocal "zathura-keys-go-where-they-say"
      {
        nativeBuildInputs = [
          home.programs.zathura.package
          launcher
          pkgs.cage
          pkgs.dbus
          pkgs.glib
          pkgs.wtype
        ];
      }
      ''
        export HOME=$PWD XDG_CONFIG_HOME=$PWD/config XDG_DATA_HOME=$PWD/data
        export XDG_RUNTIME_DIR=$PWD/run GDK_BACKEND=wayland
        export FONTCONFIG_FILE=${pkgs.makeFontsConf { fontDirectories = [ ]; }}
        export WLR_BACKENDS=headless WLR_RENDERER=pixman WLR_LIBINPUT_NO_DEVICES=1
        mkdir -p config/zathura data run
        chmod 700 run
        cp ${home.xdg.configFile."zathura/zathurarc".source} config/zathura/zathurarc
        dbus-run-session --config-file=${pkgs.dbus}/share/dbus-1/session.conf -- \
          cage -- $BASH ${./keys-test.sh} ${pages} > ran 2>&1 || true
        if [ ! -e passed ]; then cat ran; exit 1; fi
        touch $out
      '';

  # zathura says the document it has open, read over its bus by the command
  # its Recent in the launcher is led by, as its process and the whole
  # path of a document given by name in the folder it was started in, and
  # nothing once it quits (open-now-test.sh), in a compositor of its own.
  zathura-says-the-document-it-has-open =
    let
      home = account graphical;
      pages = import ../../tests/page.nix {
        inherit lib pkgs;
        pages = 1;
      };
      source = lib.findFirst (
        each: each.app or "" == "org.pwmt.zathura"
      ) null home.jitsusama.quickshell.launcher.recents;
    in
    pkgs.runCommandLocal "zathura-says-the-document-it-has-open"
      {
        nativeBuildInputs = [
          home.programs.zathura.package
          pkgs.cage
          pkgs.dbus
        ];
      }
      ''
        export HOME=$PWD XDG_CONFIG_HOME=$PWD/config XDG_DATA_HOME=$PWD/data
        export XDG_RUNTIME_DIR=$PWD/run GDK_BACKEND=wayland
        export FONTCONFIG_FILE=${pkgs.makeFontsConf { fontDirectories = [ ]; }}
        export WLR_BACKENDS=headless WLR_RENDERER=pixman WLR_LIBINPUT_NO_DEVICES=1
        mkdir -p config/zathura data run
        chmod 700 run
        cp ${home.xdg.configFile."zathura/zathurarc".source} config/zathura/zathurarc
        cp ${pages} document.pdf
        dbus-run-session --config-file=${pkgs.dbus}/share/dbus-1/session.conf -- \
          cage -- $BASH ${./open-now-test.sh} document.pdf ${lib.escapeShellArgs source.opened} \
          > ran 2>&1 || true
        if [ ! -e passed ]; then cat ran; exit 1; fi
        touch $out
      '';

  # zathura says each file it's started with to the desktop's list of files
  # opened lately, through recent-files where the desktop has it, stood in
  # here, handing on all it was given, and then runs as it would.
  zathura-says-the-files-it-opens =
    let
      home = account graphical;
      recent-files = pkgs.writeShellScriptBin "recent-files" ''echo "$*" >> "$SAID"'';
    in
    pkgs.runCommandLocal "zathura-says-the-files-it-opens"
      {
        nativeBuildInputs = [
          home.programs.zathura.package
          recent-files
        ];
      }
      ''
        export HOME=$PWD SAID=$PWD/said
        zathura --version one.pdf > version 2>&1 || { cat version; exit 1; }
        grep -q -i zathura version || { echo "zathura didn't run: $(cat version)"; exit 1; }
        told=$(if [ -e said ]; then cat said; else echo 'nothing said'; fi)
        [ "$told" = "add zathura --version one.pdf" ] || { echo "zathura said: $told"; exit 1; }
        touch $out
      '';
}
