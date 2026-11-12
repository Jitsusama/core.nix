# The checks of swayimg, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  account,
  graphical,
  lib,
  onLinux,
  pkgs,
  shellFolderLint,
  ...
}:
onLinux {
  # swayimg's page in the launcher always leads with something to look
  # through while there's anything to: the screenshots' gallery while it
  # holds any, else a gallery of the newest folder of swayimg's Recent, and
  # one of each folder its Recent's pictures are in, newest first, each
  # once, saying how many pictures it holds and where, and opening on the
  # newest of Recent's pictures still there, each gallery's drawer its
  # folder's plate with its newest picture previewed, named, and when it
  # was taken or added; and its drawer says when the newest screenshot was
  # taken and how many there are, as tests/swayimg-galleries.qml holds
  # Swayimg.js to it in Qt's own engine.
  swayimg-leads-with-a-gallery =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    # Kept in Nix because each line copies a file Nix gives it beside the
    # cases, or sets what Qt's engine reads them with.
    pkgs.runCommandLocal "swayimg-leads-with-a-gallery" { } ''
      mkdir launcher
      cp ${../../tests/swayimg-galleries.qml} swayimg-galleries.qml
      cp ${./Swayimg.js} launcher/Swayimg.js
      cp ${../quickshell/FileFacts.js} FileFacts.js
      cp ${../quickshell/When.js} When.js
      cp ${(import ../quickshell/launcher/words.nix).file} words.json
      export QML_XHR_ALLOW_FILE_READ=1
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${pkgs.makeFontsConf { fontDirectories = [ ]; }}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml swayimg-galleries.qml
      touch $out
    '';

  # swayimg shows a picture with the account's settings, on a screen of
  # its own, and quits once it's drawn. A setting it can't take stops the
  # rest of its settings from being read, said only as it starts, so
  # anything it says fails this. It has the machine's fonts, since with
  # none at all it would say it found no face. Its words over a picture
  # are in the monospaced face, a file's name and its figures, as a
  # launcher row's name is. Pictures open in swayimg.
  swayimg-accepts-its-configuration =
    let
      home = account graphical;
      files = home.xdg.configFile;
      opens = graphical.config.xdg.mime.defaultApplications."image/png" or "";
      fonts = pkgs.makeFontsConf { fontDirectories = graphical.config.fonts.packages; };
      mono = home.jitsusama.theme.font.mono.family;
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
        # swayimg can't say what face it was given, so its settings do.
        grep -qxF 'swayimg.text.font = nix.mono' config/swayimg/init.lua \
          && grep -qE '(\["mono"\]|mono) = "${mono}"' config/swayimg/nix.lua \
          || { echo "swayimg's words aren't in ${mono}:"; grep mono config/swayimg/*.lua; exit 1; }
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
  # than its window, which is how Joel was left unable to go on. Neovim's
  # keys do as the arrows they twin, a number before one makes it that
  # many, dd and Delete move a picture to the trash and u brings it back,
  # yy copies it, through wl-copy, stood in here, and ? and Space ? ask the
  # launcher for the keys, stood in too, on a picture and in the gallery.
  # q and Escape close nothing, and ZZ and Space w close it. What the
  # launcher's command line asks, as swayimg-command carries it, does what
  # it says: a number, :d, :rotate, :y, :open and :q.
  # The picture it opened with, and only that one, is said to the desktop's
  # list of files opened lately, through recent-files, stood in as well.
  # The launcher's gallery, as its provider opens it on a picture from
  # Recent, opens on that picture, newest first.
  swayimg-moves-by-its-keys =
    let
      home = account graphical;
      files = home.xdg.configFile;
      inherit (home.jitsusama.quickshell.launcher.providers.swayimg) gallery;
      fonts = pkgs.makeFontsConf { fontDirectories = graphical.config.fonts.packages; };
      recent-files = pkgs.writeShellScriptBin "recent-files" ''echo "$*" >> "$SAID"'';
      # The launcher, which only has to say what it was asked.
      launcher = pkgs.writeShellScriptBin "launcher-say" ''echo "$*" >> "$ASKED"'';
      # The wl-copy nix.lua names, for the stand-in to take its place.
      wl-copy = lib.getExe' pkgs.wl-clipboard "wl-copy";
      # niri, which only has to say what it was asked to open, and a
      # program for :open to name.
      niri = pkgs.writeShellScriptBin "niri" ''echo "$*" >> "$SPAWNED"'';
      other-editor = pkgs.writeShellScriptBin "other-editor" "true";
    in
    pkgs.runCommandLocal "swayimg-moves-by-its-keys"
      {
        nativeBuildInputs = [
          home.programs.swayimg.package
          pkgs.bash
          pkgs.cage
          pkgs.grim
          pkgs.inotify-tools
          pkgs.wtype
          launcher
          niri
          other-editor
          recent-files
          (pkgs.callPackage ./command.nix { })
        ];
      }
      ''
        export HOME=$PWD XDG_CONFIG_HOME=$PWD/config XDG_RUNTIME_DIR=$PWD/run SAID=$PWD/said-files
        export ASKED=$PWD/asked SPAWNED=$PWD/spawned
        export FONTCONFIG_FILE=${fonts} WLR_BACKENDS=headless WLR_RENDERER=pixman
        export WLR_LIBINPUT_NO_DEVICES=1
        mkdir -p config/swayimg run pictures
        chmod 700 run
        cp ${files."swayimg/init.lua".source} config/swayimg/init.lua
        printf '#!%s\ncat > %s/copied\n' "$BASH" "$PWD" > wl-copy
        chmod +x wl-copy
        sed 's|${wl-copy}|'"$PWD"'/wl-copy|' ${files."swayimg/nix.lua".source} \
          > config/swayimg/nix.lua
        grep -q "$PWD/wl-copy" config/swayimg/nix.lua \
          || { echo "nix.lua names no wl-copy to copy with"; exit 1; }
        for name in a b c d e f g h i j k l m n; do
          printf 'P3\n1 1\n255\n0 0 0\n' > pictures/$name.ppm
        done
        # The launcher's gallery, a word a line, as the script reads it.
        printf '%s\n' ${lib.escapeShellArgs gallery} > gallery
        { printf 'P5\n4 2000\n255\n'; head -c 8000 /dev/zero; } > pictures/z-tall.pgm
        timeout 180 cage -- bash ${./tests/swayimg-moves-by-its-keys.sh} || true
        test -e moved
        touch $out
      '';

  # swayimg keeps the picture it shows, read by the command its Recent in
  # the launcher is led by as its process and the whole path of a picture
  # given by name in the folder it was started in, and nothing once ZZ
  # closes it (../../tests/lib/says-what-it-has-open.sh). What a killed swayimg
  # left is cleared as it starts.
  swayimg-says-the-picture-it-shows =
    let
      home = account graphical;
      files = home.xdg.configFile;
      fonts = pkgs.makeFontsConf { fontDirectories = graphical.config.fonts.packages; };
      kept = "${home.xdg.stateHome}/swayimg/viewing";
      inherit (home.jitsusama.quickshell.launcher) recents;
      source = lib.findFirst (each: each.app or "" == "swayimg") null recents;
      # The launcher's command, reading the folder this check keeps.
      opened = lib.concatMapStringsSep " " (
        part: if part == kept then ''"$XDG_STATE_HOME/swayimg/viewing"'' else lib.escapeShellArg part
      ) source.opened;
    in
    assert lib.elem kept source.opened;
    pkgs.runCommandLocal "swayimg-says-the-picture-it-shows"
      {
        nativeBuildInputs = [
          home.programs.swayimg.package
          pkgs.cage
          pkgs.wtype
        ];
      }
      ''
        export HOME=$PWD XDG_CONFIG_HOME=$PWD/config XDG_RUNTIME_DIR=$PWD/run
        export XDG_STATE_HOME=$PWD/state
        export FONTCONFIG_FILE=${fonts} WLR_BACKENDS=headless WLR_RENDERER=pixman
        export WLR_LIBINPUT_NO_DEVICES=1
        mkdir -p config/swayimg run state/swayimg/viewing
        chmod 700 run
        cp ${files."swayimg/init.lua".source} config/swayimg/init.lua
        sed 's|${kept}|'"$XDG_STATE_HOME"'/swayimg/viewing|' ${files."swayimg/nix.lua".source} \
          > config/swayimg/nix.lua
        grep -q "$XDG_STATE_HOME/swayimg/viewing" config/swayimg/nix.lua \
          || { echo "nix.lua names no folder it keeps the picture in"; exit 1; }
        printf 'P3\n1 1\n255\n0 0 0\n' > picture.ppm
        printf '999999\t/gone.ppm\n' > state/swayimg/viewing/999999
        : > state/swayimg/viewing/.999999
        timeout 300 cage -- $BASH ${../../tests/lib/says-what-it-has-open.sh} \
          --closed-by '-M shift zz -m shift' picture.ppm swayimg -- ${opened} > ran 2>&1 || true
        if [ ! -e passed ]; then cat ran; exit 1; fi
        if [ -n "$(ls -A state/swayimg/viewing)" ]; then
          echo "swayimg left this: $(ls -A state/swayimg/viewing)"
          exit 1
        fi
        touch $out
      '';
}
