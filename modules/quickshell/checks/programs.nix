# The checks of the launcher's programs and their controls, which the folder's
# checks.nix gathers with every other place's; its opening comment says why each
# place keeps its own.
{
  account,
  graphical,
  lib,
  noFonts,
  pkgs,
  programs,
  qs,
  shellFolderLint,
  singleton,
  ...
}:
{
  # A program's step in the launcher holds what controls it, each only where
  # it applies, in order (tests/program-controls.qml): over three desktop
  # entries, with niri, the desktop's shortcuts, the notices' history and
  # what's playing stood in, a player, two streams and two notices its own
  # and one of each another's.
  the-launcher-holds-a-programs-controls =
    let
      # Two of the tester's windows, the one on the workspace in front
      # used last.
      niri = singleton "Niri.qml" ''
        property int revision: 0
        property var windows: ({
          "11": { id: 11, app_id: "tester", title: "Logbook - Tester", workspace_id: 1,
            focus_timestamp: { secs: 200, nanos: 0 } },
          "12": { id: 12, app_id: "tester", title: "Draft - Tester", workspace_id: 2,
            focus_timestamp: { secs: 100, nanos: 0 } }
        })
        property var workspaces: ({
          "1": { id: 1, idx: 1, is_focused: true, active_window_id: 11 },
          "2": { id: 2, idx: 2, is_focused: false, active_window_id: 12 }
        })
        readonly property int current: 11
        property var acted: []
        property var spawned: []
        property var spawns: []
        function workspace(id) { return workspaces[id ?? "1"] ?? null; }
        function act(action) { acted = acted.concat([action]); }
        function spawn(command) { spawned = command; spawns = spawns.concat([command]); }
      '';
      notices = singleton "Notices.qml" ''
        property var history: [
          { id: 3, at: Date.now() - 60000, app: "Tester", summary: "Newest", body: "",
            glyph: 0xf0f3, hue: "program", read: false, missed: false },
          { id: 2, at: Date.now() - 120000, app: "Other", summary: "Not its", body: "",
            glyph: 0xf0f3, hue: "program", read: false, missed: false },
          { id: 1, at: Date.now() - 180000, app: "tester", summary: "Oldest", body: "said",
            glyph: 0xf0f3, hue: "program", read: true, missed: false }
        ]
        property var live: ({})
        property var resting: ({})
        property var waiting: []
        property bool gathered: false
        property var marked: []
        // The newest still open, its picture held in memory as a sender's
        // image data is; the oldest's kept as a file. Each asking counted,
        // in a plain object so counting notifies nothing.
        readonly property var asked: ({ pictures: 0 })
        function pictureOf(notice) {
          asked.pictures += 1;
          return ({ 3: "image://qsimage/3/0", 1: "file:///kept/oldest.png" })[notice.id] ?? "";
        }
        function mark(id, read) { marked = marked.concat([id]); }
        function seen() {}
        // Who signs a notice, as its card says it: here by the name it
        // came with, no sender being known.
        function sender(name) { return { glyph: 0xf0f3, hue: "program", name: name }; }
        // A sender's window by its program's id, as Notices finds it, the
        // desktop's entries aside.
        function senderWindow(app) {
          return Object.values(Niri.windows)
            .find(w => w.app_id.toLowerCase() === app.toLowerCase()) ?? null;
        }
      '';
      # The tester's player, found by its desktop entry, a radio's, found
      # only by its identity, and one nobody's; the tester's two streams, a
      # filter's carrying its name too, and another program's; and the
      # radio recording from a microphone, beside a level meter of its own.
      playing = singleton "Playing.qml" ''
        property var players: [tester, radio, nobody]
        property var streams: [first, second, filter, other]
        property var recorders: [listening, meter]
        QtObject {
          id: tester
          property string desktopEntry: "tester"
          property string identity: "The Tester"
          property bool isPlaying: true
          property bool canPlay: true
          property bool canPause: true
          property bool canGoNext: true
          property bool canGoPrevious: true
          property string trackTitle: "Song of the test"
          property string trackArtist: "The Testers"
          property real position: 98
          property real length: 403
          property int nexts: 0
          property int previouses: 0
          function play() { isPlaying = true; }
          function pause() { isPlaying = false; }
          function next() { nexts += 1; }
          function previous() { previouses += 1; }
        }
        QtObject {
          id: radio
          property string desktopEntry: ""
          property string identity: "Radio"
          property bool isPlaying: false
          property bool canPlay: true
          property bool canPause: true
          property bool canGoNext: false
          property bool canGoPrevious: false
          property string trackTitle: ""
          property string trackArtist: ""
        }
        QtObject {
          id: nobody
          property string desktopEntry: "elsewhere"
          property string identity: "Elsewhere"
          property bool isPlaying: true
          property bool canPlay: true
          property bool canPause: true
          property bool canGoNext: true
          property bool canGoPrevious: true
          property string trackTitle: "Not its song"
          property string trackArtist: ""
        }
        // A stream is its own audio, as a PipeWire node's volume and mute
        // are its audio's.
        component Stream: QtObject {
          id: stream
          property string name
          property var properties: ({})
          property real volume: 0.4
          property bool muted: false
          readonly property var audio: stream
        }
        Stream { id: first; name: "Tester"; properties: ({ "application.name": "Tester" }) }
        Stream { id: second; name: "stream-2"; properties: ({ "application.process.binary": "tester" }) }
        Stream {
          id: filter
          name: "Tester"
          properties: ({ "application.name": "Tester", "node.link-group": "tuning" })
          volume: 0.9
        }
        Stream {
          id: other
          name: "Other"
          properties: ({ "application.name": "Other" })
          volume: 0.7
        }
        Stream { id: listening; name: "Radio"; properties: ({ "application.name": "Radio" }) }
        Stream {
          id: meter
          name: "Radio"
          properties: ({ "application.name": "Radio", "stream.monitor": "true" })
        }
      '';
      entry =
        name: lines:
        pkgs.writeText "${name}.desktop" ''
          [Desktop Entry]
          Type=Application
          ${lines}
        '';
      tester = entry "tester" ''
        Name=Tester
        GenericName=Logbook
        Exec=tester %U
        MimeType=text/html;x-scheme-handler/https;
        Categories=Office;TextEditor;
        Actions=private;new;logbook;

        [Desktop Action private]
        Name=Tester: Private window
        Exec=tester --private

        [Desktop Action new]
        Name=New Window
        Exec=tester --new-window

        [Desktop Action logbook]
        Name=Open the logbook
        Exec=tester --logbook
      '';
      quiet = entry "quiet" ''
        Name=Quiet
        Exec=quiet
      '';
      radio = entry "radio" ''
        Name=Radio
        Exec=radio
      '';
      browser = entry "browser" ''
        Name=Browser
        Exec=browser %U
      '';
      # Hidden, as Tuple's is: one another provider gives acts to, and one
      # nobody does.
      pair = entry "pair" ''
        Name=Pair
        Exec=pair
        NoDisplay=true
      '';
      ghost = entry "ghost" ''
        Name=Ghost
        Exec=ghost
        NoDisplay=true
      '';
      # A program whose entry's id isn't its command, as zathura's isn't.
      viewer = entry "viewer" ''
        Name=Viewer
        Exec=viewer %U
      '';
    in
    pkgs.runCommandLocal "the-launcher-holds-a-programs-controls" { } ''
      mkdir -p launcher share/applications share/mime
      # The kinds of file the tester opens end as shared-mime-info says.
      printf '50:text/html:*.html\n' > share/mime/globs2
      cp ${tester} share/applications/tester.desktop
      cp ${quiet} share/applications/quiet.desktop
      cp ${radio} share/applications/radio.desktop
      cp ${browser} share/applications/browser.desktop
      cp ${pair} share/applications/pair.desktop
      cp ${ghost} share/applications/ghost.desktop
      cp ${viewer} share/applications/org.example.viewer.desktop
      cp ${(account graphical).xdg.configFile."quickshell/Theme.qml".source} Theme.qml
      cp ${../Fuzzy.js} Fuzzy.js
      cp ${../launcher/Provider.qml} launcher/Provider.qml
      cp ${../ProgramGlyphs.js} ProgramGlyphs.js
      cp ${../launcher/Asking.js} launcher/Asking.js
      cp ${../launcher/Numbers.js} launcher/Numbers.js
      cp ${../launcher/Applications.qml} launcher/Applications.qml
      cp ${../launcher/HeldWindows.js} launcher/HeldWindows.js
      cp ${../launcher/Media.js} launcher/Media.js
      cp ${../launcher/Program.js} launcher/Program.js
      cp ${../FileFacts.js} FileFacts.js
      cp ${../When.js} When.js
      cp ${../Daemon.qml} Daemon.qml
      cp ${../Hearing.js} Hearing.js
      cp ${../Music.js} Music.js
      cp ${../launcher/Order.js} launcher/Order.js
      cp ${../launcher/Recents.qml} launcher/Recents.qml
      cp ${../launcher/Recents.js} launcher/Recents.js
      cp ${../launcher/Recent.js} launcher/Recent.js
      cp ${../launcher/Captured.js} launcher/Captured.js
      cp ${../launcher/PastNotices.qml} launcher/PastNotices.qml
      cp ${../launcher/Notice.js} launcher/Notice.js
      cp ${../NoticeBody.js} NoticeBody.js
      cp ${../launcher/Ago.js} launcher/Ago.js
      cp ${../Words.qml} Words.qml
      cp ${../Files.qml} Files.qml
      cp ${../launcher/words.json} launcher/words.json
      cp ${../../../tests/program-controls.qml} shell.qml
      cp ${../../../tests/lib/HeardWindows.js} HeardWindows.js
      cp ${niri} Niri.qml
      cp ${notices} Notices.qml
      cp ${playing} launcher/Playing.qml
      cp ${singleton "Shortcuts.qml" "property var actions: []"} Shortcuts.qml
      export HOME=$TMPDIR XDG_RUNTIME_DIR=$TMPDIR/runtime LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen
      export XDG_DATA_HOME=$TMPDIR/data XDG_DATA_DIRS=$PWD/share
      mkdir -m 700 $XDG_RUNTIME_DIR
      export FONTCONFIG_FILE=${noFonts}
      # What quiet had open lately, as a program keeps it, and the command
      # that lists it.
      printf '%s\n' "$HOME/src/core.nix" /etc > $HOME/recent
      export RECENT_LIST=${pkgs.writeShellScript "recent-list" ''cat "$HOME/recent"''}
      # The list programs share, as recent-files prints it: the tester by
      # its name and by its id, one file twice and a picture in a folder,
      # one of no program here, and one the radio was given, which can't
      # open a file itself, and one the viewer opened, named by its command
      # as recent-files names it, each with when, in seconds, the viewer's
      # last with none.
      printf '%s\t%s\t%s\n' Tester "$HOME/notes.md" 300 nobody-here "$HOME/lost.md" 290 \
        tester "$HOME/plan.md" 100 Tester "$HOME/notes.md" 50 radio "$HOME/song.ogg" 40 \
        tester "$HOME/Pictures/shot.png" 35 viewer "$HOME/paper.pdf" 30 \
        viewer "$HOME/old.pdf" "" > $HOME/shared
      export SHARED_LIST=${pkgs.writeShellScript "shared-list" ''cat "$HOME/shared"''}
      # The browser's sites, as chrome-recent-sites prints them, each
      # reading saying which it is.
      export SITES_LIST=${pkgs.writeShellScript "sites-list" ''
        echo . >> "$HOME/sites-read"
        n=$(wc -l < "$HOME/sites-read")
        printf 'https://example.org/%s\tPage %s\nhttps://other.example.net/a\tOlder page\n' "$n" "$n"
      ''}
      # Its tabs open now, as chrome-recent-sites open lists them: one page
      # open in two windows, the older page open again, another in front
      # of the second window, and in a third one left out, told of by its
      # title alone, in front of a page behind it; and one with no id, as
      # Chrome's session file says each tab, behind in the second window.
      export OPENED_LIST=${pkgs.writeShellScript "opened-list" ''
        printf '%s\t%s\t%s\t%s\t%s\t%s\n' \
          7 1 true 3000 https://open.example.com/a 'Open A' \
          8 1 false 2000 https://other.example.net/a 'Older page' \
          9 2 false 1000 https://open.example.com/a 'Open A' \
          10 2 true 500 https://another.example.com/b Another \
          11 3 true 400 "" 'Left out' \
          12 3 false 450 https://behind.example.com/ Behind \
          "" 2 false 100 https://filed.example.com/ Filed
      ''}
      # What the viewer has open now, as mpv's opened lists what each mpv
      # plays: one file in two of its processes, one also on the list
      # programs share, and one in a process with no window; then, as
      # Neovim's says what's open in kitty's windows, one file in two
      # windows named by niri's id, with what Go To is given, and one in a
      # window that's gone.
      export VIEWING_LIST=${pkgs.writeShellScript "viewing-list" ''
        printf '%s\t%s\n' 5151 "$HOME/open.pdf" 5252 "$HOME/paper.pdf" 5353 "$HOME/open.pdf" \
          9999 "$HOME/ghost.pdf"
        printf '%s\t%s\t%s\n' @35 "$HOME/notes.md" 10:7 @34 "$HOME/notes.md" 10:6 \
          @36 "$HOME/gone.md" 10:8
      ''}
      # The programs daemon runs each list, given the sources shell.qml's
      # Applications is, as recents.nix gives both.
      cat > sources.json <<EOF
      [{"app": "quiet", "kind": "folder", "watch": "$HOME/recent", "list": ["$RECENT_LIST"]},
       {"watch": "$HOME/shared", "list": ["$SHARED_LIST"], "dated": true},
       {"app": "browser", "kind": "site", "asked": true, "list": ["$SITES_LIST"],
        "opened": ["$OPENED_LIST"]},
       {"app": "org.example.viewer", "kind": "file", "opened": ["$VIEWING_LIST"]}]
      EOF
      ${lib.getExe programs} "$XDG_RUNTIME_DIR/daemons/programs.sock" sources.json &
      ${shellFolderLint}
      timeout 60 ${qs} -p shell.qml
      touch $out
    '';

  # A program's drawer says what it is, where its windows are, what it
  # opens and how it starts, in the order every drawer keeps, as
  # tests/program-drawer.qml holds launcher/Program.js to the programs
  # daemon's record of a desktop entry made up there.
  a-program-says-all-it-knows =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "a-program-says-all-it-knows" { } ''
      mkdir launcher
      cp ${../../../tests/program-drawer.qml} program-drawer.qml
      cp ${../launcher/Program.js} launcher/Program.js
      cp ${../launcher/Numbers.js} launcher/Numbers.js
      cp ${../FileFacts.js} FileFacts.js
      cp ${../When.js} When.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml program-drawer.qml
      touch $out
    '';
}
