# The checks of the launcher's words: one meaning a word, its commands, terms
# and the names of its parts, which the folder's checks.nix gathers with every
# other place's; its opening comment says why each place keeps its own.
{
  account,
  graphical,
  lib,
  noFonts,
  pkgs,
  qs,
  shellFolderLint,
  singleton,
  ...
}:
{
  # The launcher's vocabulary, as the account writes it, says each word of
  # one thing, but where it's listed as meant to find several, names only
  # what it knows and glyphs the theme has, and gives everything a place
  # (tests/launcher-words.py), printing each word to settle; and its own
  # file gives no key twice in one entry, which the account's copy hides.
  the-launcher-means-one-thing-a-word =
    let
      home = account graphical;
      marks = pkgs.writeText "marks.json" (builtins.toJSON (lib.attrNames home.jitsusama.theme.marks));
      words = pkgs.writeText "words.json" home.xdg.configFile."quickshell/launcher/words.json".text;
    in
    pkgs.runCommandLocal "the-launcher-means-one-thing-a-word" { } ''
      ${lib.getExe pkgs.python3} ${../../../tests/launcher-words.py} ${words} ${marks} \
        ${../launcher/words.json}
      touch $out
    '';

  # The words each daemon's tests are given are the vocabulary's own, as the
  # account writes it (tests/daemon-words.py), so no daemon passes its tests
  # on words Joel never reads. A daemon whose tests hold a slice of the
  # vocabulary adds it here.
  the-daemons-test-the-launchers-words =
    let
      words =
        pkgs.writeText "words.json"
          (account graphical).xdg.configFile."quickshell/launcher/words.json".text;
    in
    pkgs.runCommandLocal "the-daemons-test-the-launchers-words" { } ''
      cp ${../../to-do/tests/words.json} to-do.json
      cp ${../whereabouts/tests/words.json} whereabouts.json
      cp ${../../windows/tests/screen-words.json} windows-screens.json
      cp ${../../machine/tests/words.json} machine.json
      ${lib.getExe pkgs.python3} ${../../../tests/daemon-words.py} ${words} \
        to-do.json whereabouts.json windows-screens.json machine.json
      touch $out
    '';

  # Every launcher verb names the command the : line runs it by, one the key
  # language has, with what follows it, and no two on a row alike
  # (tests/launcher-commands.py), as the account writes the vocabulary.
  the-launcher-names-every-action-a-command =
    let
      words =
        pkgs.writeText "words.json"
          (account graphical).xdg.configFile."quickshell/launcher/words.json".text;
      language = ../../keys/language.toml;
    in
    pkgs.runCommandLocal "the-launcher-names-every-action-a-command" { } ''
      ${lib.getExe pkgs.python3} ${../../../tests/launcher-commands.py} ${words} ${language}
      touch $out
    '';

  # What the launcher shows says the glossary's words, never a name it
  # replaces, but what tests/launcher-shown.py lists as another sense or a
  # rename still waiting.
  the-launcher-shows-no-replaced-word =
    let
      words =
        pkgs.writeText "words.json"
          (account graphical).xdg.configFile."quickshell/launcher/words.json".text;
    in
    pkgs.runCommandLocal "the-launcher-shows-no-replaced-word" { } ''
      cp ${../../../tests/repository/glossary.py} glossary.py
      cp ${../../../tests/repository/names.py} names.py
      cp ${../../../tests/launcher-shown.py} launcher-shown.py
      ${lib.getExe pkgs.python3} launcher-shown.py ${../../../docs/glossary.md} ${words}
      touch $out
    '';

  # A launcher row that names a glossary thing says which with its term,
  # and is called by it, as tests/launcher-terms.py holds each one.
  the-launcher-names-each-thing-its-term =
    let
      words =
        pkgs.writeText "words.json"
          (account graphical).xdg.configFile."quickshell/launcher/words.json".text;
    in
    pkgs.runCommandLocal "the-launcher-names-each-thing-its-term" { } ''
      cp ${../../../tests/repository/glossary.py} glossary.py
      cp ${../../../tests/launcher-terms.py} launcher-terms.py
      ${lib.getExe pkgs.python3} launcher-terms.py ${../../../docs/glossary.md} ${words}
      touch $out
    '';

  # The launcher's catalogue, what pi-ask's tool lists and runs, tells a
  # place from an action and offers what ends something rather than doing
  # it, as tests/catalogue.qml holds launcher/Catalogue.js to it in Qt's
  # own engine.
  the-launcher-catalogues-what-it-does =
    let
      qt = pkgs.qt6.qtdeclarative;
      # The key language's commands by their whole names, each with whether
      # it changes things, ends something or starts something, as the
      # shell's Options.commands holds them.
      named =
        lib.filterAttrs (_: about: about ? command)
          (builtins.fromTOML (builtins.readFile ../../keys/language.toml)).acts;
      commands = pkgs.writeText "commands.json" (
        builtins.toJSON (
          lib.mapAttrs' (
            _: about:
            lib.nameValuePair about.command {
              changes = about.changes or false;
              ends = about.ends or false;
              starts = about.starts or false;
            }
          ) named
        )
      );
    in
    pkgs.runCommandLocal "the-launcher-catalogues-what-it-does" { } ''
      cp ${../../../tests/catalogue.qml} catalogue.qml
      cp ${../launcher/Catalogue.js} Catalogue.js
      cp ${commands} commands.json
      export QML_XHR_ALLOW_FILE_READ=1
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml catalogue.qml
      touch $out
    '';

  # The launcher's answers that build their parts' ids as they answer, the
  # calculator's bases, the time's, today's, the waits', a hardware
  # address's, a network's and a character's, name every part as the
  # vocabulary does (tests/part-names.qml), with tzdata's zones and
  # Unicode's names kept and lag-watch's last ten seconds stood in for, as
  # the machine daemon would tell them.
  the-launcher-names-the-parts-it-builds =
    pkgs.runCommandLocal "the-launcher-names-the-parts-it-builds" { }
      ''
        mkdir -p launcher runtime && chmod 700 runtime
        cp ${singleton "Watchers.qml" ''
          property var lag: ({ trouble: [], niri: { p50_ms: 4, p99_ms: 21 }, band: "calm",
            ended: 0 })
        ''} Watchers.qml
        cp ${../Fuzzy.js} Fuzzy.js
        cp ${../launcher/Provider.qml} launcher/Provider.qml
        cp ${../ProgramGlyphs.js} ProgramGlyphs.js
        cp ${../launcher/Asking.js} launcher/Asking.js
        cp ${../launcher/Numbers.js} launcher/Numbers.js
        cp ${../launcher/Zones.js} launcher/Zones.js
        cp ${../launcher/Ago.js} launcher/Ago.js
        cp ${../launcher/Addresses.js} launcher/Addresses.js
        cp ${../launcher/Calculator.qml} launcher/Calculator.qml
        cp ${../launcher/Clocks.qml} launcher/Clocks.qml
        cp ${../launcher/Keeping.js} launcher/Keeping.js
        cp ${../launcher/Today.qml} launcher/Today.qml
        cp ${../launcher/Moment.js} launcher/Moment.js
        cp ${../launcher/Waits.qml} launcher/Waits.qml
        cp ${../launcher/Vendors.qml} launcher/Vendors.qml
        cp ${../launcher/Interfaces.js} launcher/Interfaces.js
        cp ${../launcher/Networkd.js} launcher/Networkd.js
        cp ${../launcher/Subnets.js} launcher/Subnets.js
        cp ${../launcher/Networks.qml} launcher/Networks.qml
        cp ${../launcher/Order.js} launcher/Order.js
        cp ${../launcher/Characters.js} launcher/Characters.js
        cp ${../launcher/Characters.qml} launcher/Characters.qml
        cp ${../Daemon.qml} Daemon.qml
        cp ${../Hearing.js} Hearing.js
        cp ${import ../launcher/names.nix { inherit lib pkgs; }} names.txt
        cp ${(account graphical).xdg.configFile."quickshell/Theme.qml".source} Theme.qml
        cp ${../Words.qml} Words.qml
        cp ${../launcher/words.json} launcher/words.json
        cp ${import ../launcher/zones.nix { inherit lib pkgs; }} zones.jsonl
        cp ${singleton "Location.qml" ''
          property var here: null
          property string clockZone: ""
          property bool clockMoved: false
          property var moved: null
          property string movedSaid: ""
          function city(zone) { return zone; }
          function putBack() {}
        ''} Location.qml
        cp ${singleton "Timekeeping.qml" ''
          property int watching: 0
          property bool refused: false
          property var reading: (${builtins.readFile ../../../tests/timekeeping-reading.json})
        ''} Timekeeping.qml
        cp ${singleton "Calendars.qml" ''
          property string asked: ""
          property bool refused: false
          // A day's reading as timekeeping gives it, said of today, whichever
          // day the check runs on.
          property var reading: Object.assign((${builtins.readFile ../timekeeping/calendars-reading.json}),
            { day: Qt.formatDate(new Date(), "yyyy-MM-dd") })
          function ask(day) { asked = day; }
        ''} Calendars.qml
        cp ${../../../tests/part-names.qml} shell.qml
        export HOME=$TMPDIR XDG_RUNTIME_DIR=$TMPDIR/runtime LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen
        export TZ=America/Toronto TZDIR=${pkgs.tzdata}/share/zoneinfo
        export FONTCONFIG_FILE=${noFonts}
        ${shellFolderLint}
        timeout 60 ${qs} -p shell.qml
        touch $out
      '';
}
