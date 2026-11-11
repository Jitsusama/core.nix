# The checks of mpv, which checks.nix gathers with every other folder's;
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
let
  # A stand-in for one of the shell's singletons, holding only what a check
  # needs of it.
  singleton =
    name: body:
    pkgs.writeText name ''
      pragma Singleton
      import Quickshell
      import QtQuick
      Singleton { ${body} }
    '';
in
onLinux {
  # mpv's step in the launcher, its provider's as Applications.qml holds it
  # (tests/mpv-program.qml), over mpv's desktop entry, with niri, the
  # shortcuts and what's playing stood in, the shell's own watch of the
  # clipboard over a wl-paste that says nothing, and mpv-stopped by a
  # script that counts its runs: its Recent merges each file mpv left by
  # when, newest first, with the shared list's, each once, none open now;
  # one stopped part way says "12:04 of 41:10 · 12 min ago" beside it and
  # Return resumes it, one watched whole says its length and when and
  # play; each drawer says where it is and its kind, what's still to play,
  # what mpv kept, its size and its path; each listed once mpv is stepped
  # into, once an opening; and Play the Copied Link only while the copy is
  # one address mpv plays, whose cases Mpv.js's own are, run first in Qt's
  # engine.
  the-launcher-plays-on-where-mpv-stopped =
    let
      home = account graphical;
      qs = lib.getExe' graphical.pkgs.quickshell "qs";
      # What each program had open lately is the programs daemon's.
      programs = pkgs.callPackage ../programs/package.nix { };
      # The launcher's own files the step is made from, each by itself so
      # no other change there builds this again.
      launcher = ../quickshell/launcher;
      launched = [
        "Provider.qml"
        "Asking.js"
        "Numbers.js"
        "Applications.qml"
        "ReadyRows.qml"
        "ReadyRows.js"
        "Interfaces.js"
        "HeldWindows.js"
        "Order.js"
        "Program.js"
        "Media.js"
        "Recents.qml"
        "Listed.qml"
        "Recents.js"
        "Recent.js"
        "Captured.js"
        "Ago.js"
        "words.json"
      ];
      # An mpv window, its process playing now.mkv.
      niri = singleton "Niri.qml" ''
        property int revision: 0
        property var windows: ({
          "21": { id: 21, app_id: "mpv", pid: 5151, title: "now.mkv - mpv", workspace_id: 1,
            focus_timestamp: { secs: 200, nanos: 0 } }
        })
        property var workspaces: ({
          "1": { id: 1, idx: 1, is_focused: true, active_window_id: 21 }
        })
        readonly property int current: 21
        property var acted: []
        property var spawned: []
        function workspace(id) { return workspaces[id ?? "1"] ?? null; }
        function act(action) { acted = acted.concat([action]); }
        function spawn(command) { spawned = command; }
      '';
      playing = singleton "Playing.qml" ''
        property var players: []
        property var streams: []
        property var recorders: []
      '';
      entry = pkgs.writeText "mpv.desktop" ''
        [Desktop Entry]
        Type=Application
        Name=mpv Media Player
        GenericName=Multimedia player
        Exec=mpv --player-operation-mode=pseudo-gui -- %U
      '';
      # The clipboard's watcher, which hears nothing, so the case sets
      # each copy itself.
      paste = pkgs.writeShellScriptBin "wl-paste" "exec sleep 300";
    in
    pkgs.runCommandLocal "the-launcher-plays-on-where-mpv-stopped" { } ''
      mkdir -p launcher share/applications
      cp ${entry} share/applications/mpv.desktop
      cp ${home.xdg.configFile."quickshell/Theme.qml".source} Theme.qml
      cp ${../quickshell/Fuzzy.js} Fuzzy.js
      cp ${../quickshell/ProgramGlyphs.js} ProgramGlyphs.js
      cp ${../quickshell/Music.js} Music.js
      cp ${../quickshell/Words.qml} Words.qml
      cp ${../quickshell/Files.qml} Files.qml
      cp ${../quickshell/Daemon.qml} Daemon.qml
      cp ${../quickshell/Hearing.js} Hearing.js
      cp ${../quickshell/FileFacts.js} FileFacts.js
      cp ${../quickshell/When.js} When.js
      ${lib.concatMapStrings (file: ''
        cp ${launcher + "/${file}"} launcher/${file}
      '') launched}
      cp ${./MpvProgram.qml} launcher/MpvProgram.qml
      cp ${./Mpv.js} launcher/Mpv.js
      cp ${../../tests/mpv-program.qml} shell.qml
      cp ${../../tests/lib/HeardWindows.js} HeardWindows.js
      cp ${niri} Niri.qml
      cp ${playing} launcher/Playing.qml
      cp ${singleton "Shortcuts.qml" "property var actions: []"} Shortcuts.qml
      cp ${../quickshell/Copied.qml} Copied.qml
      cp ${../quickshell/Copied.js} Copied.js
      export PATH=${paste}/bin:$PATH
      export HOME=$TMPDIR XDG_RUNTIME_DIR=$TMPDIR/runtime LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen
      export XDG_DATA_HOME=$TMPDIR/data XDG_DATA_DIRS=$PWD/share NOW=$(date +%s)
      mkdir -m 700 $XDG_RUNTIME_DIR
      export FONTCONFIG_FILE=${pkgs.makeFontsConf { fontDirectories = [ ]; }}
      # The list programs share, as recent-files prints it: what plays now,
      # a file stopped part way through, one played to its end, and one mpv
      # kept nothing of.
      printf 'mpv\t%s\n' "$HOME/Videos/now.mkv" "$HOME/Videos/talk.mkv" "$HOME/Videos/song.mkv" \
        "$HOME/Videos/clip.mkv" > $HOME/shared
      export SHARED_LIST=${pkgs.writeShellScript "shared-list" ''cat "$HOME/shared"''}
      # What each mpv plays now, as playing.lua keeps it.
      export PLAYING_LIST=${pkgs.writeShellScript "playing-list" ''
        printf '5151\t%s\n' "$HOME/Videos/now.mkv"
      ''}
      # What mpv-stopped prints, newest first: what plays now, two files
      # stopped part way, the second kept played faster, louder, on its
      # second sound and without its words, one of sound alone watched
      # whole two days ago, and a line it never would, counting each run.
      export STOPPED_LIST=${pkgs.writeShellScript "stopped-list" ''
        echo $(( $(cat "$HOME/listed" 2> /dev/null || echo 0) + 1 )) > "$HOME/listed"
        printf '%s\t%s\t%s\t%s\t%s\t%s\n' \
          "$HOME/Videos/now.mkv" 30 600 $((NOW - 60)) 4000 video \
          "$HOME/Videos/long.mkv" 3723 6000 $((NOW - 600)) 5000000 video
        printf '%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\n' \
          "$HOME/Videos/talk.mkv" 724.5 2470.2 $((NOW - 720)) 1234567890 video \
          speed=1.500000 volume=80.000000 aid=2 sid=no sub-delay=0.500000
        printf '%s\t%s\t%s\t%s\t%s\t%s\t%s\n' \
          "$HOME/Videos/song.mkv" 2469.7 2470.2 $((NOW - 172800)) 940000 audio volume=50.000000
        printf '%s\t%s\t%s\t%s\t%s\t%s\n' "Videos/half.mkv" 1 2 3 4 video
      ''}
      # The programs daemon runs the shared list, what mpv plays and the
      # files it left, given the sources shell.qml's Applications is and
      # the list mpv's page asks for, as recents.nix gives them.
      cat > sources.json <<EOF
      [{"watch": "$HOME/shared", "list": ["$SHARED_LIST"]},
       {"app": "mpv", "kind": "file", "opened": ["$PLAYING_LIST"]},
       {"app": "mpv", "name": "stopped", "asked": true, "list": ["$STOPPED_LIST"]}]
      EOF
      ${lib.getExe programs} "$XDG_RUNTIME_DIR/daemons/programs.sock" sources.json &
      ${shellFolderLint}
      timeout 60 ${qs} -p shell.qml
      touch $out
    '';

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
  # Space plays on at once, even before a ?, and ZZ closes it where q, Q,
  # Escape and Space w leave it open. : asks the launcher for its command
  # line, and what mpv-command carries back does what it says. A program
  # that runs mpv with Escape of its own, forced, as the camera's
  # viewfinder does, always gets it.
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
          (pkgs.callPackage ./command.nix { })
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

  # The launcher's lister of where mpv stopped (stopped.sh), run as the
  # programs daemon runs it for its provider, reads what the account's mpv kept of clips it quit
  # part way through or at their end, as ZZ quits (stopped-test.sh): each
  # file it played on and the one held on its last frame, newest first, by
  # its whole path, where it stopped, its length, when it was left, its
  # size, whether it's a video or audio, and what mpv kept of it besides,
  # its speed, volume and tracks among them, only the last read as watched
  # whole, but no folder's redirect entry, no stream and no file deleted
  # since, nine at most; and a
  # file started as Resume starts it plays on from where it stopped, and
  # as Play From the Start does, from its start. mpv and the lister are
  # given one folder.
  mpv-says-where-it-stopped =
    let
      home = account graphical;
      mpv = lib.findFirst (package: lib.hasPrefix "mpv" (lib.getName package)) null home.home.packages;
      inherit (home.jitsusama.quickshell.launcher) recents;
      listing = (lib.findFirst (each: each.name or "" == "stopped") null recents).list;
    in
    assert lib.last listing == home.programs.mpv.config.watch-later-dir;
    pkgs.runCommandLocal "mpv-says-where-it-stopped"
      {
        nativeBuildInputs = [
          mpv
          pkgs.ffmpeg-headless
          pkgs.jq
          pkgs.socat
        ];
      }
      ''
        export HOME=$PWD FONTCONFIG_FILE=${pkgs.makeFontsConf { fontDirectories = [ ]; }}
        mkdir -p mpv
        cp ${home.xdg.configFile."mpv/mpv.conf".source} mpv/mpv.conf
        $BASH ${./stopped-test.sh} $PWD/mpv ${lib.head listing} > ran 2>&1 || true
        if [ ! -e passed ]; then cat ran; exit 1; fi
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
