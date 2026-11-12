# What more than one of the folder's checks uses: stand-ins, the shell as the
# account gets it, and the harnesses that run the shell offscreen or on niri.
# Each place's file takes what it needs of these by name.
{
  account,
  bareNixos,
  graphical,
  lib,
  pkgs,
  self,
  shellFolderLint,
  ...
}:
let
  # What each file the checks copy imports of the shell's own (imports.nix).
  imports = import ./imports.nix;
  # A path under modules/quickshell, as an import in a file in folder names it.
  under =
    folder: path:
    lib.concatStringsSep "/" (
      lib.foldl' (kept: part: if part == ".." then lib.init kept else kept ++ [ part ]) [ ] (
        lib.filter (part: part != "" && part != ".") (lib.splitString "/" "${folder}/${path}")
      )
    );
  # What a QML or JavaScript file imports of the shell's own, as it says.
  importing =
    file:
    lib.unique (
      lib.concatMap (
        line:
        let
          named = builtins.match "[[:space:]]*[.]?import[[:space:]]+\"([^\"]+)\".*" line;
        in
        if named == null then [ ] else [ (under (dirOf file) (lib.head named)) ]
      ) (lib.splitString "\n" (builtins.readFile (../. + "/${file}")))
    );
  # Copies each file a check names into the folder it runs in, where the
  # shell's folder holds it, with everything it imports, once each. A file
  # whose imports aren't what imports.nix says fails the check instead,
  # naming what differs, so the table can't drift from the files. The
  # launcher's words.json is the one its places' files join to
  # (launcher/words.nix), as the shell reads it.
  copied =
    files:
    let
      bring =
        seen: file:
        if lib.elem file seen then seen else lib.foldl' bring (seen ++ [ file ]) (imports.${file} or [ ]);
      every = lib.sort lib.lessThan (lib.foldl' bring [ ] files);
      wrong = lib.filter (
        file:
        (lib.hasSuffix ".qml" file || lib.hasSuffix ".js" file)
        && lib.sort lib.lessThan (importing file) != imports.${file} or [ ]
      ) every;
      folders = lib.unique (lib.filter (folder: folder != ".") (map dirOf every));
      words = (import ../launcher/words.nix).file;
      source = file: if file == "launcher/words.json" then words else ../. + "/${file}";
    in
    if wrong == [ ] then
      lib.concatStringsSep "\n" (
        lib.optional (folders != [ ]) "mkdir -p ${lib.concatStringsSep " " folders}"
        ++ map (file: "cp ${source file} ${file}") every
      )
    else
      ''
        echo "imports.nix doesn't say what these import: ${lib.concatStringsSep ", " wrong}" >&2
        exit 1
      '';
  # Runs a test that times itself in Qt's engine, in a fresh process each
  # time, until one is fast enough. How fast the engine runs the same code
  # differs from one process to the next by as much as half, while a
  # slower search slows every process, so the fastest of five is what's
  # judged (tests/lib/qml-in-time.sh).
  timedQml = file: ''
    ${shellFolderLint}
    ${lib.getExe pkgs.bash} ${../../../tests/lib/qml-in-time.sh} ${pkgs.qt6.qtdeclarative} ${file}
  '';
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
  # The daemon Machine's answers come from, run over a check's own /proc.
  machine = pkgs.callPackage ../../machine/package.nix { };
  programs = pkgs.callPackage ../../programs/package.nix { };
  # The daemon the launcher's windows come from, hearing a check's own niri.
  windows = pkgs.callPackage ../../windows/package.nix { };
  # Every QML file the shell and the modules beside it draw, by its path
  # under modules/, its tests left out: a module's surface is drawn in the
  # theme as the shell's own are.
  everyShellFile =
    let
      modules = toString ../../.;
    in
    map (path: lib.removePrefix "${modules}/" (toString path)) (
      lib.filter (
        path: lib.hasSuffix ".qml" (toString path) && !(lib.hasInfix "/tests/" (toString path))
      ) (lib.filesystem.listFilesRecursive ../../.)
    );
  # The launcher's maths, a QML module the shell imports as Maths
  # (maths/package.nix), and where Qt finds it.
  mathsModules = "${pkgs.callPackage ../maths/package.nix { }}/lib/qt-6/qml";
  # Fonts for a shell drawn offscreen, which needs none of its own.
  noFonts = pkgs.makeFontsConf { fontDirectories = [ ]; };
  # The account's shell as it gets it, laid out in qs, with Slack's source for
  # People and the greeter beside it, for a check to read every file of.
  shellAsGiven =
    let
      files =
        lib.filterAttrs (name: _: lib.hasPrefix "quickshell/" name)
          (account graphical).xdg.configFile;
    in
    ''
      ${lib.concatLines (
        lib.mapAttrsToList (name: file: ''
          mkdir -p "$(dirname ${lib.escapeShellArg name})"
          cp ${file.source} ${lib.escapeShellArg name}
        '') files
      )}
      mv quickshell qs
      # Slack's source for People, its conversation's row and a message's,
      # Now's Slack piece and what its program row says are installed only
      # where a machine brings Slack, as no example does, so they're read
      # here as if one had.
      mkdir -p qs/launcher/people qs/pieces
      cp ${../../slack/SlackSource.qml} qs/launcher/people/slack.qml
      cp ${../../slack/Slack.js} qs/launcher/people/Slack.js
      cp ${../../slack/SlackConversation.qml} qs/SlackConversation.qml
      cp ${../../slack/SlackMessage.qml} qs/SlackMessage.qml
      cp ${../../slack/SlackMessage.js} qs/SlackMessage.js
      cp ${../../slack/SlackRows.qml} qs/SlackRows.qml
      cp ${../../slack/SlackUnreads.qml} qs/SlackUnreads.qml
      cp ${../../slack/SlackUnreads.js} qs/SlackUnreads.js
      cp ${../../slack/NewSlack.qml} qs/pieces/slack.qml
      cp ${../../slack/SlackProgram.qml} qs/launcher/slack-program.qml
      # The VMs come with a workstation's libvirt, which the account here
      # isn't, so its provider and what it imports are read as if it were.
      cp ${../../libvirt/Machines.qml} qs/launcher/vms.qml
      cp ${../../libvirt/Machines.js} qs/launcher/Machines.js
      # Neovim's and Chrome's acts are installed only where a machine brings
      # Neovim or Chrome, as the graphical example doesn't, so they're read
      # here as if one had.
      cp ${../../neovim/NeovimProgram.qml} qs/launcher/neovim.qml
      cp ${../../neovim/NeovimOpening.js} qs/launcher/NeovimOpening.js
      cp ${../../neovim/Neovim.js} qs/launcher/Neovim.js
      cp ${../../chrome/ChromeProgram.qml} qs/launcher/chrome.qml
      cp ${../../chrome/ChromeDownloads.js} qs/launcher/ChromeDownloads.js
      cp ${../../chrome/Tabs.qml} qs/launcher/tabs.qml
      # The greeter runs beside the account's files, as greetd's module lays
      # them out, with what it's built with standing in.
      cp ${
        pkgs.replaceVars ../../greetd/greeter.qml {
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
    '';

  # The account's shell on its real niri, with its own configuration,
  # nested in a headless sway in the sandbox rather than in a machine
  # (tests/nested-niri.sh): a command run against it, which starts the
  # shell from the folder its root is in, as quickshell, and logs to logs.
  # The account says what its shell does, as the desktop test's does, and
  # counts as away after so many seconds of nothing typed or moved. Its
  # notices daemon serves the bus beside it, started as its unit would be
  # (tests/notices-beside.sh), logging to logs.
  shellOnNiri =
    {
      name,
      away,
      command,
      # The shell's root file, or null for the account's whole shell.
      root ? null,
      # What more the command runs, besides niri, the shell and a bus.
      inputs ? [ ],
      # What more the account's shell is given, as a stand-in for a program.
      shell ? { },
    }:
    let
      home = account (bareNixos [
        self.nixosModules.graphical
        {
          home-manager.users.someone.jitsusama.quickshell = lib.mkMerge [
            {
              reports = true;
              notices.away = away;
              # A caret that holds still, as the desktop test's does, so a
              # launcher left still draws nothing.
              caret.fade = 0;
            }
            shell
          ];
        }
      ]);
      # The shell, niri's configuration, and the theme the shell reads its
      # colours from. A loaded builder can stall the shell past niri's
      # repeat delay between a key's press and its release, and Qt then
      # repeats the key: one k moved the held notices two lines. No check
      # here holds a key, so none repeats.
      given =
        lib.filterAttrs (
          name: _:
          lib.any (folder: lib.hasPrefix folder name) [
            "quickshell/"
            "niri/"
            "jitsusama/"
          ]
        ) home.xdg.configFile
        // {
          "niri/keyboard.kdl".source = pkgs.writeText "keyboard.kdl" ''
            input {
                keyboard {
                    repeat-delay 60000
                }
            }
          '';
        };
      inherit (pkgs) mesa;
    in
    pkgs.runCommandLocal name
      {
        nativeBuildInputs = [
          graphical.config.programs.niri.package
          graphical.pkgs.quickshell
          pkgs.dbus
          pkgs.sway-unwrapped
        ]
        ++ inputs;
        LANG = "C.UTF-8";
        FONTCONFIG_FILE = pkgs.makeFontsConf { fontDirectories = graphical.config.fonts.packages; };
        # niri and Qt draw in software, as the desktop's machine does.
        __EGL_VENDOR_LIBRARY_FILENAMES = "${mesa}/share/glvnd/egl_vendor.d/50_mesa.json";
        LIBGL_DRIVERS_PATH = "${mesa}/lib/dri";
        GBM_BACKENDS_PATH = "${mesa}/lib/gbm";
        LIBGL_ALWAYS_SOFTWARE = 1;
        QT_QPA_PLATFORM = "wayland";
        QT_QUICK_BACKEND = "software";
        # What the machine's niri gives the session it starts
        # (modules/niri's on-demand-when-asked.patch), set here since the
        # shell starts beside this niri rather than under it.
        NIRI_ON_DEMAND_WHEN_ASKED = 1;
        NOTICES_DAEMON = home.systemd.user.services.notices.Service.ExecStart;
        NOTICES_LOG = "logs/notices.log";
      }
      ''
        export HOME=$PWD XDG_CONFIG_HOME=$PWD XDG_STATE_HOME=$PWD/state
        export XDG_RUNTIME_DIR=$(mktemp -d)
        ${lib.concatLines (
          lib.mapAttrsToList (name: file: "install -D -m 644 ${file.source} ${lib.escapeShellArg name}") given
        )}
        ${lib.optionalString (root != null) "install -m 644 ${root} quickshell/shell.qml"}
        ${lib.getExe pkgs.bash} ${../../../tests/nested-niri.sh} niri/config.kdl logs \
          ${lib.getExe pkgs.bash} ${../../../tests/notices-beside.sh} ${command}
        touch $out
      '';

  # What a chord runs, as the account's niri binds it, word by word: niri in
  # a window reads keys through its own keymap, and no keyboard here types
  # into it, so a check runs the bind's own command.
  spawnedBy =
    chord:
    let
      bind = lib.findFirst (
        bind: bind.chord == chord
      ) (throw "Nothing is bound to ${chord}.") (account graphical).jitsusama.niri.binds;
      spawned = builtins.match "spawn (.*);" bind.action;
    in
    assert spawned != null;
    lib.filter (word: word != "" && word != " ") (lib.splitString "\"" (lib.head spawned));

  # The account's notices alone on niri (tests/notices.qml): a script run
  # against them, given the shell's folder, the logs and what more it's
  # given.
  noticesOnNiri =
    {
      name,
      away,
      script,
      given ? [ ],
    }:
    shellOnNiri {
      inherit name away;
      root = ../../../tests/notices.qml;
      inputs = [
        pkgs.jq
        pkgs.libnotify
      ];
      command = "${lib.getExe pkgs.bash} ${script} quickshell logs ${lib.escapeShellArgs given}";
    };

  # The account's whole shell on niri, its launcher typed into through
  # wtype, with a file of tests run through pytest against it
  # (tests/launcher_on_niri.py starts the shell and types).
  launcherOnNiri =
    {
      name,
      tests,
      given ? [ ],
      shell ? { },
    }:
    let
      python = pkgs.python3.withPackages (python: [ python.pytest ]);
      checked = pkgs.runCommandLocal "${name}-tests" { } ''
        install -D -m 644 ${../../../tests/launcher_on_niri.py} $out/launcher_on_niri.py
        install -D -m 644 ${tests} $out/${baseNameOf tests}
      '';
    in
    shellOnNiri {
      inherit name shell;
      # Away only after an hour, since nobody types here but the check.
      away = 3600;
      # foot gives the launcher windows to find, drawn without a GPU,
      # notify-send the notices something to hold, and grim niri's screen
      # to check an opening's presented time against.
      inputs = [
        pkgs.foot
        pkgs.grim
        pkgs.libnotify
        pkgs.wl-clipboard
        pkgs.wtype
      ];
      command = lib.escapeShellArgs (
        [
          "env"
          "SHELL_FOLDER=quickshell"
          "LOGS=logs"
          "TAKE_NOTICES=${lib.escapeShellArgs (spawnedBy "Super+N")}"
          "PYTHONPATH=${checked}"
          # Machine's answers come from its daemon, as the account's do.
          "MACHINE=${lib.getExe machine}"
          # And the words it says Machine's rows in, as the account's has.
          "MACHINE_WORDS=${(import ../launcher/words.nix).file}"
          # The windows come from theirs, hearing the nested niri, and the
          # screens' rows in the words the account's has too.
          "WINDOWS=${lib.getExe windows}"
          "WINDOWS_WORDS=${(import ../launcher/words.nix).file}"
          # And Programs' rows from the programs daemon, over the entries
          # here, given no program's lists, since none has any here.
          "PROGRAMS=${lib.getExe programs}"
          "PROGRAMS_SOURCES=${pkgs.writeText "no-sources.json" "[]"}"
        ]
        # What a file of tests is given besides, as a program it runs.
        ++ given
        ++ [
          (lib.getExe python)
          "-m"
          "pytest"
          "-p"
          "no:cacheprovider"
          "-v"
          "${checked}/${baseNameOf tests}"
        ]
      );
    };

  # Now itself in Quickshell, offscreen, `test` its shell.qml, with the
  # margin's pieces, the agents' waiting list, the weather and the calendar
  # stood in, and Location.qml's own, reading whereabouts over its socket.
  # `before` readies the home a case needs, before the shell starts.
  nowShown =
    {
      name,
      test,
      before ? "",
    }:
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
        property var asked: []
        property var settled: ({})
        function session(each) { return null; }
        function watched(each) { return true; }
        function waitsFor(each) { return null; }
      '';
      # Where an item is done names its session, and For You lists
      # the sessions wanting Joel and those to reopen.
      sessions = ../tests/stand-ins/now-sessions.qml;
      # A look For You opens is opened through niri, which nothing here does.
      niri = singleton "Niri.qml" ''
        function spawn(command) {}
      '';
      # Now says a minute turned for the opening test, which these don't
      # read; without it, a minute turning mid-case threw before the time
      # was marked stale.
      report = singleton "Report.qml" "function say(text) {}";
      # For You's Desktop row holds the notices unread, as the notices
      # daemon serves them (tests/lib/NoticesDaemon.qml stands it in), and
      # For You counts a critical one among what waits while no card says
      # it.
      notices = ../tests/stand-ins/now-notices.qml;
      whereabouts = pkgs.callPackage ../whereabouts/package.nix { };
    in
    pkgs.runCommandLocal name { } ''
      mkdir launcher pieces
      ${copied [
        "Daemon.qml"
        "ForYouHolds.qml"
        "Location.qml"
        "Words.qml"
        "launcher/Now.qml"
        "launcher/PastNotices.qml"
        "launcher/Provider.qml"
        "launcher/ReadyRows.qml"
        "launcher/words.json"
      ]}
      cp ${(account graphical).xdg.configFile."quickshell/Theme.qml".source} Theme.qml
      cp ${test} shell.qml
      cp ${../../../tests/lib/Expect.js} Expect.js
      cp ${../../../tests/lib/NoticesDaemon.qml} NoticesDaemon.qml
      cp ${../../../tests/lib/Watch.js} Watch.js
      cp ${facts} Facts.qml
      cp ${waiting} WaitingOnYou.qml
      cp ${sessions} Sessions.qml
      cp ${niri} Niri.qml
      cp ${report} Report.qml
      cp ${notices} Notices.qml
      export HOME=$TMPDIR XDG_RUNTIME_DIR=$TMPDIR/runtime LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen
      mkdir -m 700 $XDG_RUNTIME_DIR
      mkdir -p $XDG_RUNTIME_DIR/whereabouts $XDG_RUNTIME_DIR/daemons root/etc
      ln -s /etc/zoneinfo/America/Halifax root/etc/localtime
      echo '{"v":2,"latitude":12.3457,"longitude":-98.7654,"accuracy":26000,'\
      '"when":1790000000,"place":"Kelpwater, Westmark","zone":"America/Toronto",'\
      '"country":"Canada","source":"wifi"}' \
        > $XDG_RUNTIME_DIR/whereabouts/here.json
      ${lib.getExe whereabouts} $XDG_RUNTIME_DIR/daemons/location.sock root \
        $XDG_RUNTIME_DIR/whereabouts/here.json $TMPDIR/pin.json $TMPDIR/moved.json \
        --words ${(import ../launcher/words.nix).file} &
      export FONTCONFIG_FILE=${noFonts}
      ${before}
      ${shellFolderLint}
      timeout 60 ${qs} -p shell.qml
      touch $out
    '';
in
{
  inherit
    copied
    timedQml
    singleton
    qs
    machine
    programs
    windows
    everyShellFile
    mathsModules
    noFonts
    shellAsGiven
    shellOnNiri
    spawnedBy
    noticesOnNiri
    launcherOnNiri
    nowShown
    ;
}
