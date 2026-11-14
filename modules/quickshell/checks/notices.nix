# The checks of the notices: the desktop's cards, their history and how they're
# kept, which the folder's checks.nix gathers with every other place's; its
# opening comment says why each place keeps its own.
{
  account,
  copied,
  graphical,
  lib,
  noFonts,
  noticesOnNiri,
  pkgs,
  qs,
  shellFolderLint,
  singleton,
  spawnedBy,
  ...
}:
{
  # The notices draw a notice a program sends as a card on a layer niri
  # lists and on the arrival line, which takes no keys and is mapped only
  # while a notice shows, and Super+Shift+N puts away one that stays
  # (tests/notices-on-niri.sh). Away only after an hour, since nobody types
  # here but the check.
  the-notices-land-on-niri = noticesOnNiri {
    name = "the-notices-land-on-niri";
    away = 3600;
    script = ../../../tests/notices-on-niri.sh;
    given = spawnedBy "Super+Shift+N";
  };

  # A notice that comes while Joel's away waits, past the time its sender
  # gave it (tests/notices-wait-while-away.sh). Away after a second, so he
  # is before it comes.
  the-notices-wait-while-joel-is-away = noticesOnNiri {
    name = "the-notices-wait-while-joel-is-away";
    away = 1;
    script = ../../../tests/notices-wait-while-away.sh;
  };

  # Do not disturb holds notices while the screen is shared and lets them go
  # after, as tests/quiet.qml drives it through Quickshell itself, with
  # niri's casts, the notices, the recorder and the shell's options stood in
  # for.
  do-not-disturb-holds-while-sharing =
    let
      chosen = "readonly property var doNotDisturb: ({ whileSharing: true })";
      options = singleton "Options.qml" chosen;
    in
    pkgs.runCommandLocal "do-not-disturb-holds-while-sharing" { } ''
      mkdir runtime && chmod 700 runtime
      ${copied [ "Quiet.qml" ]}
      cp ${../../../tests/quiet.qml} shell.qml
      cp ${singleton "Niri.qml" "property var casts: ({})"} Niri.qml
      cp ${singleton "Notices.qml" "property var history: []"} Notices.qml
      cp ${singleton "Recorder.qml" "property var windowSession: null"} Recorder.qml
      cp ${options} Options.qml
      export HOME=$TMPDIR XDG_RUNTIME_DIR=$TMPDIR/runtime LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      timeout 60 ${qs} -p shell.qml
      touch $out
    '';

  # Every program an account lists in the launcher, each desktop entry it
  # shows, wears a mark of its own in ProgramGlyphs.js's table, its logo or
  # the theme's mark for it, never the program mark the rest share, and
  # each theme mark it names is one the theme has. Each program a notice's
  # sender is named as (modules/notices/senders.json) is one the table
  # knows, so its notices wear the mark its row does.
  every-program-wears-its-own-mark =
    let
      home = account graphical;
      marks = pkgs.writeText "marks" (
        lib.concatMapStrings (name: "${name}\n") (lib.attrNames home.jitsusama.theme.marks)
      );
    in
    pkgs.runCommandLocal "every-program-wears-its-own-mark" { } ''
      sed -n '/^const table = {/,/^};/p' ${../ProgramGlyphs.js} > table
      sed -n -E 's/^ *"?([^":]+)"?: ".*/\1/p' table > known
      grep -o '"mark:[^"]*"' table | cut -d : -f 2 | tr -d '"' > named
      bare=""
      for entry in ${home.home.path}/share/applications/*.desktop; do
        if grep -q -x -E '(NoDisplay|Hidden)=true' "$entry"; then continue; fi
        id=$(basename "$entry" .desktop | tr '[:upper:]' '[:lower:]')
        grep -q -x -F "$id" known || bare="$bare $id"
      done
      if [ -n "$bare" ]; then
        echo "These programs would wear the program mark every program without one shares:$bare"
        exit 1
      fi
      while read -r mark; do
        grep -q -x -F "$mark" ${marks} || { echo "No theme mark is named $mark"; exit 1; }
      done < named
      ${lib.getExe pkgs.jq} -r '.senders[].program // empty' ${../../notices/senders.json} > senders
      [ -s senders ] || { echo "senders.json names no sender's program"; exit 1; }
      while read -r program; do
        grep -q -x -F "$program" known \
          || { echo "A notice from $program would wear the bell: no mark is known for it"; exit 1; }
      done < senders
      touch $out
    '';

  # A notice stands in its card's columns: drawn as the pop-up draws it and
  # as the held list will, through the one card and row, at 2 and at 1.5,
  # its ink stands the inset from the frame on every side, its mark centred
  # on the title's capitals, and the two cards' text columns differ by the
  # gutter alone, as modules/quickshell/tests/notice-columns.py reads the
  # pictures back.
  notices-stand-on-the-cards-columns =
    let
      machine = graphical.config;
      measure = lib.getExe (pkgs.python3.withPackages (python: [ python.pillow ]));
    in
    pkgs.runCommandLocal "notices-stand-on-the-cards-columns" { } ''
      export HOME=$TMPDIR XDG_RUNTIME_DIR=$TMPDIR/runtime LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${pkgs.makeFontsConf { fontDirectories = machine.fonts.packages; }}
      mkdir -m 700 $XDG_RUNTIME_DIR
      for scale in 2 1.5; do
        mkdir at-$scale
        cd at-$scale
        cp ${(account graphical).xdg.configFile."quickshell/Theme.qml".source} Theme.qml
        ${copied [
          "Card.qml"
          "Cover.qml"
          "Floating.qml"
          "Mark.qml"
          "NoticeRow.qml"
          "Signature.qml"
        ]}
        cp ${../../../tests/notice-columns.qml} shell.qml
        ${shellFolderLint}
        QT_SCALE_FACTOR=$scale timeout 60 ${qs} -p shell.qml > said 2>&1 || { cat said; exit 1; }
        ${measure} ${../tests/notice-columns.py} . said
        cd ..
      done
      touch $out
    '';

  # Once a notice is opened, its sender's window comes forward when it
  # doesn't by itself, as tests/notice-comes-forward.qml holds Forward.js
  # to niri's windows.
  a-notice-comes-forward =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "a-notice-comes-forward" { } ''
      cp ${../../../tests/notice-comes-forward.qml} notice-comes-forward.qml
      ${copied [ "Forward.js" ]}
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml notice-comes-forward.qml
      touch $out
    '';

  # What a notice's card offers of its sender's, its sender's own Read,
  # and the line a search names a notice by, as tests/notice.qml holds
  # Notice.js.
  a-notice-found-is-noted-by-what-was-typed =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "a-notice-found-is-noted-by-what-was-typed" { } ''
      cp ${../../../tests/notice.qml} notice.qml
      cp ${../launcher/Notice.js} Notice.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml notice.qml
      touch $out
    '';

  # What comes true by itself is said once as a notice, and again only as
  # it tells something new, its earlier notice taken back first, and in new
  # words in place, by its id, as tests/arrivals.qml holds Arrivals.qml to
  # with the facts and the notices stood in, and notify-send writing down
  # what it's asked to say and the id it gave.
  an-arrival-is-said-once-each-time =
    let
      facts = singleton "Facts.qml" "property var all: []";
      notices = singleton "Notices.qml" ''
        property var live: ({})
        property var history: []
        property var marked: []
        property var raisedIds: []
        function mark(id, read) { marked = marked.concat([id]); }
        function raise(id) { raisedIds = raisedIds.concat([id]); }
      '';
      options = singleton "Options.qml" "property bool reports: false";
      report = singleton "Report.qml" "function say(text) {}";
      # A new notice's id is the next from 101, a replaced one keeps its own.
      notifySend = pkgs.writeShellScriptBin "notify-send" ''
        next=$XDG_RUNTIME_DIR/next
        id=$(( $(cat "$next" 2>/dev/null || echo 100) + 1 ))
        replaced=""
        for word in "$@"; do
          case $word in --replace-id=*) replaced=''${word#--replace-id=} ;; esac
        done
        if [ -n "$replaced" ]; then id=$replaced; else echo "$id" > "$next"; fi
        printf '%s =>%s\n' "$*" "$id" >> "$XDG_RUNTIME_DIR/said"
        echo "$id"
      '';
    in
    pkgs.runCommandLocal "an-arrival-is-said-once-each-time" { } ''
      ${copied [ "Arrivals.qml" ]}
      cp ${facts} Facts.qml
      cp ${notices} Notices.qml
      cp ${options} Options.qml
      cp ${report} Report.qml
      cp ${../../../tests/arrivals.qml} shell.qml
      export HOME=$TMPDIR XDG_RUNTIME_DIR=$TMPDIR/runtime LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts} PATH=${notifySend}/bin:$PATH
      mkdir -m 700 $XDG_RUNTIME_DIR
      # Made before the shell starts, so its watch has a file to follow.
      touch $XDG_RUNTIME_DIR/said
      ${shellFolderLint}
      timeout 60 ${qs} -p shell.qml
      touch $out
    '';

  # The notices' stack keeps the order tests/stack.qml holds: a critical one
  # in front, the rest as they came, one unheard at the back, and turning and
  # bringing one forward as the keys and a click do.
  the-notices-keep-their-order =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "the-notices-keep-their-order" { } ''
      cp ${../../../tests/stack.qml} stack.qml
      ${copied [ "Stack.js" ]}
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml stack.qml
      touch $out
    '';

  # The arrival line shows the front of the stack still waiting on Joel,
  # only a critical one while notices are held or he's talking, none while
  # the launcher shows them, and counts the rest (tests/notice-line.qml).
  the-arrival-line-shows-what-still-waits =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "the-arrival-line-shows-what-still-waits" { } ''
      cp ${../../../tests/notice-line.qml} notice-line.qml
      ${copied [
        "Knock.js"
        "NoticeLine.js"
      ]}
      cp ${../../../tests/lib/Expect.js} Expect.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml notice-line.qml
      touch $out
    '';

  # A notice the notices daemon serves reads as Quickshell's own did, and
  # each act on it is an ask of the daemon, as tests/live-notice.qml holds
  # LiveNotice.js to.
  a-live-notice-asks-the-daemon =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "a-live-notice-asks-the-daemon" { } ''
      cp ${../../../tests/live-notice.qml} live-notice.qml
      ${copied [ "LiveNotice.js" ]}
      cp ${../../../tests/lib/Expect.js} Expect.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml live-notice.qml
      touch $out
    '';

  # The held notices take Vim's keys in each mode from one table, as
  # tests/notice-keys.qml holds NoticeKeys.js to: motions, operators and
  # objects with their counts, visual's operator taken with a second press,
  # fresh presses only for what acts, the lines typed on, a prompt's
  # editing, Space ?'s list and the foot's hints; and Search.js, which
  # their / shares with the launcher's, to what it finds.
  the-notices-take-vi-keys =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "the-notices-take-vi-keys" { } ''
      cp ${../../../tests/notice-keys.qml} notice-keys.qml
      ${copied [
        "NoticeKeys.js"
        "Search.js"
      ]}
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml notice-keys.qml
      touch $out
    '';

  # The held notices are Vim's quickfix list, as tests/notice-list.qml
  # holds NoticeList.js to: the newest first, where each motion lands, what
  # an operator takes, the view kept around the cursor, the gutter's
  # numbers, and what u and Ctrl+R move between.
  the-notices-are-a-quickfix-list =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "the-notices-are-a-quickfix-list" { } ''
      cp ${../../../tests/notice-list.qml} notice-list.qml
      ${copied [
        "Changes.js"
        "NoticeList.js"
      ]}
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml notice-list.qml
      touch $out
    '';
}
