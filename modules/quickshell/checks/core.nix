# The checks of the launcher's own machinery: its prompt, rows, keys, pacing and
# answers, and the launcher on niri, which the folder's checks.nix gathers with
# every other place's; its opening comment says why each place keeps its own.
{
  account,
  graphical,
  launcherOnNiri,
  lib,
  mathsModules,
  noFonts,
  pkgs,
  qs,
  shellFolderLint,
  singleton,
  timedQml,
  ...
}:
{
  # The launcher answers what's typed into it, and the held notices take
  # vi's keys, on the account's whole shell on niri, in seconds rather than
  # a machine's minutes (tests/launcher-on-niri-test.py). Its meanings come
  # from the meaning daemon's stand-in, which loads no model
  # (tests/desktop/stand-in-meaning.py). WARP stands in as warp-cli
  # answering from the files a test writes, the real warp reading them
  # (tests/warp-cli-stand-in.sh), and warp holds its link on the socket
  # the account's systemd would, as WARP.
  the-launcher-answers-whats-typed-on-niri =
    let
      warpCli = pkgs.writeShellScript "warp-cli" ''
        exec ${lib.getExe pkgs.bash} ${../../../tests/warp-cli-stand-in.sh} "$@"
      '';
      warp = lib.getExe (pkgs.callPackage ../../warp/package.nix { });
    in
    launcherOnNiri {
      name = "the-launcher-answers-whats-typed-on-niri";
      tests = ../../../tests/launcher-on-niri-test.py;
      given = [
        "MEANING=${
          pkgs.writers.writePython3 "stand-in-meaning" {
            doCheck = false;
          } (builtins.readFile ../../../tests/desktop/stand-in-meaning.py)
        }"
        "WARP_CONTRACT=${../../warp/contract}"
        "WARP=${pkgs.writeShellScript "warp-serve" ''
          WARP_CLI=${warpCli} exec ${warp} serve "$@"
        ''}"
      ];
      shell.launcher.providers.warp = {
        enable = true;
        warpCli = toString warpCli;
      };
    };

  # An opening draws home as the closed launcher kept it, each row as it
  # stood, then fills it in where its rows stand with nothing flashing; and
  # keys typed as it opens all land (tests/opening-on-niri-test.py).
  the-launcher-opens-on-what-it-kept-on-niri = launcherOnNiri {
    name = "the-launcher-opens-on-what-it-kept-on-niri";
    tests = ../../../tests/opening-on-niri-test.py;
  };

  # The launcher, the held notices and a window share the keys by one rule:
  # the newest Joel opened takes them, a notice arriving never does, and a
  # click on a window gives it them (tests/focus-on-niri-test.py).
  the-newest-surface-takes-the-keys-on-niri = launcherOnNiri {
    name = "the-newest-surface-takes-the-keys-on-niri";
    tests = ../../../tests/focus-on-niri-test.py;
    given = [ "WLRCTL=${lib.getExe pkgs.wlrctl}" ];
  };

  # The launcher finds what's meant: for each query in tests/fuzzy.qml the
  # right thing comes first, few rows are left, or nothing answers, and a
  # query keeps within its budget in at least one of five processes, all in
  # Qt's own JavaScript engine, the one the shell runs the matcher in.
  the-launcher-finds-what-is-meant = pkgs.runCommandLocal "the-launcher-finds-what-is-meant" { } ''
    cp ${../../../tests/fuzzy.qml} fuzzy.qml
    cp ${../Fuzzy.js} Fuzzy.js
    export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
    export FONTCONFIG_FILE=${noFonts}
    ${timedQml "fuzzy.qml"}
    touch $out
  '';

  # The launcher makes a text ready to match without growing the shell's
  # memory by more than a few kilobytes a text, kept as it keeps them
  # (tests/fuzzy-light.qml), in a fresh process of Qt's own JavaScript
  # engine reading its own memory.
  the-launcher-makes-texts-ready-lightly =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "the-launcher-makes-texts-ready-lightly" { } ''
      cp ${../../../tests/fuzzy-light.qml} fuzzy-light.qml
      cp ${../Fuzzy.js} Fuzzy.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts} QML_XHR_ALLOW_FILE_READ=1
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 120 ${qt}/bin/qml fuzzy-light.qml
      touch $out
    '';

  # The launcher lands where it did: a thing is one row, the same letters
  # keep their row, and one slip teaches nothing, as tests/ranking.qml holds
  # launcher/Rank.js to it in Qt's own engine.
  the-launcher-lands-where-it-did =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "the-launcher-lands-where-it-did" { } ''
      cp ${../../../tests/ranking.qml} ranking.qml
      cp ${../launcher/Rank.js} Rank.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml ${qt}/bin/qml ranking.qml
      touch $out
    '';

  # A list the launcher shows holds still as what's in it changes: its rows
  # keep their order, new ones come last and the selection keeps its row, as
  # tests/steady.qml holds launcher/Steady.js to it in Qt's own engine.
  the-launcher-holds-still-as-it-refreshes =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "the-launcher-holds-still-as-it-refreshes" { } ''
      cp ${../../../tests/steady.qml} steady.qml
      cp ${../launcher/Steady.js} Steady.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml steady.qml
      touch $out
    '';

  # Recent means one thing in every place it stands: the last nine things,
  # newest first, each once, never what's open, as tests/recent.qml holds
  # launcher/Recent.js to it in Qt's own engine.
  recent-means-one-thing =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "recent-means-one-thing" { } ''
      cp ${../../../tests/recent.qml} recent.qml
      cp ${../launcher/Recent.js} Recent.js
      cp ${../launcher/Recents.js} Recents.js
      cp ${../launcher/Order.js} Order.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml recent.qml
      touch $out
    '';

  # The open launcher holds its keys only once niri says no window has them,
  # so one reopened right after a closing, whose letting go reached niri
  # late, isn't closed as if a window took them (vm-desktop-looks, 3 of 6
  # runs, 2026-11-11): tests/keys-held.qml replays each order of events
  # through launcher/KeysHeld.js in Qt's own engine.
  the-launcher-closes-only-for-keys-it-held =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "the-launcher-closes-only-for-keys-it-held" { } ''
      cp ${../../../tests/keys-held.qml} keys-held.qml
      cp ${../launcher/KeysHeld.js} KeysHeld.js
      cp ${../../../tests/lib/Expect.js} Expect.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml keys-held.qml
      touch $out
    '';

  # A place's row reads alike at home and inside its parent (VKCF3Z's
  # 2a429f67, on Joel's 2026-11-08 "i don't like how the title and values on
  # the home screen differ from every other sub-menu"): tests/place-row.qml
  # holds launcher/PlaceRow.js, which makes it in both, in Qt's own engine,
  # and home still unfilled a second after an opening, each reading as being
  # read.
  a-place-reads-alike-everywhere =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "a-place-reads-alike-everywhere" { } ''
      cp ${../../../tests/place-row.qml} place-row.qml
      cp ${../launcher/PlaceRow.js} PlaceRow.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml place-row.qml
      touch $out
    '';

  # The rows drawn are told where they stand only when it changes, so a
  # refresh in the place they stay in redraws none of them:
  # tests/rows-place.qml holds launcher/RowsPlace.js to it in Qt's own engine.
  the-rows-hear-only-a-new-place =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "the-rows-hear-only-a-new-place" { } ''
      cp ${../../../tests/rows-place.qml} rows-place.qml
      cp ${../launcher/RowsPlace.js} RowsPlace.js
      cp ${../../../tests/lib/Expect.js} Expect.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml rows-place.qml
      touch $out
    '';

  # The launcher's selection, and the list's scroll with it, lands within
  # 50 ms of its key: tests/glide.qml holds Glide.js to it in Qt's own
  # engine.
  the-selection-lands-within-50-ms-of-its-key =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "the-selection-lands-within-50-ms-of-its-key" { } ''
      cp ${../../../tests/glide.qml} glide.qml
      cp ${../Glide.js} Glide.js
      cp ${../../../tests/lib/Expect.js} Expect.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml glide.qml
      touch $out
    '';

  # Each kind of list keeps its one order with nothing typed (docs/design.md,
  # Lists): tests/order.qml holds launcher/Order.js's orders to the rule in
  # Qt's own engine, and tests/every-list-keeps-its-kinds-order.py walks each
  # place that lists things: its sort its kind's order, no A to Z by hand, and
  # no order of Order.js's sorted by Qt's Array.sort, which shuffles rows alike,
  # as a stand-in sorting three ways it refuses shows.
  each-list-keeps-its-kinds-order =
    let
      qt = pkgs.qt6.qtdeclarative;
      unstable = pkgs.writeText "Unstable.js" ''
        const byName = Order.byName(one => one.name);
        const named = rows => rows.sort(byName);
        const live = rows => rows.sort(
            Order.live({ stands: one => one.state, name: one => one.name }));
        const wrapped = rows => rows.sort((a, b) => Order.byName(one => one.name)(a, b));
      '';
    in
    pkgs.runCommandLocal "each-list-keeps-its-kinds-order" { } ''
      lint="${lib.getExe pkgs.python3} ${../../../tests/every-list-keeps-its-kinds-order.py}"
      $lint ${../launcher} ${../../people}
      mkdir unstable
      cp ${unstable} unstable/Unstable.js
      if $lint unstable 2> said; then
        echo "a sort by Order.js through Array.sort passed" >&2
        exit 1
      fi
      for line in 2 3 5; do
        if ! grep -qF "Unstable.js:$line sorts by Order.js with Qt's Array.sort" said; then
          echo "Unstable.js:$line's sort by Order.js through Array.sort wasn't refused:" >&2
          cat said >&2
          exit 1
        fi
      done
      cp ${../../../tests/order.qml} order.qml
      cp ${../launcher/Order.js} Order.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml order.qml
      touch $out
    '';

  # A row an act asked a daemon to change is drawn as it will be until its
  # source agrees, as tests/becoming.qml holds launcher/Becoming.js to it in
  # Qt's own engine.
  an-act-shows-before-its-daemon-answers =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "an-act-shows-before-its-daemon-answers" { } ''
      cp ${../../../tests/becoming.qml} becoming.qml
      cp ${../launcher/Becoming.js} Becoming.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml becoming.qml
      touch $out
    '';

  # An open launcher takes a burst of its sources' changes a pace apart, and
  # never two refreshes closer than that however the first was asked for, and
  # a source's rows are wanted only where they're shown or searched, and a
  # key's act is said shown by the first frame begun after its refresh, and
  # an opening weighs the foot's verbs only once its words or face change,
  # and a daemon's changes are said presented by place, as tests/pacing.qml
  # holds launcher/Pacing.js, launcher/Presented.js and launcher/Wanted.js
  # to it in Qt's own engine.
  the-launcher-paces-its-refreshes =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "the-launcher-paces-its-refreshes" { } ''
      cp ${../../../tests/pacing.qml} pacing.qml
      cp ${../launcher/Order.js} Order.js
      cp ${../launcher/Pacing.js} Pacing.js
      cp ${../launcher/Presented.js} Presented.js
      cp ${../launcher/Wanted.js} Wanted.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml pacing.qml
      touch $out
    '';

  # A thing's drawer says each of its parts as a fact, leaving out its
  # acts and opening a holder of figures in place, and a screen's says
  # what niri says it runs, where it sits and what it offers, and what the
  # backlight daemon says sets its level and has learned, as
  # tests/plate.qml holds launcher/Plate.js, Screens.js and Backlight.js to
  # it in Qt's own engine, with the Numbers.js Backlight.js says lux through
  # and the Order.js it lists devices by.
  a-things-drawer-says-its-parts =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    # Kept in Nix because each line copies, or names in a variable, a file
    # Nix gives it, and the cases it runs are a QML file of their own
    # (tests/plate.qml).
    pkgs.runCommandLocal "a-things-drawer-says-its-parts" { } ''
      cp ${../../../tests/plate.qml} plate.qml
      cp ${../launcher/Plate.js} Plate.js
      cp ${../launcher/Screens.js} Screens.js
      cp ${../launcher/Backlight.js} Backlight.js
      cp ${../launcher/Numbers.js} Numbers.js
      cp ${../launcher/Order.js} Order.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml plate.qml
      touch $out
    '';

  # Each opening of the launcher is kept as what it took: its keys by kind,
  # its pick and how long it took, and a miss only where letters were typed
  # and nothing picked, as tests/trials.qml holds launcher/Trials.js to it in
  # Qt's own engine.
  the-launcher-keeps-its-trials =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "the-launcher-keeps-its-trials" { } ''
      cp ${../../../tests/trials.qml} trials.qml
      cp ${../launcher/Trials.js} Trials.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml trials.qml
      touch $out
    '';

  # Each close adds its own opening's line to the file's end and rewrites
  # none before it, so a close costs the same however many are kept: it
  # turned all 1,339 of Joel's into text, 39 ms a close at a load of 66
  # (2026-11-10). The shell starting carries the old list into lines and
  # lets the oldest go (tests/trials-kept.qml, through Quickshell).
  the-launcher-keeps-its-trials-one-by-one =
    # procps gives the case pkill, to end the pipe as a full disk would.
    pkgs.runCommandLocal "the-launcher-keeps-its-trials-one-by-one"
      { nativeBuildInputs = [ pkgs.procps ]; }
      ''
        export HOME=$TMPDIR XDG_STATE_HOME=$PWD/state LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen
        export XDG_RUNTIME_DIR=$TMPDIR/runtime FONTCONFIG_FILE=${noFonts}
        mkdir -m 700 launcher $XDG_RUNTIME_DIR
        cp ${../Trials.qml} Trials.qml
        cp ${../launcher/Trials.js} launcher/Trials.js
        cp ${../../../tests/trials-kept.qml} shell.qml
        cp ${../../../tests/lib/Expect.js} Expect.js
        ${shellFolderLint}
        timeout 60 ${qs} -p shell.qml
        touch $out
      '';

  # Each opening of the launcher is a trace: its steps, backs, acts and
  # searches spans under it, a void above a list's first row told from a
  # list scrolled down, and the whole sent in OTLP's JSON mapping, as
  # tests/traces.qml holds launcher/Traces.js to it in Qt's own engine.
  the-launcher-traces-each-opening =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "the-launcher-traces-each-opening" { } ''
      cp ${../../../tests/traces.qml} traces.qml
      cp ${../launcher/Traces.js} Traces.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml traces.qml
      touch $out
    '';

  # A row the launcher listed before is kept wherever it moves, and only a
  # row new to the list is drawn afresh, as tests/rows-kept.qml holds
  # launcher/RowsKept.js to it in Qt's own engine.
  a-row-kept-is-never-drawn-again =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "a-row-kept-is-never-drawn-again" { } ''
      cp ${../../../tests/rows-kept.qml} rows-kept.qml
      cp ${../launcher/RowsKept.js} RowsKept.js
      cp ${../../../tests/lib/Expect.js} Expect.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml rows-kept.qml
      touch $out
    '';

  # The launcher answers what's asked: sums and number bases, worked out by
  # the shell's maths as the shell works them out, IPv4 and IPv6
  # addresses and networks with the kind of range each is in, subnet maths
  # (a network's parts, what holds an address, a prefix alone and a network
  # split), characters and their codes, named as Unicode names them, hardware
  # addresses, measures, the time in any zone tzdata names and the
  # machine's figures from its files, its parts' heat from their sensors,
  # the keyboards in the kernel's list of input devices, how each Bluetooth
  # link is doing from what bluetooth-links writes, what Sound makes of a
  # sound device's properties, the machine's connections from what ip, the
  # routing tables, resolv.conf and wifid say, and how WARP's link stands from
  # the record warp prints (modules/warp/warp.json), as tests/answers.qml
  # lists them, and leaves alone what isn't one, in Qt's own JavaScript
  # engine.
  the-launcher-answers-what-is-asked =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "the-launcher-answers-what-is-asked" { } ''
      cp ${../../../tests/answers.qml} answers.qml
      cp ${../launcher/Numbers.js} Numbers.js
      cp ${../launcher/Addresses.js} Addresses.js
      cp ${../launcher/Subnets.js} Subnets.js
      cp ${../launcher/Characters.js} Characters.js
      cp ${import ../launcher/names.nix { inherit lib pkgs; }} names.txt
      cp ${../launcher/Units.js} Units.js
      cp ${../launcher/Zones.js} Zones.js
      cp ${../launcher/Moment.js} Moment.js
      cp ${../launcher/Keeping.js} Keeping.js
      cp ${../launcher/Warmth.js} Warmth.js
      cp ${../launcher/Here.js} Here.js
      cp ${../launcher/Colours.js} Colours.js
      cp ${../launcher/Asking.js} Asking.js
      cp ${../launcher/Machine.js} Machine.js
      cp ${../launcher/Heat.js} Heat.js
      cp ${../launcher/Power.js} Power.js
      cp ${../launcher/Seen.js} Seen.js
      cp ${../launcher/Awake.js} Awake.js
      cp ${../launcher/Standing.js} Standing.js
      cp ${../launcher/Traffic.js} Traffic.js
      cp ${../launcher/Interfaces.js} Interfaces.js
      cp ${../launcher/Networkd.js} Networkd.js
      cp ${../launcher/WifiLink.js} WifiLink.js
      cp ${../launcher/Links.js} Links.js
      cp ${../launcher/Sounds.js} Sounds.js
      cp ${../launcher/Cameras.js} Cameras.js
      cp ${../launcher/Warp.js} Warp.js
      cp ${../launcher/WarpSeen.js} WarpSeen.js
      cp ${../launcher/WarpSwitch.js} WarpSwitch.js
      cp ${../launcher/HomeNetwork.js} HomeNetwork.js
      cp ${../launcher/Ago.js} Ago.js
      cp ${../launcher/words.json} words.json
      cp ${../../warp/warp.json} warp.json
      cp ${../launcher/Keyboards.js} Keyboards.js
      cp ${../launcher/KeyCounts.js} KeyCounts.js
      cp ${../launcher/Order.js} Order.js
      cp ${../../../tests/colour-cases.js} colour-cases.js
      cp ${import ../launcher/zones.nix { inherit lib pkgs; }} zones.jsonl
      export QML_XHR_ALLOW_FILE_READ=1
      # A zone with summer time, for the day of the year.
      export TZ=America/Toronto TZDIR=${pkgs.tzdata}/share/zoneinfo
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      # A script error never reaches Qt.exit, so without a limit the check
      # would wait forever rather than fail.
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml:${mathsModules} timeout 120 ${qt}/bin/qml answers.qml
      touch $out
    '';

  # The launcher's sums and measures, through the shell's maths, take a
  # typical key well inside its budget, each input typed a letter at a time
  # (tests/maths-speed.qml), which builds the maths' module too.
  the-launcher-sums-within-a-key =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "the-launcher-sums-within-a-key" { } ''
      cp ${../../../tests/maths-speed.qml} maths-speed.qml
      cp ${../launcher/Numbers.js} Numbers.js
      cp ${../launcher/Units.js} Units.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml:${mathsModules} timeout 300 ${qt}/bin/qml maths-speed.qml
      touch $out
    '';

  # A long name slides at one reading pace between soft ends, as
  # tests/reading.qml says, in Qt's own engine.
  a-long-name-reads-at-one-pace =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "a-long-name-reads-at-one-pace" { } ''
      cp ${../../../tests/reading.qml} reading.qml
      cp ${../Reading.js} Reading.js
      cp ${../Measures.js} Measures.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml reading.qml
      touch $out
    '';

  # The launcher keeps the rows a daemon serves ready in the order each
  # row says, placing each a change names and letting those gone go, as
  # tests/placed.qml holds launcher/Placed.js to rows made up there.
  the-launcher-places-each-row-it-is-served =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "the-launcher-places-each-row-it-is-served" { } ''
      mkdir launcher
      cp ${../../../tests/placed.qml} placed.qml
      cp ${../launcher/Placed.js} launcher/Placed.js
      cp ${../../../tests/lib/Expect.js} Expect.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml placed.qml
      touch $out
    '';

  # The shell takes a place's rows as its daemon serves them ready, placing
  # only its rows by their order and wording every time form, side, note
  # and drawer, as tests/ready-rows.qml holds launcher/ReadyRows.js to
  # records made up there.
  the-launcher-words-each-ready-row-it-is-served =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    # Kept in Nix because each line copies a file Nix gives it beside the
    # cases, or sets what Qt's engine runs them with.
    pkgs.runCommandLocal "the-launcher-words-each-ready-row-it-is-served" { } ''
      mkdir launcher
      cp ${../../../tests/ready-rows.qml} ready-rows.qml
      cp ${../launcher/ReadyRows.js} launcher/ReadyRows.js
      cp ${../launcher/Ago.js} launcher/Ago.js
      cp ${../launcher/Interfaces.js} launcher/Interfaces.js
      cp ${../When.js} When.js
      cp ${../../../tests/lib/Expect.js} Expect.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml ready-rows.qml
      touch $out
    '';

  # A card says how it stands in one status line, the launcher's and held
  # notices' alike, as tests/status-line.qml holds StatusLine.js to: which
  # hints give way as room runs short, the keys list's and Return's staying.
  a-card-says-how-it-stands-in-one-row =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "a-card-says-how-it-stands-in-one-row" { } ''
      cp ${../../../tests/status-line.qml} status-line.qml
      cp ${../StatusLine.js} StatusLine.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml status-line.qml
      touch $out
    '';

  # A program's act on a copy or on the repositories under ~/src shows only
  # while it can act, and opens what it says: what decides it, in Qt's own
  # engine (tests/quick-acts.qml, over Copied.js, launcher/Repositories.js
  # and Neovim's NeovimOpening.js and Neovim.js), then each program's
  # provider with the shell's one watcher of the clipboard, through
  # Quickshell itself, over repositories made here and read once for both
  # programs by the real programs daemon (launcher/Repositories.qml),
  # with niri and the clipboard's history stood in (tests/program-acts.qml).
  # Chrome's downloads are read from Chrome's ChromeDownloads.js in Qt's
  # engine and through its provider from the real programs daemon, running
  # a stand-in for chrome-recent-sites, once Chrome is stepped into: file
  # rows in its Recent, each drawer saying where it is then its kind and
  # previewing a picture, and Open
  # the Latest Download only while there's one, waiting in its place until
  # the first reading answers. Neovim's files open now come through the same
  # daemon from a stand-in open-now, whose reading of a real Neovim is open-with's own tests', each
  # file's drawer saying where it is and its buffer.
  the-launcher-acts-only-on-what-it-can =
    let
      qt = pkgs.qt6.qtdeclarative;
      # A kitty used last, whose remote control Neovim opens through.
      niri = singleton "Niri.qml" ''
        property var windows: ({
          "4": { id: 4, app_id: "kitty", pid: 40, focus_timestamp: { secs: 10, nanos: 0 } },
          "5": { id: 5, app_id: "kitty", pid: 41, focus_timestamp: { secs: 20, nanos: 0 } }
        })
        property var spawned: []
        property var acted: []
        function spawn(command) { spawned = command; }
        function act(action) { acted = acted.concat([action]); }
      '';
      # What open-now says two Neovims in one kitty hold, as
      # tests/neovim-open-now.sh says.
      openNow = pkgs.writeShellScript "open-now" (builtins.readFile ../../../tests/neovim-open-now.sh);
      # file-preview stood in, saying which process answered.
      previewer = pkgs.writeShellScript "file-preview" (builtins.readFile ../../../tests/file-preview.sh);
      paste = pkgs.writeShellScriptBin "wl-paste" "exec sleep 300";
      programs = pkgs.callPackage ../../programs/package.nix { };
      # Chrome's downloads, as chrome-recent-sites prints them: one from a
      # page, finished at a time of its own, one Chrome kept no time or
      # page for, and a picture; the programs daemon runs it as Chrome's
      # module has it run its own.
      downloads = pkgs.writeShellScript "downloads-list" ''
        printf '%s\t%s\t%s\t%s\n' "$HOME/Downloads/report.pdf" 2048 1800000100 \
          'https://docs.example.com/q3?x=1' "$HOME/Downloads/undated.zip" 10 "" "" \
          "$HOME/Downloads/tide-chart.png" 84300 1800000050 'https://tide.example.org/'
      '';
      sources = pkgs.writeText "sources.json" (
        builtins.toJSON [
          {
            app = "google-chrome";
            name = "downloads";
            asked = true;
            list = [ "${downloads}" ];
          }
          # What each Neovim has open, which the programs daemon reads as
          # Neovim's module has it read.
          {
            app = "nvim";
            name = "open";
            asked = true;
            list = [
              "${openNow}"
              "neovim"
            ];
          }
        ]
      );
      # Three repositories a folder deep, alpha's git written last, alpha's
      # origin and gamma's upstream on GitHub and beta's on GitLab, and one
      # four folders deep, past where they're looked for; real ones, since
      # git lists a repository's files.
      git = lib.getExe pkgs.git;
      repositories = ''
        repository() {
          ${git} init --quiet "$HOME/src/$1"
          ${git} -C "$HOME/src/$1" remote add "$2" "$3"
          touch -d "@$4" "$HOME/src/$1/.git"
        }
        mkdir -p $HOME/src/alpha/docs $HOME/src/beta
        printf 'fn main() {}\n' > $HOME/src/alpha/main.rs
        printf 'KEY=1\n' > $HOME/src/alpha/.env
        printf '# Notes\n' > $HOME/src/alpha/docs/notes.md
        printf 'ignored.txt\n' > $HOME/src/alpha/.gitignore
        printf 'not listed\n' > $HOME/src/alpha/ignored.txt
        touch -d @1700000400 $HOME/src/alpha/main.rs
        touch -d @1700000300 $HOME/src/alpha/.env
        touch -d @1700000200 $HOME/src/alpha/docs/notes.md
        touch -d @1700000100 $HOME/src/alpha/.gitignore
        printf '# Beta\n' > $HOME/src/beta/README.md
        repository alpha origin git@github.com:Jitsusama/alpha.git 1700000900
        repository beta origin https://gitlab.com/someone/beta 1700000800
        repository gamma upstream https://github.com/NixOS/gamma.git 1700000700
        repository far/away/too/deep origin git@github.com:someone/deep.git 1700000950
        repository .hidden/omega origin git@github.com:someone/omega.git 1700000960
        # A document outside them, for zathura's act.
        printf '%%PDF-1.4\n' > $HOME/report.pdf
      '';
    in
    pkgs.runCommandLocal "the-launcher-acts-only-on-what-it-can" { } ''
      mkdir cases shell
      cd $TMPDIR/cases
      cp ${../../../tests/quick-acts.qml} quick-acts.qml
      cp ${../Copied.js} Copied.js
      cp ${../launcher/Repositories.js} Repositories.js
      cp ${../../neovim/NeovimOpening.js} NeovimOpening.js
      cp ${../../chrome/ChromeDownloads.js} ChromeDownloads.js
      cp ${../../neovim/Neovim.js} Neovim.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml quick-acts.qml
      cd $TMPDIR/shell
      mkdir -p launcher
      cp ${(account graphical).xdg.configFile."quickshell/Theme.qml".source} Theme.qml
      cp ${../Fuzzy.js} Fuzzy.js
      cp ${../ProgramGlyphs.js} ProgramGlyphs.js
      cp ${../Words.qml} Words.qml
      cp ${../Files.qml} Files.qml
      cp ${../FileFacts.js} FileFacts.js
      cp ${../When.js} When.js
      cp ${../Copied.qml} Copied.qml
      cp ${../Copied.js} Copied.js
      cp ${niri} Niri.qml
      cp ${../launcher/words.json} launcher/words.json
      cp ${../launcher/Provider.qml} launcher/Provider.qml
      cp ${../launcher/Asking.js} launcher/Asking.js
      cp ${../launcher/Numbers.js} launcher/Numbers.js
      cp ${../launcher/Ago.js} launcher/Ago.js
      cp ${../launcher/Repositories.qml} launcher/Repositories.qml
      cp ${../launcher/Repositories.js} launcher/Repositories.js
      cp ${../../neovim/NeovimProgram.qml} launcher/NeovimProgram.qml
      cp ${../launcher/Previewer.qml} launcher/Previewer.qml
      cp ${../launcher/Captured.js} launcher/Captured.js
      cp ${../launcher/Recent.js} launcher/Recent.js
      cp ${../launcher/Order.js} launcher/Order.js
      cp ${../../neovim/NeovimOpening.js} launcher/NeovimOpening.js
      cp ${../../neovim/Neovim.js} launcher/Neovim.js
      cp ${../../chrome/ChromeProgram.qml} launcher/ChromeProgram.qml
      cp ${../../chrome/ChromeDownloads.js} launcher/ChromeDownloads.js
      cp ${../Daemon.qml} Daemon.qml
      cp ${../Hearing.js} Hearing.js
      cp ${../launcher/Listed.qml} launcher/Listed.qml
      cp ${../../reader/ReaderProgram.qml} launcher/ReaderProgram.qml
      cp ${../../zathura/ZathuraProgram.qml} launcher/ZathuraProgram.qml
      cp ${../../zathura/Zathura.js} launcher/Zathura.js
      cp ${../../reader/Reader.js} launcher/Reader.js
      cp ${../../gimp/CopiedPicture.qml} launcher/CopiedPicture.qml
      cp ${../../../tests/program-acts.qml} shell.qml
      cp ${../../../tests/lib/Frames.js} Frames.js
      export XDG_RUNTIME_DIR=$TMPDIR/runtime
      export PATH=${
        lib.makeBinPath [
          paste
          pkgs.fd
          pkgs.coreutils
          pkgs.gnugrep
        ]
      }:$PATH
      export OPEN_NOW=${openNow}
      mkdir -m 700 $XDG_RUNTIME_DIR
      ${repositories}
      mkdir $XDG_RUNTIME_DIR/daemons
      XDG_DATA_HOME=$TMPDIR/data XDG_DATA_DIRS=$TMPDIR/data PROGRAMS_GIT=${git} \
        PROGRAMS_PREVIEW=${previewer} \
        ${programs}/bin/programs $XDG_RUNTIME_DIR/daemons/programs.sock ${sources} &
      daemon=$!
      ${shellFolderLint}
      timeout 60 ${qs} -p shell.qml
      kill $daemon
      touch $out
    '';
}
