# The checks of the reader, which checks.nix gathers with every other
# folder's; its opening comment says what each kind of check proves.
{
  account,
  graphical,
  lib,
  onLinux,
  passesClippy,
  pkgs,
  shellFolderLint,
  ...
}:
let
  home = account graphical;
  reader = lib.findFirst (package: lib.getName package == "reader") null home.home.packages;
  fonts = pkgs.makeFontsConf { fontDirectories = graphical.config.fonts.packages; };

  # A compositor of nobody's, drawn in software, with the account's theme
  # file where the reader looks for it.
  screen = ''
    export HOME=$PWD XDG_CONFIG_HOME=$PWD/config XDG_RUNTIME_DIR=$PWD/run
    export XDG_STATE_HOME=$PWD/state XDG_CACHE_HOME=$PWD/cache FONTCONFIG_FILE=${fonts}
    export WLR_BACKENDS=headless WLR_RENDERER=pixman WLR_LIBINPUT_NO_DEVICES=1
    export LIBGL_ALWAYS_SOFTWARE=1
    export __EGL_VENDOR_LIBRARY_DIRS=${pkgs.mesa}/share/glvnd/egl_vendor.d
    mkdir -p config/reader run state cache
    chmod 700 run
    cp ${home.xdg.configFile."reader/theme.json".source} config/reader/theme.json
  '';

  # The colour as ImageMagick names a pixel's.
  srgb =
    colour:
    let
      hex = lib.removePrefix "#" colour;
      channel = index: toString (lib.fromHexString (builtins.substring (index * 2) 2 hex));
    in
    "srgb(${channel 0},${channel 1},${channel 2})";

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
# The reader draws through Wayland and EGL, on the Linux desktop.
onLinux {
  # The reader's tests run as it builds: a table's columns from the words
  # in them, every line of a table and of records on the text's pitch, a
  # picture known by its first bytes, a place kept as a block and a way into
  # it and read back as it was, with the heading it sits under, how far down
  # the page it is and when it was left, a place kept before those still
  # read, the places listed newest left first without a file gone, a mark
  # asked for by its letter opened at, a save heard whether written in place or
  # moved over and another file's not, and the new file's once the watch
  # moves, a link to markdown set in place, a label's letters on its link's
  # baseline within a pixel, the file shown said for the launcher
  # and taken away on closing, the theme's roles read with Osaka Jade for
  # the rest, search found across a hyphen's break, an equation shown as
  # written when MiTeX is missing, counts and keys as Neovim takes them,
  # visual mode's motions and text objects over a page's lines, each line
  # the piece of the file it was written in, and what a selection copies,
  # the file's own markdown, where vi's paragraph, heading, view, bracket
  # and file moves land, a whole word searched for, a command line read as
  # Neovim reads it, gO's outline chosen, moved and searched through, the
  # motion's curve, and a worker let go only once its job in hand and its
  # thread have ended, the rest skipped.
  reader-sets-its-pages = pkgs.callPackage ./package.nix { };
  # The reader's step in the launcher, its provider's as Applications.qml
  # holds it (../../tests/reader-program.qml), over the reader's desktop
  # entry, with niri, the shortcuts and what's playing stood in, the
  # shell's own watch of the clipboard over a wl-paste that says nothing,
  # and reader --places by a script that counts its runs: its Recent merges
  # each file the reader left by when, newest first, with the shared list's,
  # each once, none open now; one left part way says "Installing · 40% · 12
  # min ago" beside it and Return reads on, with a row for each mark by its
  # letter reading from there, and one read to its end says so; each drawer
  # says where it is and its kind, the heading, how far down, each mark
  # and its path, and the reader's what's still to finish; each listed
  # once the reader is stepped into, once an opening; with Reader.js's own
  # cases run first in Qt's engine.
  the-launcher-reads-on-where-the-reader-left =
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
        "BrowserWindows.js"
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
      # A reader's window, its process showing now.md. Kept in Nix because
      # it's a stand-in's few properties, as each program's check in the
      # launcher writes its niri.
      niri = singleton "Niri.qml" ''
        property int revision: 0
        property var windows: ({
          "21": { id: 21, app_id: "reader", pid: 5151, title: "now.md", workspace_id: 1,
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
      entry = pkgs.writeText "reader.desktop" ''
        [Desktop Entry]
        Type=Application
        Name=Reader
        Comment=Read a markdown file as a page
        Exec=reader %f
      '';
      # The clipboard's watcher, which hears nothing.
      paste = pkgs.writeShellScriptBin "wl-paste" "exec sleep 300";
    in
    # Kept in Nix because each line copies, or names in a variable, a file
    # Nix gives it, and the places it reads are printed by a script of
    # their own (tests/places-list.sh).
    pkgs.runCommandLocal "the-launcher-reads-on-where-the-reader-left" { } ''
      mkdir -p launcher share/applications
      cp ${entry} share/applications/reader.desktop
      cp ${home.xdg.configFile."quickshell/Theme.qml".source} Theme.qml
      cp ${../quickshell/Fuzzy.js} Fuzzy.js
      cp ${../quickshell/ProgramGlyphs.js} ProgramGlyphs.js
      cp ${../quickshell/Music.js} Music.js
      cp ${../quickshell/Words.qml} Words.qml
      cp ${../quickshell/Files.qml} Files.qml
      cp ${../quickshell/Daemon.qml} Daemon.qml
      cp ${../quickshell/FileFacts.js} FileFacts.js
      cp ${../quickshell/When.js} When.js
      ${lib.concatMapStrings (file: ''
        cp ${launcher + "/${file}"} launcher/${file}
      '') launched}
      cp ${./ReaderProgram.qml} launcher/ReaderProgram.qml
      cp ${./Reader.js} launcher/Reader.js
      cp ${../../tests/reader-program.qml} shell.qml
      cp ${../../tests/lib/Expect.js} Expect.js
      cp ${../../tests/lib/Frames.js} Frames.js
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
      # The list programs share, as recent-files prints it: a file the
      # reader left part way, and one it kept no place in.
      printf 'reader\t%s\n' "$HOME/notes/guide.md" "$HOME/notes/shared.md" > $HOME/shared
      export SHARED_LIST=${pkgs.writeShellScript "shared-list" ''cat "$HOME/shared"''}
      # What each reader shows now, as the reader keeps it.
      export SHOWING_LIST=${pkgs.writeShellScript "showing-list" ''
        printf '5151\t%s\n' "$HOME/notes/now.md"
      ''}
      # What reader --places prints, as tests/places-list.sh says.
      export PLACES_LIST=${pkgs.writeShellScript "places-list" (builtins.readFile ./tests/places-list.sh)}
      # The programs daemon runs the shared list, what each reader shows and
      # its places, given the sources shell.qml's Applications is and the
      # list the reader's page asks for, as recents.nix gives them.
      cat > sources.json <<EOF
      [{"watch": "$HOME/shared", "list": ["$SHARED_LIST"]},
       {"app": "reader", "kind": "file", "opened": ["$SHOWING_LIST"]},
       {"app": "reader", "name": "places", "asked": true, "list": ["$PLACES_LIST"]}]
      EOF
      ${lib.getExe programs} "$XDG_RUNTIME_DIR/daemons/programs.sock" sources.json &
      ${shellFolderLint}
      timeout 60 ${qs} -p shell.qml
      touch $out
    '';

  # clippy finds nothing in the reader, its tests included.
  reader-passes-clippy = passesClippy "reader-passes-clippy" (pkgs.callPackage ./package.nix { });

  # The keys the launcher lists over the reader's window are the reader's
  # own: every chord its table takes, with what it does, its group and the
  # key language's act, as reader --keys says them, and no other.
  reader-says-the-keys-it-takes =
    let
      listed = builtins.toJSON (
        lib.sort (a: b: a.chord < b.chord) (
          map (key: {
            inherit (key)
              chord
              does
              group
              act
              ;
          }) home.jitsusama.reader.keys
        )
      );
    in
    pkgs.runCommandLocal "reader-says-the-keys-it-takes" { nativeBuildInputs = [ pkgs.jq ]; } ''
      ${lib.getExe reader} --keys | jq -S -c 'sort_by(.chord)' > took
      echo ${lib.escapeShellArg listed} | jq -S -c . > listed
      if ! diff took listed; then
        echo "The launcher's list of the reader's keys (home.nix) isn't the reader's own (<) any more."
        exit 1
      fi
      touch $out
    '';

  # The reader draws a long page in the account's theme and takes Neovim's
  # keys: G goes to the end, gg to the top, where the cursor's line has its
  # rule in the accent in the margin, Ctrl+F a screen, Ctrl+O back,
  # Ctrl+Equal and Ctrl+0 zoom in and back, / finds and n goes on, y y,
  # y i p, y a p and y } copy the block at the cursor, its paragraph, that
  # with its blank line and to its end, as markdown, and flash them, 2j
  # and k move the cursor and not the view, so y y copies the line it
  # reaches, Ctrl+E and Ctrl+Y move the view a line alone, v and V select
  # from the cursor's line, where a selection's end leaves it, and every
  # key of visual mode moves the selection or takes a text object, y and
  # Ctrl+C copy the file's own markdown for it onto the clipboard,
  # Escape ends it, Space ? lists the keys, Escape puts the list away, g x labels the
  # link at the page's end and its label follows it to the heading, a link
  # to another markdown file sets that file in the same window, which the
  # launcher's Recent is told and whose saves are heard, Ctrl+O comes back
  # to the place left on the first and Ctrl+I goes on to the second again.
  # Then [b and ]b go through the folder's markdown files, } and { by
  # paragraphs, ]], 2]] and [[ by headings, H, M and L down the view, zt,
  # zz and zb place it, 50% goes half way and % from a code block's { to
  # its }, * and # search for the word here, :noh, :5, E492 and E481 on
  # the command line, :%y copies the file and says its lines, :'<,'>y the
  # lines V j took, a range past the end is E16, Ctrl+W, Ctrl+U and Ctrl+H
  # edit the line and Ctrl+C drops it, Escape at rest leaves the search's marks, every change
  # (d d, c c, S, x, p, d and c with a motion, :d, d on a selection) says
  # E21, Return a line, gO lists the headings and Return in it goes
  # to one, a mark is kept with its file between openings, gf and Ctrl+]
  # open the second file, :bp, :e and :bn go between the two, :e of a
  # file that isn't is E484, :h lists the keys, :g/Part/ lists the four
  # sections and Return goes to one, :marks and :ju list theirs, a chosen
  # row too long for the list reads on, the menu
  # shows on :, narrows as e or b is typed, and Tab, Shift+Tab, the arrows,
  # Ctrl+N, Ctrl+P, Ctrl+E and Ctrl+Y choose, put away and take its lines,
  # :e and a space listing the files, @: runs the last line again, :his
  # lists them and Return runs one, and :q
  # closes it (tests/reader-takes-its-keys.sh). The page is on the theme's background,
  # its heading in the strong text. Markdown files open in the reader.
  reader-takes-its-keys =
    let
      opens = graphical.config.xdg.mime.defaultApplications."text/markdown" or "";
      inherit (home.jitsusama.theme) colours terminal;
    in
    assert opens == "reader.desktop";
    pkgs.runCommandLocal "reader-takes-its-keys"
      {
        nativeBuildInputs = [
          reader
          pkgs.cage
          pkgs.gawk
          pkgs.grim
          pkgs.imagemagick
          pkgs.inotify-tools
          pkgs.wl-clipboard
          pkgs.wtype
        ];
      }
      ''
        ${screen}
        {
          echo '# Every Key Neovim Has'
          echo 'Some of it is on [the second page](second.md).'
          echo
          echo 'A **bold** start, then `code` in a line. A second sentence.'
          echo
          echo '```sh'
          echo 'echo one'
          echo 'echo two'
          echo '```'
          echo
          # A bracket's two ends, for %.
          echo '```'
          echo '{'
          echo '  "pages": [1, 2]'
          echo '}'
          echo '```'
          for n in $(seq 120); do
            # A section every 30 paragraphs, for ]], [[ and gO.
            if [ $((n % 30)) = 1 ]; then
              echo
              echo "## Part $(((n + 29) / 30))"
            fi
            echo
            echo "Paragraph $n says enough to take a line or two of the page, so that every key here has somewhere to go."
          done
          echo
          echo '[Back to the top](#every-key-neovim-has)'
        } > page.md
        printf '# The Second Page\n\nWhat the first page sends here.\n' > second.md
        timeout 600 cage -- $BASH ${./tests/reader-takes-its-keys.sh} page.md page.png \
          '${srgb colours.strong}' '${srgb terminal.cursor}' '${srgb terminal.cursor_text}' \
          '${srgb colours.accent}' \
          > ran 2>&1 || true
        if [ ! -e passed ]; then cat ran said 2> /dev/null; exit 1; fi

        at() { magick "$1" -format "%[pixel:p{$2}]" info:; }
        [ "$(at page.png 2,2)" = "${srgb colours.background}" ] \
          || { echo "the page is $(at page.png 2,2), not the theme's background"; exit 1; }
        mkdir $out
        cp page.png $out
      '';

  # The reader keeps the file it shows, read by the command its Recent in
  # the launcher is led by as its process and the whole path of a file given
  # by name in the folder it was started in, and nothing once ZZ closes it
  # (../../tests/lib/says-what-it-has-open.sh). What a killed reader left is
  # cleared as it starts.
  reader-says-the-file-it-shows =
    let
      kept = "${home.xdg.stateHome}/reader/showing";
      source = lib.findFirst (
        each: each.app or "" == "reader"
      ) null home.jitsusama.quickshell.launcher.recents;
      # The launcher's command, reading the folder this check keeps.
      opened = lib.concatMapStringsSep " " (
        part: if part == kept then ''"$XDG_STATE_HOME/reader/showing"'' else lib.escapeShellArg part
      ) source.opened;
    in
    assert lib.elem kept source.opened;
    pkgs.runCommandLocal "reader-says-the-file-it-shows"
      {
        nativeBuildInputs = [
          reader
          pkgs.cage
          pkgs.wtype
        ];
      }
      ''
        ${screen}
        mkdir -p state/reader/showing
        printf '# A Sheet\n\nTo read.\n' > sheet.md
        printf '999999\t/gone.md\n' > state/reader/showing/999999
        : > state/reader/showing/.999999
        timeout 300 cage -- $BASH ${../../tests/lib/says-what-it-has-open.sh} --closed-by ZZ \
          sheet.md reader -- ${opened} > ran 2>&1 || true
        if [ ! -e passed ]; then cat ran; exit 1; fi
        if [ -n "$(ls -A state/reader/showing)" ]; then
          echo "the reader left this: $(ls -A state/reader/showing)"
          exit 1
        fi
        touch $out
      '';

  # The reader says each file it's started with to the desktop's list of
  # files opened lately, through recent-files where the desktop has it,
  # stood in here, handing on all it was given, and then runs as it would.
  reader-says-the-files-it-opens =
    let
      recent-files = pkgs.writeShellScriptBin "recent-files" ''echo "$*" >> "$SAID"'';
    in
    pkgs.runCommandLocal "reader-says-the-files-it-opens"
      {
        nativeBuildInputs = [
          reader
          recent-files
        ];
      }
      ''
        export HOME=$PWD SAID=$PWD/said
        reader --keys > keys 2>&1 || { cat keys; exit 1; }
        grep -q 'Close the page' keys || { echo "the reader didn't run: $(cat keys)"; exit 1; }
        told=$(if [ -e said ]; then cat said; else echo 'nothing said'; fi)
        [ "$told" = "add reader --keys" ] || { echo "the reader said: $told"; exit 1; }
        touch $out
      '';
}
