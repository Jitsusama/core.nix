# The checks of mpv, which checks.nix gathers with every other folder's;
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
  # mpv plays a moment of video and sound with the account's settings and
  # those of its controller, console, right-click menu and statistics,
  # saying nothing about any: mpv names an option it doesn't know and goes
  # on without it, and each script does the same for a key, so a setting a
  # new release drops would vanish without a word. Its messages and its
  # statistics are in the monospaced face, since they're names and
  # figures, and its subtitles in the serif upright's cut, words to read,
  # which Joel's never wants in a sans. A video file opens in
  # mpv.
  mpv-accepts-its-configuration =
    let
      home = account graphical;
      mpv = lib.findFirst (package: lib.hasPrefix "mpv" (lib.getName package)) null home.home.packages;
      files = home.xdg.configFile;
      opens = graphical.config.xdg.mime.defaultApplications."video/mp4" or "";
      mono = home.jitsusama.theme.font.mono.family;
      words = home.jitsusama.theme.font.serif.text.family;
      scripts = [
        "osc"
        "console"
        "context_menu"
        "stats"
      ];
    in
    assert opens == "mpv.desktop";
    pkgs.runCommandLocal "mpv-accepts-its-configuration" { nativeBuildInputs = [ mpv ]; } ''
      export HOME=$PWD FONTCONFIG_FILE=${pkgs.makeFontsConf { fontDirectories = [ ]; }}
      mkdir -p mpv/script-opts
      cp ${files."mpv/mpv.conf".source} mpv/mpv.conf
      ${lib.concatMapStrings (script: ''
        cp ${files."mpv/script-opts/${script}.conf".source} mpv/script-opts/${script}.conf
      '') scripts}
      # A string's length can lead it, as %21%, as mpv takes it.
      grep -qE '^osd-font=(%[0-9]+%)?${mono}$' mpv/mpv.conf \
        || { echo "mpv's messages aren't in ${mono}:"; grep font mpv/mpv.conf; exit 1; }
      grep -qE '^font=(%[0-9]+%)?${mono}$' mpv/script-opts/stats.conf \
        || { echo "mpv's statistics aren't in ${mono}:"; cat mpv/script-opts/stats.conf; exit 1; }
      grep -qE '^sub-font=(%[0-9]+%)?${words}$' mpv/mpv.conf \
        || { echo "mpv's subtitles aren't in ${words}:"; grep font mpv/mpv.conf; exit 1; }
      # Kept open, mpv would hold the last frame and never finish.
      mpv --config-dir=$PWD/mpv --vo=null --ao=null --force-window=no --keep-open=no \
        --save-position-on-quit=no --frames=10 \
        --script-opts-append=osc-visibility=always \
        'av://lavfi:testsrc=duration=1:size=64x64' > said 2>&1 || { cat said; exit 1; }
      if grep -i 'error\|unknown\|not found\|invalid' said; then exit 1; fi
      touch $out
    '';

  # mpv's keys, as the account's input.conf and vi-keys.lua bind them, do
  # what the launcher says they do (keys-test.sh): each arrow lands where
  # its letter does, with a number before them that many times, 0 and $
  # go to the start and the end, gg, G and ]b and [b go through the list,
  # J and L do what j and l did, yy copies the frame as a PNG, through
  # wl-copy, ? and F1 ask the launcher for the keys, both stood in here,
  # Space plays on at once, even before a ?, and ZZ and Space w close it
  # where q, Q and Escape leave it open. A program that runs mpv with
  # Escape of its own, forced, as the camera's viewfinder does, always
  # gets it.
  mpv-keys-go-where-they-say =
    let
      home = account graphical;
      mpv = lib.findFirst (package: lib.hasPrefix "mpv" (lib.getName package)) null home.home.packages;
      files = home.xdg.configFile;
      # The wl-copy copy-frame's options name, for the stand-in to take its
      # place.
      wl-copy = lib.getExe' pkgs.wl-clipboard "wl-copy";
      # The launcher, which only has to say what it was asked.
      launcher = pkgs.writeShellScriptBin "launcher-say" ''echo "$*" >> "$ASKED"'';
    in
    pkgs.runCommandLocal "mpv-keys-go-where-they-say"
      {
        nativeBuildInputs = [
          mpv
          pkgs.ffmpeg-headless
          launcher
          pkgs.jq
          pkgs.socat
        ];
      }
      ''
        export HOME=$PWD XDG_RUNTIME_DIR=$PWD/run ASKED=$PWD/asked
        export FONTCONFIG_FILE=${pkgs.makeFontsConf { fontDirectories = [ ]; }}
        mkdir -p mpv/scripts mpv/script-opts run
        cp ${files."mpv/mpv.conf".source} mpv/mpv.conf
        cp ${files."mpv/input.conf".source} mpv/input.conf
        cp ${files."mpv/scripts/copy-frame.lua".source} mpv/scripts/copy-frame.lua
        cp ${files."mpv/scripts/vi-keys.lua".source} mpv/scripts/vi-keys.lua
        printf '#!%s\ncat > %s/copied\n' "$BASH" "$PWD" > wl-copy
        chmod +x wl-copy
        sed 's|${wl-copy}|'"$PWD"'/wl-copy|' ${files."mpv/script-opts/copy-frame.conf".source} \
          > mpv/script-opts/copy-frame.conf
        grep -q "$PWD/wl-copy" mpv/script-opts/copy-frame.conf \
          || { echo "copy-frame names no wl-copy to copy with"; exit 1; }
        ffmpeg -loglevel error -f lavfi -i testsrc=duration=200:size=32x32:rate=1 -g 1 clip.mkv
        for subtitles in one two; do
          printf '1\n00:00:00,000 --> 00:03:20,000\n%s\n' $subtitles > $subtitles.srt
        done
        $BASH ${./keys-test.sh} $PWD/mpv clip.mkv one.srt two.srt > ran 2>&1 || true
        if [ ! -e passed ]; then cat ran; exit 1; fi
        touch $out
      '';

  # mpv says each file it plays to the desktop's list of files opened
  # lately, through recent-files where the desktop has it, stood in here,
  # each by its whole path, as mpv gives it, and a stream too, which
  # recent-files passes over.
  mpv-says-the-files-it-plays =
    let
      home = account graphical;
      mpv = lib.findFirst (package: lib.hasPrefix "mpv" (lib.getName package)) null home.home.packages;
      # It keeps what it was asked, so a call it refuses reads as it was.
      recent-files = pkgs.writeShellScriptBin "recent-files" ''
        [ "$1 $2" = "add mpv" ] || { echo "recent-files $*" >> "$SAID"; exit 1; }
        shift 2
        printf '%s\n' "$@" >> "$SAID"
      '';
      stream = "av://lavfi:testsrc=duration=1:size=64x64";
    in
    pkgs.runCommandLocal "mpv-says-the-files-it-plays"
      {
        nativeBuildInputs = [
          mpv
          recent-files
          pkgs.ffmpeg-headless
        ];
      }
      ''
        export HOME=$PWD SAID=$PWD/said FONTCONFIG_FILE=${pkgs.makeFontsConf { fontDirectories = [ ]; }}
        mkdir -p mpv/scripts
        cp ${home.xdg.configFile."mpv/scripts/recent-files.lua".source} mpv/scripts/recent-files.lua
        for clip in one two; do
          ffmpeg -loglevel error -f lavfi -i testsrc=duration=1:size=64x64 $clip.mkv
        done
        mpv --config-dir=$PWD/mpv --vo=null --ao=null --force-window=no --keep-open=no \
          --frames=2 one.mkv '${stream}' two.mkv > played 2>&1 || { cat played; exit 1; }
        told=$( [ -e said ] && tr '\n' ' ' < said || echo 'nothing said')
        [ "$told" = "$PWD/one.mkv ${stream} $PWD/two.mkv " ] || { echo "mpv said: $told"; exit 1; }
        touch $out
      '';

  # mpv keeps the file it plays now for its Recent in the launcher
  # (playing.lua), read by the launcher's own command: while a file plays
  # it reads as mpv's process id and the file's whole path, given as a
  # name in the folder mpv was started in; what an mpv that was killed
  # left is cleared as one starts; a stream isn't kept, so the line goes
  # as one follows the file; and nothing is left once mpv quits.
  mpv-says-the-file-it-plays-now =
    let
      home = account graphical;
      mpv = lib.findFirst (package: lib.hasPrefix "mpv" (lib.getName package)) null home.home.packages;
      kept = "${home.xdg.stateHome}/mpv/playing";
      inherit (home.jitsusama.quickshell.launcher) recents;
      source = lib.findFirst (each: each.app or "" == "mpv") null recents;
      fonts = pkgs.makeFontsConf { fontDirectories = [ ]; };
      # The launcher's command, reading the folder this check keeps.
      opened = lib.concatMapStringsSep " " (
        part: if part == kept then ''"$XDG_STATE_HOME/mpv/playing"'' else lib.escapeShellArg part
      ) source.opened;
      stream = "av://lavfi:testsrc=duration=1:size=64x64";
    in
    assert lib.elem kept source.opened;
    pkgs.runCommandLocal "mpv-says-the-file-it-plays-now"
      {
        nativeBuildInputs = [
          mpv
          pkgs.ffmpeg-headless
        ];
      }
      ''
        export HOME=$PWD XDG_STATE_HOME=$PWD/state FONTCONFIG_FILE=${fonts}
        mkdir -p mpv/scripts state/mpv/playing
        cp ${home.xdg.configFile."mpv/scripts/playing.lua".source} mpv/scripts/playing.lua
        ffmpeg -loglevel error -f lavfi -i testsrc=duration=5:size=64x64 one.mkv
        printf '999999\t/gone.mkv\n' > state/mpv/playing/999999
        : > state/mpv/playing/.999999
        mpv --config-dir=$PWD/mpv --vo=null --ao=null --force-window=no --keep-open=yes \
          one.mkv '${stream}' > played 2>&1 &
        playing=$!
        read_now() { ${opened} 2> /dev/null; }
        want="$(printf '%s\t%s' "$playing" "$PWD/one.mkv")"
        # Up to a minute each, so a busy machine can't fail it.
        for _ in $(seq 600); do
          [ "$(read_now)" = "$want" ] && break
          sleep 0.1
        done
        if [ "$(read_now)" != "$want" ]; then
          kill $playing; cat played
          echo "while mpv played one.mkv, mpv's opened read: $(read_now)"
          exit 1
        fi
        if [ -n "$(ls -A state/mpv/playing | grep -v -x "$playing")" ]; then
          kill $playing
          echo "what a killed mpv left stayed: $(ls -A state/mpv/playing)"
          exit 1
        fi
        for _ in $(seq 600); do
          [ -z "$(read_now)" ] && break
          sleep 0.1
        done
        if [ -n "$(read_now)" ]; then
          kill $playing
          echo "with a stream playing, mpv's opened still read: $(read_now)"
          exit 1
        fi
        kill -TERM $playing
        wait $playing || true
        if [ -n "$(ls -A state/mpv/playing)" ]; then
          echo "mpv left this as it quit: $(ls -A state/mpv/playing)"
          exit 1
        fi
        touch $out
      '';
}
