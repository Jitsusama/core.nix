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
}
