# The checks of quickshell, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  account,
  bareNixos,
  graphical,
  holds,
  lib,
  onLinux,
  passesClippy,
  pkgs,
  quickshellTypes,
  self,
  shellFolderLint,
  timedQml,
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
  # Quickshell's runner, the one the account's shell runs on.
  qs = lib.getExe' graphical.pkgs.quickshell "qs";
  # The launcher's maths, a QML module the shell imports as Maths
  # (maths/package.nix), and where Qt finds it.
  mathsModules = "${pkgs.callPackage ./maths/package.nix { }}/lib/qt-6/qml";
  # Fonts for a shell drawn offscreen, which needs none of its own.
  noFonts = pkgs.makeFontsConf { fontDirectories = [ ]; };

  # The account's notices on its real niri, with its own configuration,
  # nested in a headless sway in the sandbox rather than in a machine
  # (tests/nested-niri.sh): a script run against them, given the shell's
  # folder, the logs and what more it's given. The account says what its
  # shell does, as the desktop test's does, and counts as away after so
  # many seconds of nothing typed or moved.
  noticesOnNiri =
    {
      name,
      away,
      script,
      given ? [ ],
    }:
    let
      home = account (bareNixos [
        self.nixosModules.graphical
        {
          home-manager.users.someone.jitsusama.quickshell = {
            reports = true;
            notices.away = away;
          };
        }
      ]);
      shell = lib.filterAttrs (name: _: lib.hasPrefix "quickshell/" name) home.xdg.configFile;
      niri = lib.filterAttrs (name: _: lib.hasPrefix "niri/" name) home.xdg.configFile;
      copied =
        files:
        lib.concatLines (
          lib.mapAttrsToList (name: file: "install -D -m 644 ${file.source} ${lib.escapeShellArg name}") files
        );
      inherit (pkgs) mesa;
    in
    pkgs.runCommandLocal name
      {
        nativeBuildInputs = [
          graphical.config.programs.niri.package
          graphical.pkgs.quickshell
          pkgs.dbus
          pkgs.libnotify
          pkgs.sway-unwrapped
        ];
        LANG = "C.UTF-8";
        FONTCONFIG_FILE = pkgs.makeFontsConf { fontDirectories = graphical.config.fonts.packages; };
        # niri and Qt draw in software, as the desktop's machine does.
        __EGL_VENDOR_LIBRARY_FILENAMES = "${mesa}/share/glvnd/egl_vendor.d/50_mesa.json";
        LIBGL_DRIVERS_PATH = "${mesa}/lib/dri";
        GBM_BACKENDS_PATH = "${mesa}/lib/gbm";
        LIBGL_ALWAYS_SOFTWARE = 1;
        QT_QPA_PLATFORM = "wayland";
        QT_QUICK_BACKEND = "software";
      }
      ''
        export HOME=$PWD XDG_CONFIG_HOME=$PWD XDG_STATE_HOME=$PWD/state
        ${copied shell}
        ${copied niri}
        install -m 644 ${../../tests/notices.qml} quickshell/shell.qml
        ${lib.getExe pkgs.bash} ${../../tests/nested-niri.sh} niri/config.kdl logs \
          ${lib.getExe pkgs.bash} ${script} quickshell logs ${lib.escapeShellArgs given}
        touch $out
      '';
in
# The shell runs only on Linux, so a Mac has none of its checks: most build
# Quickshell or a program only Linux has, and the rest read a Linux machine,
# which Linux's own checks already do.
onLinux {
  # The notices draw a notice a program sends as a card on a layer niri
  # lists, and Super+Shift+N puts away one that stays
  # (tests/notices-on-niri.sh). Away only after an hour, since nobody types
  # here but the check.
  the-notices-land-on-niri = noticesOnNiri {
    name = "the-notices-land-on-niri";
    away = 3600;
    script = ../../tests/notices-on-niri.sh;
    # What Super+Shift+N runs, as the account's niri binds it: niri in a
    # window reads keys through its own keymap, and no keyboard here types
    # into it, so the check runs the bind's own command.
    given =
      let
        bind = lib.findFirst (
          bind: bind.chord == "Super+Shift+N"
        ) (throw "Nothing puts the notices away.") (account graphical).jitsusama.niri.binds;
        spawned = builtins.match "spawn (.*);" bind.action;
      in
      assert spawned != null;
      lib.filter (word: word != "" && word != " ") (lib.splitString "\"" (lib.head spawned));
  };

  # A notice that comes while Joel's away waits, past the time its sender
  # gave it (tests/notices-wait-while-away.sh). Away after a second, so he
  # is before it comes.
  the-notices-wait-while-joel-is-away = noticesOnNiri {
    name = "the-notices-wait-while-joel-is-away";
    away = 1;
    script = ../../tests/notices-wait-while-away.sh;
  };

  # A switch restarts Quickshell when any file it reads changes, so the shell
  # on screen is never the one from before the switch.
  quickshell-restarts-with-its-files =
    let
      home = account graphical;
      files = lib.filterAttrs (name: _: lib.hasPrefix "quickshell/" name) home.xdg.configFile;
      triggers = home.systemd.user.services.quickshell.Unit.X-Restart-Triggers or [ ];
    in
    holds "quickshell-restarts-with-its-files"
      "A Quickshell file isn't among its unit's restart triggers, so a switch would leave the old one on screen."
      (files != { } && lib.all (file: lib.elem "${file.source}" triggers) (lib.attrValues files));

  # A machine shapes what the overview shows in a line a change: a piece
  # moved, cut, shown only while true, a threshold changed and the rest kept,
  # and a piece of its own linked beside the built-in ones.
  the-overview-takes-its-pieces =
    let
      own = pkgs.writeText "wide.qml" "Piece { value: settings.text }";
      machine = bareNixos [
        self.nixosModules.workstation
        self.nixosModules.graphical
        {
          home-manager.users.someone.jitsusama.quickshell.overview.pieces = {
            date.tier = null;
            network.order = 25;
            battery.low = 15;
            volume.tier = "when";
            wide = {
              order = 60;
              source = own;
              label = "wide";
              text = "a piece of the machine's own";
              collapse = 5;
              mark = "on";
            };
          };
        }
      ];
      home = account machine;
      rendered = lib.findFirst (lib.hasInfix "property var pieces: ") "" (
        lib.splitString "\n" home.xdg.configFile."quickshell/Overview.qml".text
      );
      drawn = builtins.fromJSON (lib.last (lib.splitString "property var pieces: " rendered));
      piece = lib.listToAttrs (map (piece: lib.nameValuePair piece.name piece) drawn);
    in
    holds "the-overview-takes-its-pieces"
      "The overview didn't draw a machine's own pieces as it set them: their order, their cuts, their settings or its own file."
      (
        map (piece: piece.name) drawn == [
          "time"
          "network"
          "battery"
          "volume"
          "spend"
          "pi"
          "notices"
          "google"
          "meeting"
          "quiet"
          "disk"
          "lag"
          "awake"
          "wide"
          "mail"
          "profile"
          "microphone"
          "capslock"
          "layer"
          "call"
          "meet"
          "sharing"
          "camera"
          "recording"
          "playing"
        ]
        && piece.battery.low == 15
        && piece.battery.critical == 10
        && piece.volume.tier == "when"
        && piece.wide.mark == "on"
        && piece.wide.file == "wide.qml"
        && "${home.xdg.configFile."quickshell/pieces/wide.qml".source}" == "${own}"
      );

  # qmllint reads the shell as the account gets it, and the greeter beside it,
  # as the module qs, against
  # Quickshell's own type files as quickshellTypes corrects them, and any
  # warning fails. Two kinds of report are left out, since each one here is
  # Quickshell's files, not the shell: a property typed by a name its files
  # don't export (AuthFlow, BluetoothAdapter and the like), and a handler
  # whose parameter is a Qt enum QML can't see (QProcess::ExitStatus).
  quickshell-passes-qmllint =
    let
      files =
        lib.filterAttrs (name: _: lib.hasPrefix "quickshell/" name)
          (account graphical).xdg.configFile;
      qt = pkgs.kdePackages.qtdeclarative;
    in
    # In UTF-8, which Qt otherwise switches to and says so.
    pkgs.runCommandLocal "quickshell-passes-qmllint" { LC_ALL = "C.UTF-8"; } ''
      ${lib.concatLines (
        lib.mapAttrsToList (name: file: ''
          mkdir -p "$(dirname ${lib.escapeShellArg name})"
          cp ${file.source} ${lib.escapeShellArg name}
        '') files
      )}
      mv quickshell qs
      # Slack's source for People is installed only where a machine brings
      # Slack, as no example does, so it's read here as if one had.
      mkdir -p qs/launcher/people
      cp ${../slack/SlackSource.qml} qs/launcher/people/slack.qml
      # The greeter runs beside the account's files, as greetd's module lays
      # them out, with what it's built with standing in.
      cp ${
        pkgs.replaceVars ../greetd/greeter.qml {
          accounts = builtins.toJSON [
            {
              name = "someone";
              called = "Someone";
            }
          ];
          session = builtins.toJSON [ "niri-session" ];
          analyze = "systemd-analyze";
        }
      } qs/greeter.qml
      # Quickshell finds a folder's files as a module by its path; qmllint
      # needs a qmldir saying so (tests/qmldir.sh).
      ${lib.getExe pkgs.bash} ${../../tests/qmldir.sh} qs
      ${qt}/bin/qmllint --ignore-settings --max-warnings 0 \
        -I ${quickshellTypes}/lib/qt-6/qml -I ${qt}/lib/qt-6/qml -I . \
        --unresolved-type disable --signal-handler-parameters disable \
        $(find qs -name '*.qml' | sort)
      touch $out
    '';

  # The shell hands every program it opens to niri, which gives it a scope of
  # its own. One the shell started itself would leave its helpers in the
  # shell's service, and restarting the shell would kill them.
  quickshell-opens-programs-through-niri =
    let
      dir = ./.;
      qml = lib.filter (lib.hasSuffix ".qml") (lib.attrNames (builtins.readDir dir));
      # Split on each literal rather than lib.hasInfix, whose ".*" pattern
      # recurses once a character and overflows the evaluator's stack on a
      # file past about 100 kB, as Launcher.qml is.
      holding = infix: file: builtins.length (builtins.split (lib.escapeRegex infix) file) > 1;
      starts = file: holding "execDetached" file || holding ".execute()" file;
      culprits = lib.filter (name: starts (builtins.readFile (dir + "/${name}"))) qml;
    in
    holds "quickshell-opens-programs-through-niri"
      "These start programs themselves rather than through Niri.spawn: ${lib.concatStringsSep ", " culprits}"
      (qml != [ ] && culprits == [ ]);

  # The launcher finds what's meant: for each query in tests/fuzzy.qml the
  # right thing comes first, few rows are left, or nothing answers, and a
  # query keeps within its budget in at least one of five processes, all in
  # Qt's own JavaScript engine, the one the shell runs the matcher in.
  the-launcher-finds-what-is-meant = pkgs.runCommandLocal "the-launcher-finds-what-is-meant" { } ''
    cp ${../../tests/fuzzy.qml} fuzzy.qml
    cp ${./Fuzzy.js} Fuzzy.js
    export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
    export FONTCONFIG_FILE=${noFonts}
    ${timedQml "fuzzy.qml"}
    touch $out
  '';

  # The launcher's vocabulary, as the account writes it, says each word of
  # one thing, but where it's listed as meant to find several, names only
  # what it knows and glyphs the theme has, and gives everything a place
  # (tests/launcher-words.py), printing each word to settle.
  the-launcher-means-one-thing-a-word =
    let
      home = account graphical;
      marks = pkgs.writeText "marks.json" (builtins.toJSON (lib.attrNames home.jitsusama.theme.marks));
      words = pkgs.writeText "words.json" home.xdg.configFile."quickshell/launcher/words.json".text;
    in
    pkgs.runCommandLocal "the-launcher-means-one-thing-a-word" { } ''
      ${lib.getExe pkgs.python3} ${../../tests/launcher-words.py} ${words} ${marks}
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
      cp ${../../tests/glossary.py} glossary.py
      cp ${../../tests/names.py} names.py
      cp ${../../tests/launcher-shown.py} launcher-shown.py
      ${lib.getExe pkgs.python3} launcher-shown.py ${../../docs/glossary.md} ${words}
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
      cp ${../../tests/glossary.py} glossary.py
      cp ${../../tests/launcher-terms.py} launcher-terms.py
      ${lib.getExe pkgs.python3} launcher-terms.py ${../../docs/glossary.md} ${words}
      touch $out
    '';

  # Shortcuts groups every layer's chords by what each acts on, as
  # tests/key-groups.qml holds launcher/KeyGroups.js to an account's own
  # binds, Neovim's, pi's, Chrome's and Slack's among them, in Qt's own
  # engine, each group named in the account's vocabulary; and steps through them in levels, as
  # tests/key-map.qml drives launcher/KeyMap.qml through Quickshell itself
  # over the same account's shortcuts, with niri stood in for. The picture
  # viewer is given a key of its own here, one no viewer would use, so its
  # layer is there to lead over its window whatever keys it has yet.
  the-launcher-groups-every-shortcut =
    let
      qt = pkgs.qt6.qtdeclarative;
      home = account (bareNixos [
        self.nixosModules.graphical
        {
          home-manager.users.someone = {
            imports = [
              self.homeModules.chrome
              self.homeModules.neovim
              self.homeModules.pi
              self.homeModules.slack
            ];
            jitsusama.keys.swayimg = [
              {
                chord = "Ctrl+Alt+F12";
                does = "Stand in for the viewer's own keys";
              }
            ];
          };
        }
      ]);
      fields = map (b: {
        inherit (b)
          chord
          keys
          does
          when
          run
          group
          ;
      });
      layers = pkgs.writeText "layers.json" (
        builtins.toJSON (lib.mapAttrs (_: fields) home.jitsusama.keys)
      );
      files = home.xdg.configFile;
      words = pkgs.writeText "words.json" files."quickshell/launcher/words.json".text;
      niri = pkgs.writeText "Niri.qml" ''
        pragma Singleton
        import Quickshell
        import QtQuick
        Singleton {
            property var spawned: []
            function spawn(command) { spawned = command; }
        }
      '';
    in
    pkgs.runCommandLocal "the-launcher-groups-every-shortcut" { } ''
      export QML_XHR_ALLOW_FILE_READ=1
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      mkdir groups shell
      cd $TMPDIR/groups
      cp ${../../tests/key-groups.qml} key-groups.qml
      cp ${./launcher/KeyGroups.js} KeyGroups.js
      cp ${layers} layers.json
      cp ${words} words.json
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml ${qt}/bin/qml key-groups.qml
      cd $TMPDIR/shell
      mkdir -p runtime launcher && chmod 700 runtime
      cp ${../../tests/key-map.qml} shell.qml
      cp ${./Fuzzy.js} Fuzzy.js
      cp ${./launcher/Provider.qml} launcher/Provider.qml
      cp ${./launcher/Asking.js} launcher/Asking.js
      cp ${./launcher/Numbers.js} launcher/Numbers.js
      cp ${./launcher/KeyMap.qml} launcher/KeyMap.qml
      cp ${./launcher/KeyGroups.js} launcher/KeyGroups.js
      cp ${files."quickshell/Theme.qml".source} Theme.qml
      cp ${files."quickshell/Shortcuts.qml".source} Shortcuts.qml
      cp ${./Words.qml} Words.qml
      cp ${words} launcher/words.json
      cp ${niri} Niri.qml
      export XDG_RUNTIME_DIR=$TMPDIR/shell/runtime
      ${shellFolderLint}
      timeout 60 ${qs} -p shell.qml
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
      cp ${../../tests/ranking.qml} ranking.qml
      cp ${./launcher/Rank.js} Rank.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml ${qt}/bin/qml ranking.qml
      touch $out
    '';

  # The launcher's catalogue, what pi-ask's tool lists and runs, tells a
  # place from an action and offers what ends something rather than doing
  # it, as tests/catalogue.qml holds launcher/Catalogue.js to it in Qt's
  # own engine.
  the-launcher-catalogues-what-it-does =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "the-launcher-catalogues-what-it-does" { } ''
      cp ${../../tests/catalogue.qml} catalogue.qml
      cp ${./launcher/Catalogue.js} Catalogue.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml catalogue.qml
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
      cp ${../../tests/steady.qml} steady.qml
      cp ${./launcher/Steady.js} Steady.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml steady.qml
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
      cp ${../../tests/trials.qml} trials.qml
      cp ${./launcher/Trials.js} Trials.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml trials.qml
      touch $out
    '';

  # The launcher finds emoji by what they're called and what they mean, best
  # first, as tests/emoji.qml lists them, over the list it's built with, and
  # in about three times the least a search could take.
  the-launcher-finds-emoji =
    let
      emoji = import ./launcher/emoji.nix { inherit lib pkgs; } "none";
    in
    pkgs.runCommandLocal "the-launcher-finds-emoji" { } ''
      mkdir launcher
      cp ${../../tests/emoji.qml} emoji.qml
      cp ${./Fuzzy.js} Fuzzy.js
      cp ${./launcher/Emoji.js} launcher/Emoji.js
      cp ${emoji} emoji.jsonl
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export QML_XHR_ALLOW_FILE_READ=1
      export FONTCONFIG_FILE=${noFonts}
      ${timedQml "emoji.qml"}
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
  # routing tables, resolv.conf and iw say, as tests/answers.qml lists them,
  # and leaves alone what isn't one, in Qt's own JavaScript engine.
  the-launcher-answers-what-is-asked =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "the-launcher-answers-what-is-asked" { } ''
      cp ${../../tests/answers.qml} answers.qml
      cp ${./launcher/Numbers.js} Numbers.js
      cp ${./launcher/Addresses.js} Addresses.js
      cp ${./launcher/Subnets.js} Subnets.js
      cp ${./launcher/Characters.js} Characters.js
      cp ${import ./launcher/names.nix { inherit lib pkgs; }} names.txt
      cp ${./launcher/Units.js} Units.js
      cp ${./launcher/Zones.js} Zones.js
      cp ${./launcher/Colours.js} Colours.js
      cp ${./launcher/Asking.js} Asking.js
      cp ${./launcher/Machine.js} Machine.js
      cp ${./launcher/Heat.js} Heat.js
      cp ${./launcher/Power.js} Power.js
      cp ${./launcher/Standing.js} Standing.js
      cp ${./launcher/Traffic.js} Traffic.js
      cp ${./launcher/Interfaces.js} Interfaces.js
      cp ${./launcher/Links.js} Links.js
      cp ${./launcher/Sounds.js} Sounds.js
      cp ${./launcher/Cameras.js} Cameras.js
      cp ${./launcher/Keyboards.js} Keyboards.js
      cp ${../../tests/colour-cases.js} colour-cases.js
      cp ${import ./launcher/zones.nix { inherit lib pkgs; }} zones.jsonl
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
      cp ${../../tests/maths-speed.qml} maths-speed.qml
      cp ${./launcher/Numbers.js} Numbers.js
      cp ${./launcher/Units.js} Units.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml:${mathsModules} timeout 300 ${qt}/bin/qml maths-speed.qml
      touch $out
    '';

  # The region picker's keys start, move and size its box, and letters name
  # the grid's cells, as tests/picking.qml says, in Qt's own engine.
  the-picker-picks-what-the-keys-say =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "the-picker-picks-what-the-keys-say" { } ''
      cp ${../../tests/picking.qml} picking.qml
      cp ${./Picking.js} Picking.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml ${qt}/bin/qml picking.qml
      touch $out
    '';

  # The askpass prompt says who's asking for what, for each program's words,
  # as tests/askpass.qml says, in Qt's own engine.
  the-askpass-says-who-asks-for-what =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "the-askpass-says-who-asks-for-what" { } ''
      cp ${../../tests/askpass.qml} askpass.qml
      cp ${./Askpass.js} Askpass.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml ${qt}/bin/qml askpass.qml
      touch $out
    '';

  # The askpass asks a shell that restarted mid-prompt again, and refuses
  # once none has answered for a while, rather than holding ssh-tpm-agent
  # and every key behind it forever, as tests/askpass-waits.sh says. The
  # desktop test restarts a real shell, in minutes; this says it in one.
  the-askpass-outlives-its-shell =
    let
      askpass = (account graphical).home.sessionVariables.SSH_ASKPASS;
    in
    pkgs.runCommandLocal "the-askpass-outlives-its-shell" { } ''
      ${lib.getExe' pkgs.coreutils "timeout"} 300 ${lib.getExe pkgs.bash} \
        ${../../tests/askpass-waits.sh} ${askpass} ||
        { echo "the askpass is still waiting, or got something wrong"; exit 1; }
      touch $out
    '';

  # A D-Bus call from the account's shells, as run0 asks for polkit's prompt
  # in, waits past systemd's own 25 seconds for an answer, as
  # tests/prompt-outlasts-the-bus.py says, on a session bus of its own.
  a-prompt-waits-for-a-person =
    let
      timeout = (account graphical).home.sessionVariables.SYSTEMD_BUS_TIMEOUT or "";
      python = pkgs.python3.withPackages (python: [ python.jeepney ]);
    in
    pkgs.runCommandLocal "a-prompt-waits-for-a-person" { } ''
      export HOME=$TMPDIR
      ${lib.getExe' pkgs.dbus "dbus-run-session"} \
        --dbus-daemon=${lib.getExe' pkgs.dbus "dbus-daemon"} \
        --config-file=${pkgs.dbus}/share/dbus-1/session.conf -- \
        ${lib.getExe python} ${../../tests/prompt-outlasts-the-bus.py} \
        ${lib.escapeShellArg timeout} ${lib.getExe' pkgs.systemd "busctl"}
      touch $out
    '';

  # Do not disturb holds notices while the screen is shared and lets them go
  # after, as tests/quiet.qml drives it through Quickshell itself, with
  # niri's casts, the notices, the recorder and the shell's options stood in
  # for.
  do-not-disturb-holds-while-sharing =
    let
      options = singleton "Options.qml" "readonly property var quiet: ({ whileSharing: true })";
    in
    pkgs.runCommandLocal "do-not-disturb-holds-while-sharing" { } ''
      mkdir runtime && chmod 700 runtime
      cp ${./Quiet.qml} Quiet.qml
      cp ${../../tests/quiet.qml} shell.qml
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

  # The disk piece reads the guards' status files as tests/disk.qml writes
  # them, through Quickshell itself, with niri stood in for.
  the-disk-piece-reads-the-guards =
    let
      niri = pkgs.writeText "Niri.qml" ''
        pragma Singleton
        import Quickshell
        import QtQuick
        Singleton {
            property var spawned: []
            function spawn(command) { spawned = command; }
        }
      '';
    in
    pkgs.runCommandLocal "the-disk-piece-reads-the-guards" { } ''
      mkdir runtime launcher && chmod 700 runtime
      cp ${./pieces/Piece.qml} Piece.qml
      cp ${./pieces/Disk.qml} Disk.qml
      cp ${./Words.qml} Words.qml
      cp ${./launcher/words.json} launcher/words.json
      cp ${./pieces/StatusFile.qml} StatusFile.qml
      cp ${../../tests/disk.qml} shell.qml
      cp ${niri} Niri.qml
      cp ${(account graphical).xdg.configFile."quickshell/Theme.qml".source} Theme.qml
      export HOME=$TMPDIR XDG_RUNTIME_DIR=$TMPDIR/runtime LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      timeout 60 ${qs} -p shell.qml
      touch $out
    '';

  # The keyboard's backlight is heard when its firmware changes it, over a
  # folder laid out as the kernel's lights are, through Quickshell itself.
  the-keyboard-light-is-heard = pkgs.runCommandLocal "the-keyboard-light-is-heard" { } ''
    mkdir -p runtime/leds/test::kbd_backlight runtime/leds/input0::capslock && chmod 700 runtime
    echo 3 > runtime/leds/test::kbd_backlight/max_brightness
    touch runtime/leds/test::kbd_backlight/brightness_hw_changed
    cp ${./Lights.qml} Lights.qml
    cp ${../../tests/lights.qml} shell.qml
    export HOME=$TMPDIR XDG_RUNTIME_DIR=$TMPDIR/runtime LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen
    export FONTCONFIG_FILE=${noFonts}
    ${shellFolderLint}
    timeout 60 ${qs} -p shell.qml
    touch $out
  '';

  # The AI spend piece and the launcher's collection of it read what
  # spend-watch writes, from a day of a session's turns on two models, and the states
  # tests/spend.qml writes by hand, through Quickshell itself, with niri, do
  # not disturb and the shortcuts stood in for.
  the-shell-reads-the-spend-watch =
    let
      watch = pkgs.callPackage ../spend-watch/package.nix { };
      niri = singleton "Niri.qml" ''
        property var spawned: []
        function spawn(command) { spawned = command; }
      '';
      shortcuts = singleton "Shortcuts.qml" ''
        property var actions: [{ opens: "spend", chord: [{ text: "Super" }, { text: "U" }] }]
      '';
    in
    pkgs.runCommandLocal "the-shell-reads-the-spend-watch" { } ''
      mkdir -p runtime/written runtime/spend-watch sessions/thing state launcher && chmod 700 runtime
      ${lib.getExe pkgs.bash} ${../../tests/spend-session.sh} sessions/thing/session.jsonl
      cat > config.json <<EOF
      {"sessions": "$PWD/sessions", "runs": "$PWD/runs.db", "registry": "$PWD/registry",
       "quests": "$PWD/quests", "state": "$PWD/state", "runtime": "$PWD/runtime/written",
       "zone": "UTC", "notices": {"enable": false}}
      EOF
      ${lib.getExe watch} --config config.json import
      ${lib.getExe watch} --config config.json show > runtime/written/status.json
      ${lib.getExe watch} --config config.json show collection > runtime/written/collection.json
      cp ${./pieces/Piece.qml} Piece.qml
      cp ${./pieces/Spend.qml} Spend.qml
      cp ${./pieces/StatusFile.qml} StatusFile.qml
      cp ${./Fuzzy.js} Fuzzy.js
      cp ${./launcher/Provider.qml} launcher/Provider.qml
      cp ${./launcher/Asking.js} launcher/Asking.js
      cp ${./launcher/Numbers.js} launcher/Numbers.js
      cp ${./launcher/Spend.qml} launcher/Spend.qml
      cp ${(account graphical).xdg.configFile."quickshell/Theme.qml".source} Theme.qml
      cp ${./Words.qml} Words.qml
      cp ${./launcher/words.json} launcher/words.json
      cp ${./SpendWatch.qml} SpendWatch.qml
      cp ${../../tests/spend.qml} shell.qml
      cp ${niri} Niri.qml
      cp ${singleton "Quiet.qml" "property bool sharing: false"} Quiet.qml
      cp ${shortcuts} Shortcuts.qml
      export HOME=$TMPDIR XDG_RUNTIME_DIR=$TMPDIR/runtime LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      timeout 60 ${qs} -p shell.qml
      touch $out
    '';

  # The launcher's quests find pi's quests in a store written here, as the
  # real pi-quests reads it (tests/quests-store.sh), and open pi on one or
  # go to its session (tests/quests.qml), with niri and pi's sessions stood
  # in.
  the-launcher-finds-the-quests =
    let
      niri = singleton "Niri.qml" ''
        property var spawned: []
        property var acted: null
        function spawn(command) { spawned = command; }
        function act(request) { acted = request; }
      '';
      sessions = singleton "Sessions.qml" ''
        readonly property string home: Quickshell.env("HOME")
        readonly property string quests: Quickshell.env("QUESTS")
        property var running: []
        function isOf(session) { return `window:''${session.window.id}`; }
        function goTo(session) { Niri.act({ FocusWindow: { id: session.window.id } }); }
        property var started: null
        function start(quest, folder) { started = [quest, folder]; }
        readonly property string machine: "alpha"
        property var places: []
        property string fits: ""
        readonly property bool choosesMachine: places.length > 1
        property int lookedAround: 0
        function lookAround() { lookedAround += 1; }
        property var startedOn: null
        function startOn(on, quest, folder) { startedOn = [on, quest, folder]; }
      '';
      quests = pkgs.callPackage ./pi-helper.nix {
        name = "pi-quests";
        source = ./quests.rs;
      };
    in
    pkgs.runCommandLocal "the-launcher-finds-the-quests" { } ''
      mkdir launcher
      ${lib.getExe pkgs.bash} ${../../tests/quests-store.sh} $PWD
      cp ${(account graphical).xdg.configFile."quickshell/Theme.qml".source} Theme.qml
      cp ${./Fuzzy.js} Fuzzy.js
      cp ${./launcher/Provider.qml} launcher/Provider.qml
      cp ${./launcher/Asking.js} launcher/Asking.js
      cp ${./launcher/Numbers.js} launcher/Numbers.js
      cp ${./launcher/Quests.qml} launcher/Quests.qml
      cp ${./Words.qml} Words.qml
      cp ${./launcher/words.json} launcher/words.json
      cp ${../../tests/quests.qml} shell.qml
      cp ${niri} Niri.qml
      cp ${sessions} Sessions.qml
      export HOME=$TMPDIR XDG_RUNTIME_DIR=$TMPDIR/runtime LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen
      # At home here, so a folder here reads from ~.
      export HOME=$PWD QUESTS=$PWD/store PATH=${quests}/bin:$PATH
      mkdir -m 700 $XDG_RUNTIME_DIR
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      timeout 60 ${qs} -p shell.qml
      touch $out
    '';

  # The orchestrators' talks (tests/talks.qml), over 120 inboxes with 120
  # sessions running, two inboxes holding talks: only those are read, and
  # focus moving makes no reader again, so the shell's threads, and with
  # them its file watchers, stay bounded.
  the-launcher-reads-talks-boundedly =
    let
      niri = singleton "Niri.qml" ''
        property int revision: 0
        function spawn(command) {}
        function act(request) {}
      '';
      sessions = singleton "Sessions.qml" ''
        readonly property string home: Quickshell.env("HOME")
        readonly property string state: Quickshell.env("PWD") + "/state"
        readonly property string inboxes: state + "/steer"
        property var running: Array.from({ length: 120 },
          (_, at) => ({ address: "A" + String(at).padStart(3, "0") }))
        property var present: ({})
        function resume(id, quest, folder) {}
        function start(quest, folder) {}
      '';
      delivered = words: ''{"stage":"delivered","at":"2026-10-29T10:00:00Z","text":"${words}"}'';
    in
    pkgs.runCommandLocal "the-launcher-reads-talks-boundedly" { } ''
      mkdir -p launcher state/quest-workflow
      for at in $(seq -w 0 119); do mkdir -p state/steer/A$at; done
      mkdir state/steer/A003/replies state/steer/A117/replies
      echo '${delivered "wrap up the release"}' > state/steer/A003/replies/t-1.jsonl
      echo '${delivered "check the lights"}' > state/steer/A117/replies/t-2.jsonl
      echo '{"orchestrators":[{"address":"A003","quest":"QEST-A","title":"A"}]}' \
        > state/quest-workflow/orchestrators.json
      echo '{"quests":[]}' > state/quest-workflow/quests.json
      cp ${(account graphical).xdg.configFile."quickshell/Theme.qml".source} Theme.qml
      cp ${./Fuzzy.js} Fuzzy.js
      cp ${./launcher/Provider.qml} launcher/Provider.qml
      cp ${./launcher/Asking.js} launcher/Asking.js
      cp ${./launcher/Numbers.js} launcher/Numbers.js
      cp ${./launcher/Orchestrators.qml} launcher/Orchestrators.qml
      cp ${./Words.qml} Words.qml
      cp ${./launcher/words.json} launcher/words.json
      cp ${../../tests/talks.qml} shell.qml
      cp ${niri} Niri.qml
      cp ${sessions} Sessions.qml
      export HOME=$TMPDIR XDG_RUNTIME_DIR=$TMPDIR/runtime LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen
      mkdir -m 700 $XDG_RUNTIME_DIR
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      timeout 60 ${qs} -p shell.qml
      touch $out
    '';

  # Now's own rows, last: the time, the date, the location, the weather
  # read as the launcher opens and the next meeting, whose Return is the
  # meeting's own (tests/now-moment.qml), with the margin's pieces, the
  # agents' waiting list, the weather and the calendar stood in. The
  # location is Location.qml's own, reading the file whereabouts would
  # write, written here, and a clock whose zone a stand-in timedatectl says
  # is another than the place's.
  the-launcher-shows-the-moment-in-now =
    let
      facts = singleton "Facts.qml" ''
        property var all: []
      '';
      waiting = singleton "WaitingOnYou.qml" ''
        property var items: []
        property var now: []
        property var later: []
        property var foremost: null
        function titled(each) { return each.title; }
        function identity(each) { return each.title; }
        function questOf(each) { return ""; }
        function windowOf(each) { return null; }
        function settle(each, how) {}
        function go(each) {}
      '';
      # Where an item is done names its session, and Waiting on You lists
      # the sessions wanting Joel, none of which it needs here.
      sessions = singleton "Sessions.qml" ''
        property var running: []
        property var wanting: []
        function wanted(wants) { return "asks you"; }
      '';
      timedatectl = pkgs.writeShellScriptBin "timedatectl" "echo America/Halifax";
    in
    pkgs.runCommandLocal "the-launcher-shows-the-moment-in-now" { } ''
      mkdir launcher
      cp ${(account graphical).xdg.configFile."quickshell/Theme.qml".source} Theme.qml
      cp ${./Fuzzy.js} Fuzzy.js
      cp ${./Marks.js} Marks.js
      cp ${./launcher/Provider.qml} launcher/Provider.qml
      cp ${./launcher/Asking.js} launcher/Asking.js
      cp ${./launcher/Numbers.js} launcher/Numbers.js
      cp ${./launcher/WhatsOn.qml} launcher/WhatsOn.qml
      cp ${./launcher/Now.js} launcher/Now.js
      cp ${./Words.qml} Words.qml
      cp ${./launcher/words.json} launcher/words.json
      cp ${../../tests/now-moment.qml} shell.qml
      cp ${facts} Facts.qml
      cp ${waiting} WaitingOnYou.qml
      cp ${sessions} Sessions.qml
      cp ${./Location.qml} Location.qml
      export HOME=$TMPDIR XDG_RUNTIME_DIR=$TMPDIR/runtime LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen
      export PATH=${timedatectl}/bin:$PATH
      mkdir -m 700 $XDG_RUNTIME_DIR
      echo '{"v":2,"latitude":43.6748,"longitude":-79.3794,"accuracy":26000,'\
      '"when":1790000000,"place":"Lighthouse, Ontario","zone":"America/Toronto"}' \
        > $XDG_RUNTIME_DIR/whereabouts.json
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      timeout 60 ${qs} -p shell.qml
      touch $out
    '';

  # The launcher's weather answers now, its parts and the rain by the hour
  # from one forecast, and the rain's own words answer the next rain
  # (tests/weather.qml), with what Open-Meteo would say kept beforehand, so
  # nothing leaves the sandbox; and here is where the laptop is, as each
  # case gives Location's here, ahead of the place set and the time zone.
  the-launcher-answers-the-rain = pkgs.runCommandLocal "the-launcher-answers-the-rain" { } ''
    mkdir launcher
    cp ${./Fuzzy.js} Fuzzy.js
    cp ${./launcher/Provider.qml} launcher/Provider.qml
    cp ${./launcher/Asking.js} launcher/Asking.js
    cp ${./launcher/Numbers.js} launcher/Numbers.js
    cp ${./launcher/Weather.qml} launcher/Weather.qml
    cp ${(account graphical).xdg.configFile."quickshell/Theme.qml".source} Theme.qml
    cp ${./Words.qml} Words.qml
    cp ${./launcher/words.json} launcher/words.json
    cp ${../../tests/weather.qml} shell.qml
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

  # The launcher's Ask row says why when pi-ask can't start, and gives each
  # question a pi of its own (tests/ask.qml), over a pi-ask that ends with
  # an error and the desktop tests' stand-in, which speaks pi's RPC as the
  # work machine's pi does.
  the-launcher-asks-a-fresh-pi =
    let
      broken = pkgs.writeShellScript "pi-ask-broken" ''
        echo "Error: Failed to get a token." >&2
        exit 1
      '';
      standIn = pkgs.writers.writePython3 "stand-in-ask" {
        doCheck = false;
      } (builtins.readFile ../../tests/desktop/stand-in-ask.py);
      niri = singleton "Niri.qml" "function spawn(command) {}";
      sessions = singleton "Sessions.qml" ''
        readonly property string home: Quickshell.env("HOME")
        function tell(words, folder, program) {}
        function talk(address, words) { return true; }
      '';
    in
    pkgs.runCommandLocal "the-launcher-asks-a-fresh-pi" { } ''
      mkdir -p launcher runtime && chmod 700 runtime
      cp ${(account graphical).xdg.configFile."quickshell/Theme.qml".source} Theme.qml
      cp ${./Fuzzy.js} Fuzzy.js
      cp ${./launcher/Provider.qml} launcher/Provider.qml
      cp ${./launcher/Asking.js} launcher/Asking.js
      cp ${./launcher/Numbers.js} launcher/Numbers.js
      cp ${./launcher/Ask.qml} launcher/Ask.qml
      cp ${./Words.qml} Words.qml
      cp ${./launcher/words.json} launcher/words.json
      cp ${niri} Niri.qml
      cp ${sessions} Sessions.qml
      cp ${../../tests/ask.qml} shell.qml
      export HOME=$TMPDIR XDG_RUNTIME_DIR=$PWD/runtime LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts} ASK_BROKEN=${broken} ASK_STAND_IN=${standIn}
      ${shellFolderLint}
      timeout 60 ${qs} -p shell.qml
      touch $out
    '';

  # The launcher's Lag says what's slowing the desktop in plain words and
  # what to do about it (tests/waits.qml), over lag-watch's status file as
  # it writes one, the test writing it in turn.
  the-launcher-says-why-the-desktop-lags =
    pkgs.runCommandLocal "the-launcher-says-why-the-desktop-lags" { }
      ''
        mkdir -p launcher runtime/lag-watch && chmod 700 runtime
        cp ${(account graphical).xdg.configFile."quickshell/Theme.qml".source} Theme.qml
        cp ${./Fuzzy.js} Fuzzy.js
        cp ${./launcher/Provider.qml} launcher/Provider.qml
        cp ${./launcher/Asking.js} launcher/Asking.js
        cp ${./launcher/Numbers.js} launcher/Numbers.js
        cp ${./launcher/Waits.qml} launcher/Waits.qml
        cp ${./Words.qml} Words.qml
        cp ${./launcher/words.json} launcher/words.json
        cp ${../../tests/waits.qml} shell.qml
        export HOME=$TMPDIR XDG_RUNTIME_DIR=$PWD/runtime LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen
        export FONTCONFIG_FILE=${noFonts}
        ${shellFolderLint}
        timeout 60 ${qs} -p shell.qml
        touch $out
      '';

  # The launcher's answers that build their parts' ids as they answer, the
  # calculator's bases, the time's, today's, the waits', a hardware
  # address's, a network's and a character's, name every part as the
  # vocabulary does (tests/part-names.qml), with tzdata's zones and
  # Unicode's names kept and lag-watch's last ten seconds written here.
  the-launcher-names-the-parts-it-builds =
    pkgs.runCommandLocal "the-launcher-names-the-parts-it-builds" { }
      ''
        mkdir -p launcher runtime/lag-watch && chmod 700 runtime
        echo '{"trouble":[],"niri":{"p50_ms":4,"p99_ms":21},"band":"calm","ended":0}' \
          > runtime/lag-watch/status.json
        cp ${./Fuzzy.js} Fuzzy.js
        cp ${./launcher/Provider.qml} launcher/Provider.qml
        cp ${./launcher/Asking.js} launcher/Asking.js
        cp ${./launcher/Numbers.js} launcher/Numbers.js
        cp ${./launcher/Zones.js} launcher/Zones.js
        cp ${./launcher/Addresses.js} launcher/Addresses.js
        cp ${./launcher/Calculator.qml} launcher/Calculator.qml
        cp ${./launcher/Clocks.qml} launcher/Clocks.qml
        cp ${./launcher/Today.qml} launcher/Today.qml
        cp ${./launcher/Waits.qml} launcher/Waits.qml
        cp ${./launcher/Vendors.qml} launcher/Vendors.qml
        cp ${./launcher/Interfaces.js} launcher/Interfaces.js
        cp ${./launcher/Subnets.js} launcher/Subnets.js
        cp ${./launcher/Networks.qml} launcher/Networks.qml
        cp ${./launcher/Characters.js} launcher/Characters.js
        cp ${./launcher/Characters.qml} launcher/Characters.qml
        cp ${import ./launcher/names.nix { inherit lib pkgs; }} names.txt
        cp ${(account graphical).xdg.configFile."quickshell/Theme.qml".source} Theme.qml
        cp ${./Words.qml} Words.qml
        cp ${./launcher/words.json} launcher/words.json
        cp ${import ./launcher/zones.nix { inherit lib pkgs; }} zones.jsonl
        cp ${singleton "Location.qml" ''
          property string clockZone: ""
          property bool clockMoved: false
          property var moved: null
          property string movedSaid: ""
          function city(zone) { return zone; }
          function putBack() {}
        ''} Location.qml
        cp ${../../tests/part-names.qml} shell.qml
        export HOME=$TMPDIR XDG_RUNTIME_DIR=$TMPDIR/runtime LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen
        export TZ=America/Toronto TZDIR=${pkgs.tzdata}/share/zoneinfo
        export FONTCONFIG_FILE=${noFonts}
        ${shellFolderLint}
        timeout 60 ${qs} -p shell.qml
        touch $out
      '';

  # The launcher's Machine answers the processors and the memory from the
  # sandbox's own /proc without making their parts or listing the
  # processes, and lists them only once the parts are looked at
  # (tests/machine.qml), the list stood in for by one that says each time
  # it's run, and the queue and niri stood in for.
  the-machine-lists-processes-for-its-parts =
    let
      listing = pkgs.writeShellScript "process-list" ''
        echo listed >> "$XDG_RUNTIME_DIR/listed"
        echo 4096
        exec ${lib.getExe' pkgs.coreutils "cat"} /proc/stat
      '';
    in
    pkgs.runCommandLocal "the-machine-lists-processes-for-its-parts" { } ''
      mkdir -p launcher runtime && chmod 700 runtime
      cp ${./Fuzzy.js} Fuzzy.js
      cp ${./launcher/Provider.qml} launcher/Provider.qml
      cp ${./launcher/Asking.js} launcher/Asking.js
      cp ${./launcher/Numbers.js} launcher/Numbers.js
      cp ${./launcher/Heat.js} launcher/Heat.js
      cp ${./launcher/Interfaces.js} launcher/Interfaces.js
      cp ${./launcher/Machine.js} launcher/Machine.js
      cp ${./launcher/Machine.qml} launcher/Machine.qml
      cp ${./launcher/Sensors.qml} launcher/Sensors.qml
      cp ${(account graphical).xdg.configFile."quickshell/Theme.qml".source} Theme.qml
      cp ${./Words.qml} Words.qml
      cp ${./launcher/words.json} launcher/words.json
      cp ${singleton "Jobs.qml" "property var said: null"} Jobs.qml
      cp ${singleton "Niri.qml" "function act(action) {}"} Niri.qml
      cp ${../../tests/machine.qml} shell.qml
      export HOME=$TMPDIR XDG_RUNTIME_DIR=$TMPDIR/runtime LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen
      export PROCESSES=${listing}
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      timeout 60 ${qs} -p shell.qml
      touch $out
    '';

  # The launcher's Sound leaves out a jack with nothing plugged in and a
  # display's output with no display, and never listens to a headset's
  # microphone while the headset plays music (tests/sound.qml), deciding
  # over stand-in devices, with niri and the music stood in for.
  sound-hears-only-what-can-be-heard =
    let
      niri = singleton "Niri.qml" ''
        property var workspaces: ({ "1": { id: 1, output: "eDP-1" } })
      '';
      music = singleton "Music.qml" ''
        property bool playing: false
        property string title: ""
        property string artist: ""
      '';
    in
    pkgs.runCommandLocal "sound-hears-only-what-can-be-heard" { } ''
      mkdir -p launcher runtime && chmod 700 runtime
      cp ${./Fuzzy.js} Fuzzy.js
      cp ${./launcher/Provider.qml} launcher/Provider.qml
      cp ${./launcher/Asking.js} launcher/Asking.js
      cp ${./launcher/Numbers.js} launcher/Numbers.js
      cp ${./launcher/Sounds.js} launcher/Sounds.js
      cp ${./launcher/Sound.qml} launcher/Sound.qml
      cp ${(account graphical).xdg.configFile."quickshell/Theme.qml".source} Theme.qml
      cp ${./Words.qml} Words.qml
      cp ${./launcher/words.json} launcher/words.json
      cp ${niri} Niri.qml
      cp ${music} Music.qml
      cp ${../../tests/sound.qml} shell.qml
      export HOME=$TMPDIR XDG_RUNTIME_DIR=$TMPDIR/runtime LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      timeout 60 ${qs} -p shell.qml
      touch $out
    '';

  # The launcher tells how the desk light is lit and sets it, and the
  # brightness bar hears its changes but not a camera lighting it
  # (tests/desk-light.qml), over a stand-in litra that keeps a light's
  # state and says each change on its watch, as the real one does.
  the-launcher-finds-the-desk-light =
    let
      niri = pkgs.writeText "Niri.qml" ''
        pragma Singleton
        import Quickshell
        import QtQuick
        Singleton { function spawn(command) { Quickshell.execDetached(command); } }
      '';
      litra = pkgs.writeShellApplication {
        name = "litra";
        runtimeInputs = [ pkgs.coreutils ];
        text = ''
          state=$XDG_RUNTIME_DIR/litra-state
          feed=$XDG_RUNTIME_DIR/litra-feed
          [ -e "$state" ] || echo "true 40 4000" > "$state"
          read -r on percent kelvin < "$state"
          case "$1" in
            watch)
              printf '[{"node":"/dev/hidraw1","on":%s,"percent":%s,"kelvin":%s}]\n' \
                "$on" "$percent" "$kelvin" >> "$feed"
              exec tail -n +1 -f "$feed"
              ;;
            on) on=true ;;
            off) on=false ;;
            brightness) percent=$2 ;;
            temperature) kelvin=$2 ;;
            # Not litra's: the light's own buttons, or a camera, and the
            # light unplugged.
            press) on=$2 percent=$3 kelvin=$4 ;;
            unplug)
              echo "[]" >> "$feed"
              exit 0
              ;;
            *) exit 2 ;;
          esac
          echo "$on $percent $kelvin" > "$state"
          printf '[{"node":"/dev/hidraw1","on":%s,"percent":%s,"kelvin":%s}]\n' \
            "$on" "$percent" "$kelvin" >> "$feed"
        '';
      };
    in
    pkgs.runCommandLocal "the-launcher-finds-the-desk-light" { } ''
      mkdir launcher
      cp ${(account graphical).xdg.configFile."quickshell/Theme.qml".source} Theme.qml
      cp ${./Fuzzy.js} Fuzzy.js
      cp ${./DeskLight.qml} DeskLight.qml
      cp ${./launcher/Provider.qml} launcher/Provider.qml
      cp ${./launcher/Asking.js} launcher/Asking.js
      cp ${./launcher/Numbers.js} launcher/Numbers.js
      cp ${./launcher/Light.qml} launcher/Light.qml
      cp ${./launcher/Power.js} launcher/Power.js
      cp ${./Words.qml} Words.qml
      cp ${./launcher/words.json} launcher/words.json
      cp ${../../tests/desk-light.qml} shell.qml
      cp ${niri} Niri.qml
      export HOME=$TMPDIR XDG_RUNTIME_DIR=$TMPDIR/runtime LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen
      export PATH=${litra}/bin:$PATH
      mkdir -m 700 $XDG_RUNTIME_DIR
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      timeout 60 ${qs} -p shell.qml
      touch $out
    '';

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
        function workspace(id) { return workspaces[id ?? "1"] ?? null; }
        function act(action) { acted = acted.concat([action]); }
        function spawn(command) { spawned = command; }
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
        property var waiting: []
        property bool gathered: false
        property var marked: []
        function mark(id, read) { marked = marked.concat([id]); }
        function seen() {}
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
      mkdir -p launcher share/applications
      cp ${tester} share/applications/tester.desktop
      cp ${quiet} share/applications/quiet.desktop
      cp ${radio} share/applications/radio.desktop
      cp ${browser} share/applications/browser.desktop
      cp ${pair} share/applications/pair.desktop
      cp ${ghost} share/applications/ghost.desktop
      cp ${viewer} share/applications/org.example.viewer.desktop
      cp ${(account graphical).xdg.configFile."quickshell/Theme.qml".source} Theme.qml
      cp ${./Fuzzy.js} Fuzzy.js
      cp ${./launcher/Provider.qml} launcher/Provider.qml
      cp ${./launcher/Asking.js} launcher/Asking.js
      cp ${./launcher/Numbers.js} launcher/Numbers.js
      cp ${./launcher/Applications.qml} launcher/Applications.qml
      cp ${./launcher/Recents.qml} launcher/Recents.qml
      cp ${./launcher/PastNotices.qml} launcher/PastNotices.qml
      cp ${./launcher/Notice.js} launcher/Notice.js
      cp ${./Words.qml} Words.qml
      cp ${./Files.qml} Files.qml
      cp ${./launcher/words.json} launcher/words.json
      cp ${../../tests/program-controls.qml} shell.qml
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
      # its name and by its id, one file twice, one of no program here, and
      # one the radio was given, which can't open a file itself, and one the
      # viewer opened, named by its command as recent-files names it.
      printf 'Tester\t%s\nnobody-here\t%s\ntester\t%s\nTester\t%s\nradio\t%s\nviewer\t%s\n' \
        "$HOME/notes.md" "$HOME/lost.md" "$HOME/plan.md" "$HOME/notes.md" "$HOME/song.ogg" \
        "$HOME/paper.pdf" > $HOME/shared
      export SHARED_LIST=${pkgs.writeShellScript "shared-list" ''cat "$HOME/shared"''}
      # The browser's sites, as chrome-recent-sites prints them, each
      # reading saying which it is.
      export SITES_LIST=${pkgs.writeShellScript "sites-list" ''
        echo . >> "$HOME/sites-read"
        n=$(wc -l < "$HOME/sites-read")
        printf 'https://example.org/%s\tPage %s\nhttps://other.example.net/a\tOlder page\n' "$n" "$n"
      ''}
      ${shellFolderLint}
      timeout 60 ${qs} -p shell.qml
      touch $out
    '';

  # Every program an account lists in the launcher, each desktop entry it
  # shows, wears a mark of its own in launcher/Provider.qml's table, its
  # logo or the theme's mark for it, never the program mark the rest share,
  # and each theme mark it names is one the theme has.
  every-program-wears-its-own-mark =
    let
      home = account graphical;
      marks = pkgs.writeText "marks" (
        lib.concatMapStrings (name: "${name}\n") (lib.attrNames home.jitsusama.theme.marks)
      );
    in
    pkgs.runCommandLocal "every-program-wears-its-own-mark" { } ''
      sed -n '/programGlyphs: Object.assign({/,/settings.glyphs/p' ${./launcher/Provider.qml} > table
      grep -o '"[^"]*": "' table | cut -d '"' -f 2 > known
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
      touch $out
    '';

  # The launcher's runs find pi's fleets and review rounds in a harness's
  # state written here (tests/runs-state.sh), as the real pi-runs reads it,
  # and go to the session that started one or read a participant's answer
  # (tests/runs.qml), with niri, pi's sessions and the screen's sharing
  # stood in. pi-runs gives the answer whole, for the pager.
  the-launcher-finds-the-runs =
    let
      niri = singleton "Niri.qml" ''
        property var spawned: []
        property var acted: null
        function spawn(command) { spawned = command; }
        function act(request) { acted = request; }
      '';
      sessions = singleton "Sessions.qml" ''
        readonly property string state: Quickshell.env("PI_STATE")
        property var running: []
        function goTo(session) { Niri.act({ FocusWindow: { id: session.window.id } }); }
      '';
      runs = pkgs.callPackage ./pi-helper.nix {
        name = "pi-runs";
        source = ./runs.rs;
      };
    in
    pkgs.runCommandLocal "the-launcher-finds-the-runs" { } ''
      state=$PWD/state
      # One still at work, its process this sleep.
      sleep 300 &
      live=$!
      ${lib.getExe pkgs.bash} ${../../tests/runs-state.sh} $state $live

      printf 'All good.\n\n- One "finding".\n' > expected
      ${lib.getExe' runs "pi-runs"} answer $state/subagent-workflow/runs/fleet-live/reviewers/b/result.json \
        > answered
      cmp expected answered

      mkdir launcher
      cp ${(account graphical).xdg.configFile."quickshell/Theme.qml".source} Theme.qml
      cp ${./Fuzzy.js} Fuzzy.js
      cp ${./launcher/Provider.qml} launcher/Provider.qml
      cp ${./launcher/Asking.js} launcher/Asking.js
      cp ${./launcher/Numbers.js} launcher/Numbers.js
      cp ${./launcher/Runs.qml} launcher/Runs.qml
      cp ${./Words.qml} Words.qml
      cp ${./launcher/words.json} launcher/words.json
      cp ${../../tests/runs.qml} shell.qml
      cp ${niri} Niri.qml
      cp ${sessions} Sessions.qml
      cp ${singleton "Quiet.qml" "property bool sharing: false"} Quiet.qml
      export HOME=$TMPDIR XDG_RUNTIME_DIR=$TMPDIR/runtime LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen
      export PI_STATE=$state PATH=${runs}/bin:$PATH
      mkdir -m 700 $XDG_RUNTIME_DIR
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      timeout 60 ${qs} -p shell.qml
      kill $live
      touch $out
    '';

  # The launcher's queue, Agents' place for the machine's heavy work, says
  # what each job is from the queue pi's harness writes (tests/queue.qml),
  # with the queue, pi's sessions, niri, Quests and a slice's memory stood
  # in: names in words, times against the last run, what a waiting job
  # waits for, and where Return goes.
  the-launcher-finds-the-queue =
    let
      niri = singleton "Niri.qml" ''
        property var acted: null
        function act(request) { acted = request; }
      '';
      sessions = singleton "Sessions.qml" ''
        property var running: [{ presence: { session: "s1" }, window: { id: 7 }, address: "QUEUE1" }]
        function goTo(session) { Niri.act({ FocusWindow: { id: session.window.id } }); }
      '';
      jobs = singleton "Jobs.qml" ''
        property var said: null
        readonly property var jobs: said?.jobs ?? []
        readonly property var running: jobs.filter(job => job.state === "running")
        readonly property var waiting: jobs.filter(job => job.state === "waiting")
      '';
    in
    pkgs.runCommandLocal "the-launcher-finds-the-queue" { } ''
      mkdir -p launcher cgroups/work.scope
      # 5.8 GiB, held by the running job's slice.
      echo 6227702579 > cgroups/work.scope/memory.current
      cp ${(account graphical).xdg.configFile."quickshell/Theme.qml".source} Theme.qml
      cp ${./Fuzzy.js} Fuzzy.js
      cp ${./launcher/Provider.qml} launcher/Provider.qml
      cp ${./launcher/Asking.js} launcher/Asking.js
      cp ${./launcher/Numbers.js} launcher/Numbers.js
      cp ${./launcher/Queue.qml} launcher/Queue.qml
      cp ${./Words.qml} Words.qml
      cp ${./launcher/words.json} launcher/words.json
      cp ${../../tests/queue.qml} shell.qml
      cp ${niri} Niri.qml
      cp ${sessions} Sessions.qml
      cp ${jobs} Jobs.qml
      export HOME=$TMPDIR XDG_RUNTIME_DIR=$TMPDIR/runtime LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen
      export CGROUPS=$PWD/cgroups
      mkdir -m 700 $XDG_RUNTIME_DIR
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      timeout 60 ${qs} -p shell.qml
      touch $out
    '';

  # pi's sessions are known by their own records and the processes those
  # name, never by a window's title, and each one's window through the
  # kitty it runs in (tests/sessions.qml), over real processes
  # tests/sessions-state.sh starts, with niri and kitten stood in.
  a-session-is-known-by-its-own-record =
    let
      # Three windows of the same title, one of kitty 500's and two of 600's.
      niri = singleton "Niri.qml" ''
        property var windows: ({
          "10": { id: 10, pid: 500, title: "\u03c0 - Lighthouse" },
          "20": { id: 20, pid: 600, title: "\u03c0 - Lighthouse" },
          "21": { id: 21, pid: 600, title: "\u03c0 - Lighthouse" }
        })
        property int revision: 0
        property var current: null
        property var acted: null
        property var spawned: []
        function act(request) { acted = request; }
        function spawn(command) { spawned = command; }
      '';
      options = singleton "Options.qml" ''
        readonly property var sounds: ({ wants: "", player: "" })
      '';
      quiet = singleton "Quiet.qml" ''
        property bool on: false
      '';
      notices = singleton "Notices.qml" ''
        property var live: ({})
        function mark(id, read) {}
      '';
      waiting = singleton "WaitingOnYou.qml" ''
        readonly property var standing: ({})
      '';
      report = singleton "Report.qml" "function say(text) {}";
    in
    pkgs.runCommandLocal "a-session-is-known-by-its-own-record" { } ''
      export XDG_STATE_HOME=$PWD/state XDG_DATA_HOME=$PWD/data
      ${lib.getExe pkgs.bash} ${../../tests/sessions-state.sh} $XDG_STATE_HOME
      export PATH=$XDG_STATE_HOME/bin:$PATH OTHER_PID=$(cat $XDG_STATE_HOME/other.pid)
      cp ${./Sessions.qml} Sessions.qml
      cp ${./Waiting.js} Waiting.js
      cp ${waiting} WaitingOnYou.qml
      cp ${report} Report.qml
      cp ${../../tests/sessions.qml} shell.qml
      cp ${niri} Niri.qml
      cp ${options} Options.qml
      cp ${quiet} Quiet.qml
      cp ${notices} Notices.qml
      export HOME=$TMPDIR XDG_RUNTIME_DIR=$TMPDIR/runtime LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen
      mkdir -m 700 $XDG_RUNTIME_DIR $XDG_RUNTIME_DIR/mycelium
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      timeout 60 ${qs} -p shell.qml
      touch $out
    '';

  # The launcher's People, with Tuple as its source, follows Tuple's
  # contacts through the real tuple-people, which reads a stand-in for
  # Tuple's follow here, and calls, joins or favours one (tests/people.qml),
  # with niri stood in. Stopping the
  # helper stops Tuple's follow, which writes only when someone's presence
  # changes and so would never find out by itself. A listener stands in for
  # the daemon's socket, and once it's stopped, leaving the socket's file
  # as Tuple's daemon does, Tuple must not be run again, since any tuple
  # command would start a daemon.
  the-launcher-finds-the-people =
    let
      niri = singleton "Niri.qml" ''
        property var spawned: []
        function spawn(command) { spawned = command; }
      '';
      helper = pkgs.callPackage ./pi-helper.nix {
        name = "tuple-people";
        source = ../tuple/people.rs;
      };
      contact = id: full: short: email: status: favorited: recent: {
        inherit
          id
          email
          status
          favorited
          recent
          ;
        full_name = full;
        short_name = short;
        kind = "user";
      };
      # Strangers between those Tuple knows, so their order is the search's.
      snapshot =
        ada:
        pkgs.writeText "contacts.json" (
          builtins.toJSON [
            (contact 101 "Eli Fenwick" null "eli@example.com" "unknown" false false)
            (contact 102 "Ada Brook" "Ada" "ada@example.com" ada true false)
            (contact 103 null null "gus@example.com" "unknown" false false)
            (contact 104 "Ben Carrow" null "ben@example.com" "busy" true false)
            (contact 105 "Cal Dunmore" null "cal@example.com" "offline" false true)
            (contact 106 "Zoë \"Zed\" Quill" null "zoe@example.com" "unknown" false false)
            (contact 107 "Dee Ellery" null "dee@example.com" "unavailable" true false)
            (contact 108 "" "Hal" null "unknown" false false)
          ]
          + "\n"
        );
      # Tuple's follow says everyone, then again once Ada goes into a call,
      # and goes on following. It waits as itself rather than as a sleep,
      # so it's found by its name until it ends. Anything else asked of it
      # is written down, since the shell asks Tuple through niri alone.
      tuple = pkgs.writeShellScriptBin "tuple" ''
        if [ "$*" != "--format json contacts follow" ]; then
          echo "$*" >> "$XDG_RUNTIME_DIR/asked"
          exit 0
        fi
        echo followed >> "$XDG_RUNTIME_DIR/follows"
        cat ${snapshot "online"}
        while [ ! -e "$XDG_RUNTIME_DIR/next" ]; do sleep 0.1; done
        cat ${snapshot "busy"}
        while :; do sleep 1; done
      '';
      # Waits up to 2 s for Tuple's stand-in to be following, given yes, or
      # to be gone, given no.
      followed = pkgs.writeShellScript "followed" ''
        for _ in $(seq 20); do
          found=no
          for cmdline in /proc/[0-9]*/cmdline; do
            case "$(tr '\0' ' ' < "$cmdline" 2>/dev/null)" in
              *${tuple}/bin/tuple*contacts*follow*) found=yes ;;
            esac
          done
          [ "$found" = "$1" ] && exit 0
          sleep 0.1
        done
        exit 1
      '';
      # Stops the daemon's stand-in, its socket's file left behind.
      stop = pkgs.writeShellScript "stop-tuple" ''
        kill "$(cat "$XDG_RUNTIME_DIR/daemon")"
      '';
    in
    pkgs.runCommandLocal "the-launcher-finds-the-people" { } ''
      mkdir -p launcher/people
      cp ${(account graphical).xdg.configFile."quickshell/Theme.qml".source} Theme.qml
      cp ${./Fuzzy.js} Fuzzy.js
      cp ${./launcher/Provider.qml} launcher/Provider.qml
      cp ${./launcher/Asking.js} launcher/Asking.js
      cp ${./launcher/Numbers.js} launcher/Numbers.js
      cp ${../people/People.qml} launcher/People.qml
      cp ${../people/Person.js} launcher/Person.js
      cp ${../people/PersonSource.qml} PersonSource.qml
      cp ${../tuple/TupleSource.qml} launcher/people/tuple.qml
      cp ${../tuple/TuplePeople.qml} TuplePeople.qml
      cp ${./Words.qml} Words.qml
      cp ${./launcher/words.json} launcher/words.json
      cp ${../../tests/people.qml} shell.qml
      cp ${niri} Niri.qml
      export HOME=$TMPDIR XDG_RUNTIME_DIR=$TMPDIR/runtime LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen
      export FOLLOWED=${followed} STOP_TUPLE=${stop} PATH=${helper}/bin:${tuple}/bin:$PATH
      mkdir -m 700 $XDG_RUNTIME_DIR
      ${lib.getExe pkgs.socat} UNIX-LISTEN:$XDG_RUNTIME_DIR/tuple-0,fork,unlink-close=0 /dev/null &
      echo $! > $XDG_RUNTIME_DIR/daemon
      while [ ! -S $XDG_RUNTIME_DIR/tuple-0 ]; do sleep 0.1; done
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      timeout 60 ${qs} -p shell.qml
      if [ -e $XDG_RUNTIME_DIR/asked ]; then
        echo "the shell asked Tuple itself:"; cat $XDG_RUNTIME_DIR/asked; exit 1
      fi
      if [ "$(wc -l < $XDG_RUNTIME_DIR/follows)" != 2 ]; then
        echo "Tuple was followed $(wc -l < $XDG_RUNTIME_DIR/follows) times, not twice: a stopped Tuple was run"
        exit 1
      fi
      touch $out
    '';

  # A FileView told not to print errors says nothing of a file that opens
  # and then refuses every read, as some sysfs files do, in the Quickshell
  # the machine runs (fileview-quiet.patch); /proc/self/mem is one, since
  # its first page is never mapped. One that may print does warn, so the
  # check would see the warning if the patch were gone.
  a-quiet-file-says-nothing =
    let
      shell = pkgs.writeText "shell.qml" ''
        import Quickshell
        import Quickshell.Io
        import QtQuick
        ShellRoot {
            FileView { id: quiet; path: "/proc/self/mem"; blockLoading: true; printErrors: false }
            FileView { id: loud; path: "/proc/self/mem"; blockLoading: true }
            Component.onCompleted: {
                quiet.text();
                loud.text();
                Qt.exit(0);
            }
        }
      '';
    in
    pkgs.runCommandLocal "a-quiet-file-says-nothing" { } ''
      mkdir runtime && chmod 700 runtime
      cp ${shell} shell.qml
      export HOME=$TMPDIR XDG_RUNTIME_DIR=$TMPDIR/runtime LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen
      ${shellFolderLint}
      timeout 60 ${qs} -p shell.qml > log 2>&1 || true
      cat log
      # Each warning names the line its FileView is on: the quiet one's
      # 5, the loud one's 6.
      if grep -q 'shell.qml\[5:.*read() failed' log; then
        echo "a FileView told not to print errors warned of a failed read"; exit 1
      fi
      grep -q 'shell.qml\[6:.*read() failed' log || { echo "the loud FileView never warned"; exit 1; }
      touch $out
    '';

  # A long IPC reply reaches its caller whole, in the Quickshell the machine
  # runs (ipc-replies-whole.patch): without the patch, one past the socket's
  # buffer was cut off, so the launcher's catalogue never reached pi-ask.
  an-ipc-reply-arrives-whole = pkgs.runCommandLocal "an-ipc-reply-arrives-whole" { } ''
    mkdir runtime && chmod 700 runtime
    cp ${../../tests/ipc-replies.qml} shell.qml
    export HOME=$TMPDIR XDG_RUNTIME_DIR=$TMPDIR/runtime LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen
    export FONTCONFIG_FILE=${noFonts} QS=${qs} PATH=${pkgs.coreutils}/bin:$PATH
    ${shellFolderLint}
    timeout 60 ${qs} -p shell.qml
    touch $out
  '';

  # A status file is heard again once its folder is made again, and when
  # its folder is made only after the shell starts, as tests/watched.qml
  # drives it, in the Quickshell the machine runs
  # (fileview-watch-nearest.patch): a guard's runtime folder goes each time
  # systemd restarts the guard.
  a-watched-file-comes-back = pkgs.runCommandLocal "a-watched-file-comes-back" { } ''
    mkdir runtime && chmod 700 runtime
    cp ${./pieces/StatusFile.qml} StatusFile.qml
    cp ${../../tests/watched.qml} shell.qml
    export HOME=$TMPDIR XDG_RUNTIME_DIR=$TMPDIR/runtime LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen
    export FONTCONFIG_FILE=${noFonts}
    ${shellFolderLint}
    timeout 60 ${qs} -p shell.qml
    touch $out
  '';

  # A row's graph is ten braille characters over a dim track of every dot,
  # a bar filling from the left, a spark from the right with the newest last
  # and a strip a reading for each member, in the warning colour at 95% or
  # when it says it warns; rows' graphs stand in one column, and a figure
  # wears no glyph, as tests/graphs.qml draws them through Quickshell in the
  # theme's own face and sizes, at a panel's width.
  palette-graphs-draw-in-braille =
    let
      machine = graphical.config;
    in
    pkgs.runCommandLocal "palette-graphs-draw-in-braille" { } ''
      cp ${(account graphical).xdg.configFile."quickshell/Theme.qml".source} Theme.qml
      cp ${./Choice.qml} Choice.qml
      cp ${./Graph.qml} Graph.qml
      cp ${./Chord.qml} Chord.qml
      cp ${../../tests/graphs.qml} shell.qml
      export HOME=$TMPDIR XDG_RUNTIME_DIR=$TMPDIR/runtime LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen
      mkdir -m 700 $XDG_RUNTIME_DIR
      export FONTCONFIG_FILE=${pkgs.makeFontsConf { fontDirectories = machine.fonts.packages; }}
      ${shellFolderLint}
      timeout 60 ${qs} -p shell.qml
      touch $out
    '';

  # A palette's row and its field keep their words apart and inside their
  # edges however long they run, ending what doesn't fit in an ellipsis, as
  # tests/rows.qml draws them through Quickshell in the theme's own face and
  # sizes, at a panel's width.
  palette-rows-keep-their-words-apart =
    let
      machine = graphical.config;
      lights = pkgs.writeText "Lights.qml" ''
        pragma Singleton
        import Quickshell
        import QtQuick
        Singleton { property bool caps: false }
      '';
    in
    pkgs.runCommandLocal "palette-rows-keep-their-words-apart" { } ''
      cp ${(account graphical).xdg.configFile."quickshell/Theme.qml".source} Theme.qml
      cp ${./Choice.qml} Choice.qml
      cp ${./Graph.qml} Graph.qml
      cp ${./Chord.qml} Chord.qml
      cp ${./Prompt.qml} Prompt.qml
      cp ${./KeyHints.qml} KeyHints.qml
      cp ${lights} Lights.qml
      cp ${../../tests/rows.qml} shell.qml
      export HOME=$TMPDIR XDG_RUNTIME_DIR=$TMPDIR/runtime LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen
      mkdir -m 700 $XDG_RUNTIME_DIR
      export FONTCONFIG_FILE=${pkgs.makeFontsConf { fontDirectories = machine.fonts.packages; }}
      ${shellFolderLint}
      timeout 60 ${qs} -p shell.qml
      touch $out
    '';

  # The launcher's Keyboard collection finds the ErgoDox's keys and actions
  # and shows what it finds on the layout, and the board's parts among the
  # keyboards plugged in hold its Layer and Lights, as tests/keyboard.qml
  # drives them through Quickshell itself, over the firmware's own
  # description of the board, locked on Numb, with niri, the shortcuts and
  # the shell's lights stood in for.
  the-launcher-finds-the-keyboards-keys =
    let
      firmware = import ../ergodox/firmware.nix pkgs {
        firmware = self.inputs.qmk;
        inherit (self.inputs) lufa printf;
      };
      niri = singleton "Niri.qml" ''
        property var spawned: []
        function spawn(command) { spawned = command; }
      '';
      # No caps lock and no laptop backlight, as on a desk with the board
      # alone.
      lights = singleton "Lights.qml" ''
        property bool caps: false
        property string leds: ""
        property string backlight: ""
        signal turned
      '';
    in
    pkgs.runCommandLocal "the-launcher-finds-the-keyboards-keys" { } ''
      mkdir -p runtime config/ergodox launcher && chmod 700 runtime
      echo "Numb locked" > runtime/keyboard-layer
      cp ${firmware}/shell.json config/ergodox/layout.json
      cp ${../../tests/keyboard.qml} shell.qml
      cp ${./KeyboardLayout.qml} KeyboardLayout.qml
      cp ${./KeyboardLayer.qml} KeyboardLayer.qml
      cp ${./Fuzzy.js} Fuzzy.js
      cp ${./launcher/Provider.qml} launcher/Provider.qml
      cp ${./launcher/Asking.js} launcher/Asking.js
      cp ${./launcher/Numbers.js} launcher/Numbers.js
      cp ${./launcher/Keyboard.qml} launcher/Keyboard.qml
      cp ${./launcher/Keyboards.qml} launcher/Keyboards.qml
      cp ${./launcher/Keyboards.js} launcher/Keyboards.js
      cp ${(account graphical).xdg.configFile."quickshell/Theme.qml".source} Theme.qml
      cp ${./Words.qml} Words.qml
      cp ${./launcher/words.json} launcher/words.json
      cp ${niri} Niri.qml
      cp ${singleton "Shortcuts.qml" "property var actions: []"} Shortcuts.qml
      cp ${lights} Lights.qml
      export HOME=$TMPDIR XDG_RUNTIME_DIR=$TMPDIR/runtime XDG_CONFIG_HOME=$TMPDIR/config
      export LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      timeout 60 ${qs} -p shell.qml
      touch $out
    '';

  # BlueZ says a new device is connected once its link is up, before it's
  # paired, so the launcher's Bluetooth would stop following a pairing early
  # and leave the device untrusted. The VM has no radio, so this drives it
  # through Quickshell itself with stand-in devices
  # (tests/bluetooth-pairing.qml), with the facts desk-facts would write
  # for a connected mouse, and how its link and some headphones' are doing,
  # with their last reads and how each is held, and the adapter's traffic
  # and version, as bluetooth-links would write it: the headphones'
  # skipping channels 22 to 39, where a Wi-Fi network on its channel 6 sits.
  bluetooth-pairs-before-it-connects =
    pkgs.runCommandLocal "bluetooth-pairs-before-it-connects" { }
      ''
        mkdir -p runtime/desk launcher && chmod 700 runtime
        cat > runtime/desk/bluetooth.json <<'EOF'
        {
          "C3:9C:E3:2A:F3:6E": [
            {"name": "connection", "value": "LE, HID over GATT"},
            {"name": "buttons", "value": "5"}
          ]
        }
        EOF
        map=$(printf '1%.0s' {1..22}; printf '0%.0s' {1..18}; printf '1%.0s' {1..39})
        cat > runtime/bluetooth-links.json <<EOF
        {"at": 1761350400,
         "adapters": [{"name": "hci9", "address": "02:00:00:00:00:01", "up": true,
           "version": 14, "maker": 2, "received": 1200, "sent": 300,
           "traffic": [0, 750, 1500]}],
         "links": [
          {"address": "00:1B:66:AA:BB:CC", "rssi": 0, "quality": 230, "power": 4,
           "channels": 61, "map": "$map", "le": false,
           "held": {"central": true, "outgoing": false, "security": "secure"},
           "history": {"rssi": [-20, null, 0], "quality": [204, 230]}},
          {"address": "C3:9C:E3:2A:F3:6E", "rssi": -58, "quality": null, "power": 4,
           "channels": null, "map": null, "le": true,
           "held": {"central": false, "outgoing": true, "security": "none"}}
        ]}
        EOF
        cp ${./Fuzzy.js} Fuzzy.js
        cp ${./launcher/Provider.qml} launcher/Provider.qml
        cp ${./launcher/Asking.js} launcher/Asking.js
        cp ${./launcher/Numbers.js} launcher/Numbers.js
        cp ${./launcher/Accessories.qml} launcher/Accessories.qml
        cp ${./launcher/Links.js} launcher/Links.js
        cp ${(account graphical).xdg.configFile."quickshell/Theme.qml".source} Theme.qml
        cp ${./Words.qml} Words.qml
        cp ${./launcher/words.json} launcher/words.json
        cp ${../../tests/bluetooth-pairing.qml} shell.qml
        cp ${pkgs.writeText "Niri.qml" ''
          pragma Singleton
          import Quickshell
          import QtQuick
          Singleton { function spawn(command) {} }
        ''} Niri.qml
        export HOME=$TMPDIR XDG_RUNTIME_DIR=$TMPDIR/runtime LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen
        export FONTCONFIG_FILE=${noFonts}
        ${shellFolderLint}
        timeout 60 ${qs} -p shell.qml
        touch $out
      '';

  # The music is the player that matters: of those playing, the last to
  # start, else the last that played, which a play key resumes, and never
  # playerctld's echo of another, as tests/music.qml lists them, with where
  # in the track said as the card says it.
  the-music-follows-the-player =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "the-music-follows-the-player" { } ''
      cp ${../../tests/music.qml} music.qml
      cp ${./Music.js} Music.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml music.qml
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
      cp ${../../tests/notice-comes-forward.qml} notice-comes-forward.qml
      cp ${./Forward.js} Forward.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml notice-comes-forward.qml
      touch $out
    '';

  # A notice's card and its row in the launcher list the same acts in one
  # order, as tests/notice.qml holds Notice.js.
  a-notice-is-one-thing =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "a-notice-is-one-thing" { } ''
      cp ${../../tests/notice.qml} notice.qml
      cp ${./launcher/Notice.js} Notice.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml notice.qml
      touch $out
    '';

  # What comes true by itself is said once as a notice, and again only as
  # it tells something new, its earlier notice taken back first, as
  # tests/arrivals.qml holds Arrivals.qml to with the facts and the notices
  # stood in, and notify-send writing down what it's asked to say.
  an-arrival-is-said-once-each-time =
    let
      facts = singleton "Facts.qml" "property var all: []";
      notices = singleton "Notices.qml" ''
        property var live: ({})
        property var history: []
        property var marked: []
        function mark(id, read) { marked = marked.concat([id]); }
      '';
      options = singleton "Options.qml" "property bool reports: false";
      report = singleton "Report.qml" "function say(text) {}";
      notifySend = pkgs.writeShellScriptBin "notify-send" ''
        printf '%s\n' "$*" >> "$XDG_RUNTIME_DIR/said"
      '';
    in
    pkgs.runCommandLocal "an-arrival-is-said-once-each-time" { } ''
      cp ${./Arrivals.qml} Arrivals.qml
      cp ${facts} Facts.qml
      cp ${notices} Notices.qml
      cp ${options} Options.qml
      cp ${report} Report.qml
      cp ${../../tests/arrivals.qml} shell.qml
      export HOME=$TMPDIR XDG_RUNTIME_DIR=$TMPDIR/runtime LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts} PATH=${notifySend}/bin:$PATH
      mkdir -m 700 $XDG_RUNTIME_DIR
      # Made before the shell starts, so its watch has a file to follow.
      touch $XDG_RUNTIME_DIR/said
      ${shellFolderLint}
      timeout 60 ${qs} -p shell.qml
      touch $out
    '';

  # Now lists what's on by its groups, what wants Joel first, then by name
  # within each, as tests/now-groups.qml holds launcher/Now.js to it.
  now-lists-by-group =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "now-lists-by-group" { } ''
      cp ${../../tests/now-groups.qml} now-groups.qml
      cp ${./launcher/Now.js} Now.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml now-groups.qml
      touch $out
    '';

  # Every glyph in the margin stands the same height on the gap's middle
  # line, however tall its font draws it, and they line up from the corner
  # by how much each asks, in the colour that says so, as tests/marks.qml
  # holds them.
  the-margins-glyphs-stand-level =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "the-margins-glyphs-stand-level" { } ''
      cp ${../../tests/marks.qml} marks.qml
      cp ${./Marks.js} Marks.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml marks.qml
      touch $out
    '';

  # Waiting on You holds only what truly waits on Joel, now or later, each
  # once, and a session's want is his only while it's true, as
  # tests/waiting-on-you.qml holds Waiting.js to the agents' own files.
  waiting-on-you-is-true =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "waiting-on-you-is-true" { } ''
      cp ${../../tests/waiting-on-you.qml} waiting-on-you.qml
      cp ${./Waiting.js} Waiting.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml waiting-on-you.qml
      touch $out
    '';

  # Every mark the margin can show is the theme's drawing of it, standing the
  # grid's full height and centred across it as Qt draws it, so a row of
  # them is level and each has the same room either side (tests/drawings.qml).
  the-margins-marks-are-drawn-level =
    let
      qt = pkgs.qt6.qtdeclarative;
      home = account graphical;
      marks = lib.unique (
        lib.filter (mark: mark != null) (
          lib.mapAttrsToList (_: piece: piece.mark) home.jitsusama.quickshell.overview.pieces
        )
      );
      drawn = lib.mapAttrs (_: mark: mark.drawn) home.jitsusama.theme.marks;
      drawings = pkgs.writeText "Drawings.js" ''
        .pragma library
        var marks = ${builtins.toJSON marks};
        var drawn = ${builtins.toJSON drawn};
      '';
    in
    pkgs.runCommandLocal "the-margins-marks-are-drawn-level" { } ''
      cp ${../../tests/drawings.qml} drawings.qml
      cp ${drawings} Drawings.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml drawings.qml
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
      cp ${../../tests/stack.qml} stack.qml
      cp ${./Stack.js} Stack.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml stack.qml
      touch $out
    '';

  # A sender kept to a level reaches no further, however urgent it says it
  # is, and each level shows, counts and stays as tests/levels.qml holds.
  a-sender-keeps-to-its-level =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "a-sender-keeps-to-its-level" { } ''
      cp ${../../tests/levels.qml} levels.qml
      cp ${./Levels.js} Levels.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml levels.qml
      touch $out
    '';

  # Whether Joel's in a Tuple call comes from the daemon's log, which only a
  # real call writes, so tests/tuple.qml gives Tuple.js lines like its own:
  # what starts a call, what ends one, when it began, and a call's link.
  the-shell-follows-tuple-calls =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "the-shell-follows-tuple-calls" { } ''
      cp ${../../tests/tuple.qml} tuple.qml
      cp ${../tuple/Tuple.js} Tuple.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml tuple.qml
      touch $out
    '';

  # How the call Joel's in stands comes from Tuple's stream, as tuple-people
  # says it (tests/tuple-call.qml): the shell's Tuple reads it as a stand-in
  # helper says a call's lines, one at a time as the test writes them, and
  # tells who comes and goes through a stand-in notify-send that keeps what
  # it's asked. The helper's own reading of Tuple's stream is its unit
  # tests', run as it builds.
  the-shell-hears-tuples-call =
    let
      niri = pkgs.writeText "Niri.qml" ''
        pragma Singleton
        import Quickshell
        import QtQuick
        Singleton {
          property var windows: ({})
          property var casts: ({})
          function spawn(command) { }
          function act(request) { }
        }
      '';
      # Each run follows a feed of its own, the first call-feed and every
      # later one call-feed.next, until a line says END.
      people = pkgs.writeShellApplication {
        name = "tuple-people";
        runtimeInputs = [ pkgs.coreutils ];
        text = ''
          runs=$XDG_RUNTIME_DIR/runs
          ran=$(cat "$runs" 2>/dev/null || echo 0)
          echo $((ran + 1)) > "$runs"
          feed=$XDG_RUNTIME_DIR/call-feed
          [ "$ran" -eq 0 ] || feed=$feed.next
          touch "$feed"
          while read -r line; do
            [ "$line" = END ] && exit 0
            echo "$line"
          done < <(tail -n +1 -f "$feed")
        '';
      };
      notify = pkgs.writeShellApplication {
        name = "notify-send";
        text = ''
          printf '%s\n' "$*" >> "$XDG_RUNTIME_DIR/told"
          echo 1
        '';
      };
      paste = pkgs.writeShellScriptBin "wl-paste" "exec sleep 300";
    in
    pkgs.runCommandLocal "the-shell-hears-tuples-call" { } ''
      cp ${../tuple/Tuple.qml} Tuple.qml
      # Tuple's rows are named as the launcher's vocabulary says.
      mkdir launcher
      cp ${(account graphical).xdg.configFile."quickshell/Theme.qml".source} Theme.qml
      cp ${./Words.qml} Words.qml
      cp ${./launcher/words.json} launcher/words.json
      cp ${../tuple/Tuple.js} Tuple.js
      cp ${../../tests/tuple-call.qml} shell.qml
      cp ${niri} Niri.qml
      export HOME=$TMPDIR XDG_RUNTIME_DIR=$TMPDIR/runtime XDG_DATA_HOME=$TMPDIR/data
      export LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen
      export PATH=${people}/bin:${notify}/bin:${paste}/bin:$PATH
      mkdir -m 700 $XDG_RUNTIME_DIR
      mkdir -p $XDG_DATA_HOME/tuple/0
      touch $XDG_DATA_HOME/tuple/0/log.txt $XDG_RUNTIME_DIR/told
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      timeout 60 ${qs} -p shell.qml
      touch $out
    '';

  # Shape is a language (docs/design.md, Shape and Kinds): a border's width
  # says "this is a thing" and a radius says whole or part, so no surface
  # picks either for itself. Every corner is the theme's radius, or a part's
  # radius the theme derives from it, and every border is the theme's.
  quickshell-takes-its-shape-from-the-theme =
    let
      dir = ./.;
      files =
        lib.filter (lib.hasSuffix ".qml") (lib.attrNames (builtins.readDir dir))
        ++ map (name: "pieces/${name}") (
          lib.filter (lib.hasSuffix ".qml") (lib.attrNames (builtins.readDir (dir + "/pieces")))
        );
      allowed = {
        radius = [
          "Theme.radius"
          "Theme.inner"
        ];
        width = [ "Theme.border" ];
      };
      shape =
        line:
        let
          space = "[[:space:]]*";
          property = "(border\\.width|[a-zA-Z]*[rR]adius)";
          value = "([^;]*[^;[:space:]])";
          found = builtins.match (space + property + space + ":" + space + value + space + ";?" + space) line;
        in
        if found == null then
          null
        else
          {
            kind = if lib.head found == "border.width" then "width" else "radius";
            value = lib.elemAt found 1;
          };
      culprits = lib.concatMap (
        name:
        lib.concatMap (
          line:
          let
            found = shape line;
          in
          lib.optional (
            found != null && !(lib.elem found.value allowed.${found.kind})
          ) "${name}: ${lib.trim line}"
        ) (lib.splitString "\n" (builtins.readFile (dir + "/${name}")))
      ) files;
    in
    holds "quickshell-takes-its-shape-from-the-theme" ''
      These pick a radius or a border width of their own rather than the theme's:
      ${lib.concatStringsSep "\n" culprits}'' (files != [ ] && culprits == [ ]);

  # The shell follows dictation as its service says (tests/dictation-watch.py
  # stands in for it): each state said, a service that stops watched again
  # once its socket starts it, and a socket nothing listens on tried once,
  # never in a loop.
  the-shell-follows-dictation =
    let
      niri = pkgs.writeText "Niri.qml" ''
        pragma Singleton
        import Quickshell
        import QtQuick
        Singleton {
          property var windows: ({})
          property var casts: ({})
          function spawn(command) { }
          function act(request) { }
        }
      '';
    in
    pkgs.runCommandLocal "the-shell-follows-dictation" { } ''
      cp ${../dictation/Dictation.qml} Dictation.qml
      cp ${../../tests/dictation.qml} shell.qml
      cp ${niri} Niri.qml
      export HOME=$TMPDIR XDG_RUNTIME_DIR=$TMPDIR/runtime
      export LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen QT_FORCE_STDERR_LOGGING=1
      export FONTCONFIG_FILE=${noFonts}
      mkdir -m 700 $XDG_RUNTIME_DIR
      timeout 120 ${lib.getExe pkgs.python3} ${../../tests/dictation-watch.py} \
        ${qs}
      touch $out
    '';
}
// onLinux {
  # clippy finds nothing in the clipboard's history, its tests included.
  clipboard-history-passes-clippy = passesClippy "clipboard-history-passes-clippy" (
    pkgs.callPackage ./clipboard/package.nix { }
  );

  # clippy finds nothing in the window recorder, its tests included.
  record-window-passes-clippy = passesClippy "record-window-passes-clippy" (
    pkgs.callPackage ./record-window/package.nix { }
  );

  # clippy finds nothing in clock-follows, its tests included.
  clock-follows-passes-clippy = passesClippy "clock-follows-passes-clippy" (
    pkgs.callPackage ./clock-follows/package.nix { }
  );

  # clippy finds nothing in timekeeping, its tests included.
  timekeeping-passes-clippy = passesClippy "timekeeping-passes-clippy" (
    pkgs.callPackage ./timekeeping/package.nix { }
  );

  # clippy finds nothing in whereabouts, its tests included.
  whereabouts-passes-clippy = passesClippy "whereabouts-passes-clippy" (
    pkgs.callPackage ./whereabouts/package.nix { }
  );
}
