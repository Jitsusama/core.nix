# The checks of the launcher's home: For You, Now, and the weather Now tells,
# which the folder's checks.nix gathers with every other place's; its opening
# comment says why each place keeps its own.
{
  account,
  copied,
  graphical,
  noFonts,
  nowShown,
  pkgs,
  qs,
  shellFolderLint,
  singleton,
  ...
}:
{
  # For You's :filter narrows to a kind and a source, by their words, as
  # VKCF3Z ruled it; modules/quickshell/tests/for-you-filter.qml holds
  # launcher/ForYouFilter.js to it in Qt's own engine.
  for-you-narrows-by-kind-and-source =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "for-you-narrows-by-kind-and-source" { } ''
      cp ${../tests/for-you-filter.qml} for-you-filter.qml
      cp ${../launcher/ForYouFilter.js} ForYouFilter.js
      cp ${../launcher/Order.js} Order.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml for-you-filter.qml
      touch $out
    '';

  # For You weighs what needs Joel, heaviest first and ties as they came,
  # bands what needs him now apart from To Look At and To Try, the
  # heaviest first and ties the oldest first in each, says each row's
  # strongest reason, reads the looks' texts into what waits and what's
  # history, a look three days old gone quiet into History, says a
  # look's ask under its why, or in its second form what happened and why
  # it matters, offers its choices with what follows each, each answered by
  # a steer to the session that left it or its quest's, saying so when it
  # may not have gone, and gathers what other sources give it, as
  # Passwords' one to-do, weighed beside the rest or done, as
  # modules/quickshell/tests/for-you.qml holds ForYou.js, LookForm.js and
  # Looks.js to it in Qt's own engine.
  for-you-weighs-what-needs-joel =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "for-you-weighs-what-needs-joel" { } ''
      cp ${../tests/for-you.qml} for-you.qml
      cp ${../launcher/ForYou.js} ForYou.js
      cp ${../launcher/LookForm.js} LookForm.js
      cp ${../launcher/Looks.js} Looks.js
      cp ${../launcher/Order.js} Order.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml for-you.qml
      touch $out
    '';

  # A look's Talk and Go To reach the session that left it, the agents
  # daemon's answer for its cell, or a session on its quest, and another
  # quest's only as the one that left a look on no quest, an old look naming
  # no cell is led by the one its session goes by, a Go whose session works
  # on another quest now says it moved on, and a served look settles only
  # once it opened, as
  # modules/quickshell/tests/look-cells.qml holds Looks.js to it in Qt's own
  # engine.
  a-look-leads-to-its-cell =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "a-look-leads-to-its-cell" { } ''
      cp ${../tests/look-cells.qml} look-cells.qml
      cp ${../launcher/ForYou.js} ForYou.js
      cp ${../launcher/LookForm.js} LookForm.js
      cp ${../launcher/Looks.js} Looks.js
      cp ${../launcher/Order.js} Order.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml look-cells.qml
      touch $out
    '';

  # For You stands what needs Joel now under the cell behind each, the cells
  # by their heaviest and what a cell asks leading it, heads each in Agents'
  # words for it, finds a session's own notices by the cell they name, and
  # sets Agents Asking above every heading, as
  # modules/quickshell/tests/for-you-cells.qml holds ForYouCells.js to it in
  # Qt's own engine.
  for-you-stands-each-under-its-cell =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "for-you-stands-each-under-its-cell" { } ''
      cp ${../tests/for-you-cells.qml} for-you-cells.qml
      cp ${../launcher/ForYou.js} ForYou.js
      cp ${../launcher/ForYouCells.js} ForYouCells.js
      cp ${../launcher/Order.js} Order.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml for-you-cells.qml
      touch $out
    '';

  # Now's rows after For You and before Lately's: the time, the date, the
  # location, the weather read as the launcher opens and the next meeting,
  # whose Return is the meeting's own (modules/quickshell/tests/now-moment.qml),
  # with the margin's pieces, the agents' waiting list, the weather and the
  # calendar stood in. The location is
  # Location.qml's own, reading whereabouts itself over its socket: here is the
  # one it kept from before it started, written here, GeoClue isn't there to say
  # another, and the clock's zone is a link that names another than the place's.
  # For You's looks are given as the to-do daemon's stream tells them, kept only
  # while each reads the same, and what's on as pieces whose rows are kept while
  # each reads the same. Now's Desktop holds the notices' rows as their
  # daemon serves them, stood in (tests/lib/NoticesDaemon.qml), each kept while
  # its notice is the same. Lately's rows are kept as well, so a notice arriving
  # makes only Desktop's row again, and a session's move that leaves what For
  # You reads of it the same makes none.
  the-launcher-shows-the-moment-in-now = nowShown {
    name = "the-launcher-shows-the-moment-in-now";
    test = ../tests/now-moment.qml;
  };

  # An opening into a place that shows none of For You's rows places no
  # time or date, which makes Time's and Today's rows, until Now's rows are
  # wanted, and they place it once, only once a minute has turned since it
  # was placed (modules/quickshell/tests/now-retime.qml).
  the-launcher-places-the-time-only-when-now-is-wanted = nowShown {
    name = "the-launcher-places-the-time-only-when-now-is-wanted";
    test = ../tests/now-retime.qml;
  };

  # A look Joel acts on shows so at once; one the to-do daemon refuses to
  # settle is put back as it read and said in the journal, and one it
  # settles stays as its stream then tells it, the same look in the
  # daemon's own words leaving the act's text, and a served look whose
  # session is gone is never settled, Now telling the launcher once, the
  # daemon stood in on its socket by the case itself
  # (modules/quickshell/tests/now-look-unwritten.qml).
  # With reports off, the journal names the look's file by its letters and
  # keeps the daemon's code, never its words, since a file is named for what
  # Joel wrote.
  the-launcher-puts-back-a-look-it-couldnt-write = nowShown {
    name = "the-launcher-puts-back-a-look-it-couldnt-write";
    test = ../tests/now-look-unwritten.qml;
    after = ''
      wanted="didn't settle 6 letters ({\"code\":\"refused\",\"message\":\"29 letters\"})"
      if ! grep -F -- "$wanted" said.log; then
        echo "the journal didn't say the look unsettled by its letters, wanted: $wanted"
        echo "what the launcher said:"
        grep -a 'launcher:' said.log || true
        exit 1
      fi
      if grep -e 'a\.json' -e 'read-only' said.log; then
        echo "the journal said the look's file or the daemon's words"
        exit 1
      fi
    '';
  };

  # The launcher's weather answers now, its parts and the rain by the hour from
  # one forecast, and the rain's own words answer the next rain
  # (modules/quickshell/tests/weather.qml), with what Open-Meteo would say kept
  # beforehand, so nothing leaves the sandbox; and here is where the laptop is,
  # as each case gives Location's here, ahead of the place set and the time
  # zone.
  the-launcher-answers-the-rain = pkgs.runCommandLocal "the-launcher-answers-the-rain" { } ''
    mkdir launcher
    ${copied [
      "Words.qml"
      "launcher/Provider.qml"
      "launcher/Weather.qml"
      "launcher/words.json"
    ]}
    cp ${(account graphical).xdg.configFile."quickshell/Theme.qml".source} Theme.qml
    cp ${../tests/weather.qml} shell.qml
    cp ${singleton "Location.qml" ''
      property var here: null
      property string clockZone: ""
    ''} Location.qml
    export HOME=$TMPDIR XDG_RUNTIME_DIR=$TMPDIR/runtime LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen
    mkdir -m 700 $XDG_RUNTIME_DIR
    export FONTCONFIG_FILE=${noFonts}
    ${shellFolderLint}
    timeout 60 ${qs} -p shell.qml
    touch $out
  '';

  # Now's Lately holds what's unread from each source until that source says
  # it's done, and History says how, as modules/quickshell/tests/lately.qml
  # holds Lately.js.
  lately-follows-each-source =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "lately-follows-each-source" { } ''
      cp ${../tests/lately.qml} lately.qml
      cp ${../launcher/Lately.js} Lately.js
      cp ${../launcher/Order.js} Order.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml lately.qml
      touch $out
    '';

  # Now lists what's on by its groups, what wants Joel first, then by name
  # within each, as modules/quickshell/tests/now-groups.qml holds
  # launcher/NowGroups.js to it.
  now-lists-by-group =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "now-lists-by-group" { } ''
      cp ${../tests/now-groups.qml} now-groups.qml
      cp ${../launcher/NowGroups.js} NowGroups.js
      cp ${../launcher/Order.js} Order.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml now-groups.qml
      touch $out
    '';

  # Explain It asks the realm's node only what one session is, by its address
  # alone, and tells the quick model only what Joel knows the work by, as
  # modules/quickshell/tests/explain-it.qml holds launcher/Explaining.js to it
  # in Qt's own engine.
  explain-it-asks-only-what-a-cell-is =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "explain-it-asks-only-what-a-cell-is" { } ''
      cp ${../tests/explain-it.qml} explain-it.qml
      cp ${../../../tests/lib/Expect.js} Expect.js
      ${copied [ "launcher/Explaining.js" ]}
      cp ${(import ../launcher/words.nix).places.on} on.json
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts} QML_XHR_ALLOW_FILE_READ=1
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml explain-it.qml
      touch $out
    '';

  # Explain It reads each session on its own from the realm's node, tells a node
  # that can't say from a session it doesn't know, and shows its pi's answer in
  # the act's place only while it's the act's own
  # (modules/quickshell/tests/explainer.qml), over a stand-in mycelium
  # (modules/quickshell/tests/explain-it-node.sh) that knows two sessions, one
  # asking Joel something, answers another late, never answers a fourth,
  # can't say a fifth, and finds only another by a sixth's title words.
  explain-it-shows-its-own-answer =
    # Kept in Nix because each line copies a file Nix gives it beside the
    # cases, or sets what the shell runs them with.
    pkgs.runCommandLocal "explain-it-shows-its-own-answer" { } ''
      mkdir -p launcher bin runtime && chmod 700 runtime
      ${copied [ "launcher/Explainer.qml" ]}
      { echo "#!${pkgs.runtimeShell}"; cat ${../tests/explain-it-node.sh}; } > bin/mycelium
      chmod +x bin/mycelium
      cp ${../tests/explainer.qml} shell.qml
      cp ${../../../tests/lib/Expect.js} Expect.js
      cp ${../../../tests/lib/Until.qml} Until.qml
      touch asked
      export HOME=$TMPDIR XDG_RUNTIME_DIR=$PWD/runtime LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts} MYCELIUM_ASKED=$PWD/asked PATH=$PWD/bin:$PATH
      ${shellFolderLint}
      timeout 60 ${qs} -p shell.qml
      touch $out
    '';

  # For You's to-dos hold only what Joel can do, now or later, in the order he
  # does them, and a session's want is his only while it's true, as
  # modules/quickshell/tests/waiting-on-you.qml holds Waiting.js to the agents'
  # own files, as the mycelium node serves them.
  needs-you-is-true =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "needs-you-is-true" { } ''
      cp ${../tests/waiting-on-you.qml} waiting-on-you.qml
      ${copied [ "Waiting.js" ]}
      cp ${../launcher/NeedsYou.js} NeedsYou.js
      cp ${../launcher/Ago.js} Ago.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml waiting-on-you.qml
      touch $out
    '';

  # For You weighs what waits on Joel again only when the node says something
  # new of it, not each time a file of it is written again, as
  # modules/quickshell/tests/waiting-kept.qml holds the real WaitingOnYou.qml
  # to, through Quickshell, with a stand-in for the node and the rest of the
  # shell.
  for-you-weighs-again-only-for-news =
    let
      folder = pkgs.linkFarm "for-you-weighs-again-only-for-news" {
        "WaitingOnYou.qml" = ../WaitingOnYou.qml;
        "Waiting.js" = ../Waiting.js;
        "PiSource.js" = ../PiSource.js;
        "Daemon.qml" = ../Daemon.qml;
        "Hearing.js" = ../Hearing.js;
        "Mending.js" = ../Mending.js;
        "Expect.js" = ../../../tests/lib/Expect.js;
        "shell.qml" = ../tests/waiting-kept.qml;
        "Sessions.qml" = singleton "Sessions.qml" ''
          readonly property string nodePath: Quickshell.env("XDG_RUNTIME_DIR") + "/node.sock"
          property var present: ({})
          property var running: []
          property var parked: []
          function wakeToGo(resting, session) {}
        '';
        "Words.qml" = singleton "Words.qml" ''
          function of(name) { return { said: {} }; }
          function name(id) { return id; }
        '';
        "Niri.qml" = singleton "Niri.qml" ''
          property var windows: ({})
          function act(request) {}
        '';
        "Notices.qml" = singleton "Notices.qml" ''
          property var live: ({})
          function mark(id, read, how) {}
          function settleWaiting(standing) {}
        '';
      };
    in
    pkgs.runCommandLocal "for-you-weighs-again-only-for-news" { } ''
      cp -L ${folder}/* .
      mkdir -m 700 runtime
      export HOME=$TMPDIR XDG_STATE_HOME=$TMPDIR/state XDG_RUNTIME_DIR=$PWD/runtime
      export LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      timeout 60 ${qs} -p shell.qml
      touch $out
    '';
}
