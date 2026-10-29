# The checks of swayimg, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  account,
  graphical,
  onLinux,
  pkgs,
  ...
}:
onLinux {
  # swayimg shows a picture with the account's settings, on a screen of
  # its own, and quits once it's drawn. A setting it can't take stops the
  # rest of its settings from being read, said only as it starts, so
  # anything it says fails this. It has the machine's fonts, since with
  # none at all it would say it found no face. Pictures open in swayimg.
  swayimg-accepts-its-configuration =
    let
      home = account graphical;
      files = home.xdg.configFile;
      opens = graphical.config.xdg.mime.defaultApplications."image/png" or "";
      fonts = pkgs.makeFontsConf { fontDirectories = graphical.config.fonts.packages; };
    in
    assert opens == "swayimg.desktop";
    pkgs.runCommandLocal "swayimg-accepts-its-configuration"
      {
        nativeBuildInputs = [
          home.programs.swayimg.package
          pkgs.cage
        ];
      }
      ''
        export HOME=$PWD XDG_CONFIG_HOME=$PWD/config XDG_RUNTIME_DIR=$PWD/run
        export FONTCONFIG_FILE=${fonts} WLR_BACKENDS=headless WLR_RENDERER=pixman
        export WLR_LIBINPUT_NO_DEVICES=1
        mkdir -p config/swayimg run
        chmod 700 run
        cp ${files."swayimg/init.lua".source} config/swayimg/init.lua
        cp ${files."swayimg/nix.lua".source} config/swayimg/nix.lua
        printf 'P3\n1 1\n255\n0 0 0\n' > picture.ppm
        timeout 30 cage -- swayimg \
          --execute='swayimg.on_redrawn(function() swayimg.exit(0) end)' picture.ppm \
          > said 2>&1 || { cat said; exit 1; }
        # cage's own lines begin with the time.
        if grep -v '^[0-9][0-9]:' said | grep -i -E 'error|warning|deprecated|unable'; then
          exit 1
        fi
        touch $out
      '';

  # The arrows go to the previous and the next picture, and Home and End to
  # the first and the last, pressed as a keyboard presses them, with the
  # account's settings: swayimg's own arrows only move a picture larger
  # than its window, which is how Joel was left unable to go on. The
  # picture it opened with, and only that one, is said to the desktop's
  # list of files opened lately, through recent-files, stood in here.
  swayimg-moves-by-its-keys =
    let
      home = account graphical;
      files = home.xdg.configFile;
      fonts = pkgs.makeFontsConf { fontDirectories = graphical.config.fonts.packages; };
      recent-files = pkgs.writeShellScriptBin "recent-files" ''echo "$*" >> "$SAID"'';
    in
    pkgs.runCommandLocal "swayimg-moves-by-its-keys"
      {
        nativeBuildInputs = [
          home.programs.swayimg.package
          pkgs.bash
          pkgs.cage
          pkgs.inotify-tools
          pkgs.wtype
          recent-files
        ];
      }
      ''
        export HOME=$PWD XDG_CONFIG_HOME=$PWD/config XDG_RUNTIME_DIR=$PWD/run SAID=$PWD/said-files
        export FONTCONFIG_FILE=${fonts} WLR_BACKENDS=headless WLR_RENDERER=pixman
        export WLR_LIBINPUT_NO_DEVICES=1
        mkdir -p config/swayimg run
        chmod 700 run
        cp ${files."swayimg/init.lua".source} config/swayimg/init.lua
        cp ${files."swayimg/nix.lua".source} config/swayimg/nix.lua
        for name in a b c; do
          printf 'P3\n1 1\n255\n0 0 0\n' > $name.ppm
        done
        timeout 60 cage -- bash ${./tests/swayimg-moves-by-its-keys.sh} || true
        test -e moved
        touch $out
      '';
}
