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
  # The daemon Machine's answers come from, run over a check's own /proc.
  machine = pkgs.callPackage ../machine/package.nix { };
  programs = pkgs.callPackage ../programs/package.nix { };
  # The daemon the launcher's windows come from, hearing a check's own niri.
  windows = pkgs.callPackage ../windows/package.nix { };
  # Every QML file the shell and the modules beside it draw, by its path
  # under modules/, its tests left out: a module's surface is drawn in the
  # theme as the shell's own are.
  everyShellFile =
    let
      modules = toString ../.;
    in
    map (path: lib.removePrefix "${modules}/" (toString path)) (
      lib.filter (
        path: lib.hasSuffix ".qml" (toString path) && !(lib.hasInfix "/tests/" (toString path))
      ) (lib.filesystem.listFilesRecursive ../.)
    );
  # The launcher's maths, a QML module the shell imports as Maths
  # (maths/package.nix), and where Qt finds it.
  mathsModules = "${pkgs.callPackage ./maths/package.nix { }}/lib/qt-6/qml";
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
      cp ${../slack/SlackSource.qml} qs/launcher/people/slack.qml
      cp ${../slack/Slack.js} qs/launcher/people/Slack.js
      cp ${../slack/SlackConversation.qml} qs/SlackConversation.qml
      cp ${../slack/SlackMessage.qml} qs/SlackMessage.qml
      cp ${../slack/SlackMessage.js} qs/SlackMessage.js
      cp ${../slack/SlackUnreads.qml} qs/SlackUnreads.qml
      cp ${../slack/SlackUnreads.js} qs/SlackUnreads.js
      cp ${../slack/NewSlack.qml} qs/pieces/slack.qml
      cp ${../slack/SlackProgram.qml} qs/launcher/slack-program.qml
      # The VMs come with a workstation's libvirt, which the account here
      # isn't, so its provider and what it imports are read as if it were.
      cp ${../libvirt/Machines.qml} qs/launcher/vms.qml
      cp ${../libvirt/Machines.js} qs/launcher/Machines.js
      # Neovim's and Chrome's acts are installed only where a machine brings
      # Neovim or Chrome, as the graphical example doesn't, so they're read
      # here as if one had.
      cp ${../neovim/NeovimProgram.qml} qs/launcher/neovim.qml
      cp ${../neovim/NeovimOpening.js} qs/launcher/NeovimOpening.js
      cp ${../neovim/Neovim.js} qs/launcher/Neovim.js
      cp ${../chrome/ChromeProgram.qml} qs/launcher/chrome.qml
      cp ${../chrome/ChromeDownloads.js} qs/launcher/ChromeDownloads.js
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
    '';

  # The account's shell on its real niri, with its own configuration,
  # nested in a headless sway in the sandbox rather than in a machine
  # (tests/nested-niri.sh): a command run against it, which starts the
  # shell from the folder its root is in, as quickshell, and logs to logs.
  # The account says what its shell does, as the desktop test's does, and
  # counts as away after so many seconds of nothing typed or moved.
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
      }
      ''
        export HOME=$PWD XDG_CONFIG_HOME=$PWD XDG_STATE_HOME=$PWD/state
        ${lib.concatLines (
          lib.mapAttrsToList (name: file: "install -D -m 644 ${file.source} ${lib.escapeShellArg name}") given
        )}
        ${lib.optionalString (root != null) "install -m 644 ${root} quickshell/shell.qml"}
        ${lib.getExe pkgs.bash} ${../../tests/nested-niri.sh} niri/config.kdl logs ${command}
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
      root = ../../tests/notices.qml;
      inputs = [ pkgs.libnotify ];
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
        install -D -m 644 ${../../tests/launcher_on_niri.py} $out/launcher_on_niri.py
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
          "MACHINE_WORDS=${./launcher/words.json}"
          # The windows come from theirs, hearing the nested niri.
          "WINDOWS=${lib.getExe windows}"
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
      sessions = ./tests/stand-ins/now-sessions.qml;
      # A look For You opens is opened through niri, which nothing here does.
      niri = singleton "Niri.qml" ''
        function spawn(command) {}
      '';
      # For You's Desktop row holds the notices unread, as their own rows
      # say them, and For You counts a critical one among what waits while
      # no card says it.
      notices = ./tests/stand-ins/now-notices.qml;
      whereabouts = pkgs.callPackage ./whereabouts/package.nix { };
    in
    pkgs.runCommandLocal name { } ''
      mkdir launcher pieces
      cp ${./ForYouHolds.qml} ForYouHolds.qml
      cp ${./pieces/Holding.js} pieces/Holding.js
      cp ${(account graphical).xdg.configFile."quickshell/Theme.qml".source} Theme.qml
      cp ${./Fuzzy.js} Fuzzy.js
      cp ${./Marks.js} Marks.js
      cp ${./launcher/Provider.qml} launcher/Provider.qml
      cp ${./ProgramGlyphs.js} ProgramGlyphs.js
      cp ${./launcher/Asking.js} launcher/Asking.js
      cp ${./launcher/Numbers.js} launcher/Numbers.js
      cp ${./launcher/Now.qml} launcher/Now.qml
      cp ${./launcher/PastNotices.qml} launcher/PastNotices.qml
      cp ${./launcher/Notice.js} launcher/Notice.js
      cp ${./NoticeBody.js} NoticeBody.js
      cp ${./launcher/NeedsYou.js} launcher/NeedsYou.js
      cp ${./launcher/Moment.js} launcher/Moment.js
      cp ${./launcher/NowGroups.js} launcher/NowGroups.js
      cp ${./launcher/Showing.js} launcher/Showing.js
      cp ${./launcher/Order.js} launcher/Order.js
      cp ${./launcher/ForYou.js} launcher/ForYou.js
      cp ${./launcher/ForYouFilter.js} launcher/ForYouFilter.js
      cp ${./launcher/Lately.js} launcher/Lately.js
      cp ${./launcher/PlaceRow.js} launcher/PlaceRow.js
      cp ${./launcher/Ago.js} launcher/Ago.js
      cp ${./launcher/Here.js} launcher/Here.js
      cp ${./When.js} When.js
      cp ${./Words.qml} Words.qml
      cp ${./launcher/words.json} launcher/words.json
      cp ${test} shell.qml
      cp ${../../tests/lib/Expect.js} Expect.js
      cp ${facts} Facts.qml
      cp ${waiting} WaitingOnYou.qml
      cp ${sessions} Sessions.qml
      cp ${niri} Niri.qml
      cp ${notices} Notices.qml
      cp ${./Location.qml} Location.qml
      cp ${./Daemon.qml} Daemon.qml
      export HOME=$TMPDIR XDG_RUNTIME_DIR=$TMPDIR/runtime LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen
      mkdir -m 700 $XDG_RUNTIME_DIR
      mkdir -p $XDG_RUNTIME_DIR/whereabouts root/etc
      ln -s /etc/zoneinfo/America/Halifax root/etc/localtime
      echo '{"v":2,"latitude":12.3457,"longitude":-98.7654,"accuracy":26000,'\
      '"when":1790000000,"place":"Kelpwater, Westmark","zone":"America/Toronto",'\
      '"country":"Canada","source":"wifi"}' \
        > $XDG_RUNTIME_DIR/whereabouts/here.json
      ${lib.getExe whereabouts} $XDG_RUNTIME_DIR/daemons/location.sock root \
        $XDG_RUNTIME_DIR/whereabouts/here.json $TMPDIR/pin.json $TMPDIR/moved.json \
        --words ${./launcher/words.json} &
      export FONTCONFIG_FILE=${noFonts}
      ${before}
      ${shellFolderLint}
      timeout 60 ${qs} -p shell.qml
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
    given = spawnedBy "Super+Shift+N";
  };

  # A notice that comes while Joel's away waits, past the time its sender
  # gave it (tests/notices-wait-while-away.sh). Away after a second, so he
  # is before it comes.
  the-notices-wait-while-joel-is-away = noticesOnNiri {
    name = "the-notices-wait-while-joel-is-away";
    away = 1;
    script = ../../tests/notices-wait-while-away.sh;
  };

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
        exec ${lib.getExe pkgs.bash} ${../../tests/warp-cli-stand-in.sh} "$@"
      '';
      warp = lib.getExe (pkgs.callPackage ../warp/package.nix { });
    in
    launcherOnNiri {
      name = "the-launcher-answers-whats-typed-on-niri";
      tests = ../../tests/launcher-on-niri-test.py;
      given = [
        "MEANING=${
          pkgs.writers.writePython3 "stand-in-meaning" {
            doCheck = false;
          } (builtins.readFile ../../tests/desktop/stand-in-meaning.py)
        }"
        "WARP_CONTRACT=${../warp/contract}"
        "WARP=${pkgs.writeShellScript "warp-serve" ''
          WARP_CLI=${warpCli} exec ${warp} serve "$@"
        ''}"
      ];
      shell.launcher.providers.warp = {
        enable = true;
        warpCli = toString warpCli;
      };
    };

  # An agent's row does, on Return, the act its state makes harmless, its
  # rows the agents daemon's, made from the realm's own node's sessions, as
  # a workstation's are (tests/agents-on-niri-test.py).
  an-agents-return-does-its-harmless-act-on-niri = launcherOnNiri {
    name = "an-agents-return-does-its-harmless-act-on-niri";
    tests = ../../tests/agents-on-niri-test.py;
    given = [
      "MYCELIUM=${lib.getExe self.inputs.mycelium.packages.x86_64-linux.default}"
      "AGENTS=${lib.getExe (pkgs.callPackage ../agents/package.nix { })}"
      "WORDS=${./launcher/words.json}"
    ];
  };

  # The Passwords place asks the passwords daemon over its socket, its one
  # manager a stand-in for 1Password's that never runs op: nothing until
  # Joel steps in, unlocking it from its first row, an entry's Return
  # copying its password through wl-copy as the launcher goes, the margin
  # wearing the clipboard's mark and the foot, opened again, counting to
  # its clearing, resting on one reading it in full, stepping into one
  # holding its acts in order, and Open the Website handing a stand-in
  # Chrome its website as the launcher goes
  # (tests/passwords-on-niri-test.py). A VM adds only the units, which
  # passwords-runs-each-password-manager reads.
  the-passwords-place-copies-through-its-daemon-on-niri = launcherOnNiri {
    name = "the-passwords-place-copies-through-its-daemon-on-niri";
    tests = ../../tests/passwords-on-niri-test.py;
    given = [
      "PASSWORDS=${lib.getExe (pkgs.callPackage ../passwords/package.nix { })}"
      "PASSWORDS_STAND_IN=${../../tests/passwords-stand-in.sh}"
    ];
    # On here, where no agent runs and no secret is real.
    shell.launcher.providers.passwords.off = lib.mkForce false;
  };

  # An opening draws home as the closed launcher kept it, each row as it
  # stood, then fills it in where its rows stand with nothing flashing; and
  # keys typed as it opens all land (tests/opening-on-niri-test.py).
  the-launcher-opens-on-what-it-kept-on-niri = launcherOnNiri {
    name = "the-launcher-opens-on-what-it-kept-on-niri";
    tests = ../../tests/opening-on-niri-test.py;
  };

  # The launcher, the held notices and a window share the keys by one rule:
  # the newest Joel opened takes them, a notice arriving never does, and a
  # click on a window gives it them (tests/focus-on-niri-test.py).
  the-newest-surface-takes-the-keys-on-niri = launcherOnNiri {
    name = "the-newest-surface-takes-the-keys-on-niri";
    tests = ../../tests/focus-on-niri-test.py;
    given = [ "WLRCTL=${lib.getExe pkgs.wlrctl}" ];
  };

  # Super+V's paste lands in the terminal under within 50 ms of Return
  # though the shell is held stopped once it has asked, a stand-in for its
  # thread busy after an opening: the clipboard's history serves the copy
  # and presses the paste keys (tests/pastes-on-niri-test.py).
  a-paste-lands-while-the-shell-is-busy-on-niri = launcherOnNiri {
    name = "a-paste-lands-while-the-shell-is-busy-on-niri";
    tests = ../../tests/pastes-on-niri-test.py;
    given = [ "CLIPBOARD=${lib.getExe (pkgs.callPackage ./clipboard/package.nix { })}" ];
  };

  # Turn Every Screen Off and :set noscreens ask niri to power its monitors
  # off, which keeps each output connected, read on their way to niri; and
  # a scale niri refuses, turned into its ReturnError on the way, says
  # niri's words for why on the warning line.
  the-screens-go-off-on-niri = launcherOnNiri {
    name = "the-screens-go-off-on-niri";
    tests = ../../tests/screens-off-on-niri-test.py;
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

  # A switch never restarts the kitty every session's window is in, so no
  # switch closes them.
  the-sessions-kitty-outlives-a-switch =
    holds "the-sessions-kitty-outlives-a-switch"
      "A switch would restart the sessions' kitty, closing every session's window."
      (
        ((account graphical).systemd.user.services.sessions-kitty.Unit.X-SwitchMethod or null) == "keep-old"
      );

  # The sessions' kitty draws only the frames niri asks for, so the windows
  # niri doesn't show stop being drawn.
  the-sessions-kitty-keeps-niris-pace =
    let
      service = (account graphical).systemd.user.services.sessions-kitty.Service;
    in
    holds "the-sessions-kitty-keeps-niris-pace"
      "The sessions' kitty draws as it pleases, so every hidden session's window is drawn too."
      (lib.any (lib.hasInfix "-o sync_to_monitor=yes") (lib.toList (service.ExecStart or [ ])));

  # A machine shapes what the overview shows in a line a change: a piece
  # moved, cut, shown only while true, a threshold changed and the rest kept,
  # and a piece of its own linked beside the built-in ones.
  the-margin-takes-its-pieces =
    let
      own = pkgs.writeText "wide.qml" "Piece { value: settings.text }";
      machine = bareNixos [
        self.nixosModules.workstation
        self.nixosModules.graphical
        {
          home-manager.users.someone.jitsusama.quickshell.margin.pieces = {
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
      rendered = lib.findFirst (lib.hasInfix "property var inOrder: ") "" (
        lib.splitString "\n" home.xdg.configFile."quickshell/Pieces.qml".text
      );
      drawn = builtins.fromJSON (lib.last (lib.splitString "property var inOrder: " rendered));
      piece = lib.listToAttrs (map (piece: lib.nameValuePair piece.name piece) drawn);
    in
    holds "the-margin-takes-its-pieces"
      "The margin didn't draw a machine's own pieces as it set them: their order, their cuts, their settings or its own file."
      (
        map (piece: piece.name) drawn == [
          "time"
          "network"
          "battery"
          "devices"
          "volume"
          "tokens"
          "spend"
          "pi"
          "notices"
          "forYou"
          "google"
          "meeting"
          "quiet"
          "disk"
          "lag"
          "awake"
          "wide"
          "mail"
          "profile"
          "clearing"
          "microphone"
          "capslock"
          "layer"
          "call"
          "meet"
          "sharing"
          "camera"
          "recording"
          "timers"
          "playing"
        ]
        && piece.battery.low == 15
        && piece.battery.critical == 10
        && piece.volume.tier == "when"
        && piece.wide.mark == "on"
        && piece.wide.file == "wide.qml"
        && "${home.xdg.configFile."quickshell/pieces/wide.qml".source}" == "${own}"
      );

  # Qt's own compiler takes every file of the shell as the account gets it,
  # the greeter's included, on the Qt the shell runs on
  # (modules/quickshell/tests/qml-compiles.sh).
  # It refuses what the shell's engine can't run, such as an object spread,
  # and crashes on what would crash the shell as it loads the file, which
  # qmllint and the checks that run a file offscreen can both pass.
  quickshell-compiles-every-file =
    let
      qt = graphical.pkgs.kdePackages.qtdeclarative;
    in
    pkgs.runCommandLocal "quickshell-compiles-every-file" { LC_ALL = "C.UTF-8"; } ''
      ${shellAsGiven}
      ${lib.getExe pkgs.bash} ${./tests/qml-compiles.sh} ${qt}/libexec/qmlcachegen qs
      touch $out
    '';

  # qmllint reads the shell as the account gets it, and the greeter beside it,
  # as the module qs, against
  # Quickshell's own type files as quickshellTypes corrects them, and any
  # warning fails. Two kinds of report are left out, since each one here is
  # Quickshell's files, not the shell: a property typed by a name its files
  # don't export (AuthFlow, BluetoothAdapter and the like), and a handler
  # whose parameter is a Qt enum QML can't see (QProcess::ExitStatus).
  # Every message qmllint counts is printed, as its own report can leave
  # one out and fail showing only infos (tests/lib/qmllint-counted.sh).
  quickshell-passes-qmllint =
    let
      qt = pkgs.kdePackages.qtdeclarative;
      lint = lib.getExe (pkgs.callPackage ../../tests/lib/qmllint-counted.nix { });
    in
    # In UTF-8, which Qt otherwise switches to and says so.
    pkgs.runCommandLocal "quickshell-passes-qmllint" { LC_ALL = "C.UTF-8"; } ''
      ${shellAsGiven}
      # Quickshell finds a folder's files as a module by its path; qmllint
      # needs a qmldir saying so (modules/quickshell/tests/qmldir.sh).
      ${lib.getExe pkgs.bash} ${./tests/qmldir.sh} qs
      # Each file is named by its real, whole path, as Qt 6.12's qmllint
      # names a qmldir's entries: named any other way, the file it lints and
      # the qmldir's entry are two types to it, the first never told it's a
      # singleton.
      ${lint} --ignore-settings --max-warnings 0 \
        -I ${quickshellTypes}/lib/qt-6/qml -I ${qt}/lib/qt-6/qml -I . \
        --unresolved-type disable --signal-handler-parameters disable \
        $(find "$(pwd -P)/qs" -name '*.qml' | sort)
      touch $out
    '';

  # Every file a shell file imports by its path is installed beside it, as
  # home.nix's list lays them out. One left off the list only warns as the
  # shell starts, and the component needing it is never made: Firmware.js
  # was found that way, by a VM test, 25 minutes in (2026-10-30).
  quickshell-installs-what-it-imports =
    pkgs.runCommandLocal "quickshell-installs-what-it-imports" { }
      ''
        ${shellAsGiven}
        ${lib.getExe pkgs.bash} ${./tests/imports-installed.sh} qs
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

  # The launcher makes a text ready to match without growing the shell's
  # memory by more than a few kilobytes a text, kept as it keeps them
  # (tests/fuzzy-light.qml), in a fresh process of Qt's own JavaScript
  # engine reading its own memory.
  the-launcher-makes-texts-ready-lightly =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "the-launcher-makes-texts-ready-lightly" { } ''
      cp ${../../tests/fuzzy-light.qml} fuzzy-light.qml
      cp ${./Fuzzy.js} Fuzzy.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts} QML_XHR_ALLOW_FILE_READ=1
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 120 ${qt}/bin/qml fuzzy-light.qml
      touch $out
    '';

  # Every run of text the shell sets names its type's level, never a step, a
  # weight or a spacing of its own (type-is-set-by-level.py). So the check
  # can't pass by seeing nothing, each way of straying is shown to fail
  # first: a step, a level mixed with a step, a line carried on and a
  # script's assignment. The reader's page and its foot, Typst
  # held in Rust, name a level too, and a size, a weight, a tracking, a
  # strong's delta or an axis said by number fails, as a size named with one
  # does, while a comment saying one doesn't.
  type-is-set-by-level =
    let
      check = "${lib.getExe pkgs.python3} ${./type-is-set-by-level.py}";
      shells = lib.fileset.toSource {
        root = ../.;
        fileset = lib.fileset.fileFilter (file: file.hasExt "qml") ../.;
      };
      pages = [
        ../reader/src/page.rs
        ../reader/src/prompt.rs
      ];
    in
    pkgs.runCommandLocal "type-is-set-by-level" { } ''
      # A file that sets its type as given, and what the check must say of it.
      strays() {
        rm -rf modules
        mkdir -p modules/shell
        printf '%s\n' "$1" > modules/shell/Surface.qml
        if ${check} modules > said; then
          echo "The type check passed on: $1"
          exit 1
        fi
        if ! grep -qF -- "$2" said; then
          echo "Of $1 the type check didn't say: $2"
          cat said
          exit 1
        fi
      }
      # A page's template as given, and what the check must say of it.
      sets() {
        rm -rf modules
        mkdir -p modules
        printf '%s\n' "$1" > page.rs
        if ${check} modules page.rs > said; then
          echo "The type check passed on the page: $1"
          exit 1
        fi
        if ! grep -qF -- "$2" said; then
          echo "Of the page $1 the type check didn't say: $2"
          cat said
          exit 1
        fi
      }
      strays 'font.pointSize: Theme.mono.large' 'Surface.qml:1: font.pointSize'
      strays 'font.pointSize: row.big ? Theme.level.title.pointSize : Theme.mono.large' \
        'font.pointSize is row.big'
      strays $'font.weight: row.selected\n    ? Font.Medium : Theme.level.name.weight' \
        'font.weight is row.selected ? Font.Medium'
      strays 'label.font.letterSpacing = 2' 'font.letterSpacing is 2'
      sets '#set text(..level.words.text, size: 0.93 * body-size)' 'page.rs:1: size is 0.93'
      sets '#show heading: set text(weight: 540, tracking: -0.02em)' 'weight is 540'
      sets '#show heading: set text(weight: 540, tracking: -0.02em)' 'tracking is -0.02em'
      sets '#set strong(delta: 100)' 'delta is 100'
      sets '#let display-axes = (opsz: 144, SOFT: 100, WONK: 1)' 'opsz is 144'
      sets '#let code-size = 10.5pt' 'code-size is 10.5pt'
      sets '#let heading-weight = 540' 'heading-weight is 540'
      sets $'#[cfg(test)]\nconst ROWS: u8 = 3;\n#let lede-size = 12pt' 'lede-size is 12pt'
      # A level, read from the theme or a property holding one, passes, as
      # a comment saying a size and the test module after the page's own
      # lines do.
      mkdir -p modules/shell
      printf '%s\n' 'font.pointSize: Theme.level.note.pointSize * set.scale' \
        'font.weight: chord.level.weight' 'font: other.font' > modules/shell/Surface.qml
      printf '%s\n' '#set text(..level.note.text, fill: ink.on_accent)' \
        '/// The words at 13.69 pt, weight: 380.' '// size: 12pt, as a note says' \
        '#[cfg(test)]' 'mod tests {' 'size: 11.5pt' > page.rs
      ${check} modules page.rs
      ${check} ${shells} ${lib.concatStringsSep " " pages}
      touch $out
    '';

  # Every space the shell sets names the rung of the ladder it sits between,
  # the beat or a level's pitch, never a step of the size scale
  # (space-is-set-by-rung.py). So it can't pass by seeing nothing, a step and
  # a step summed with a rung are each shown to fail first, while a rung, the
  # beat, a pitch and a comment naming a step pass.
  space-is-set-by-rung =
    let
      check = "${lib.getExe pkgs.python3} ${./space-is-set-by-rung.py}";
      shells = lib.fileset.toSource {
        root = ../.;
        fileset = lib.fileset.fileFilter (file: file.hasExt "qml") ../.;
      };
    in
    pkgs.runCommandLocal "space-is-set-by-rung" { } ''
      # A file that sets its space as given, and what the check must say of it.
      strays() {
        rm -rf modules
        mkdir -p modules/shell
        printf '%s\n' "$1" > modules/shell/Surface.qml
        if ${check} modules > said; then
          echo "The space check passed on: $1"
          exit 1
        fi
        if ! grep -qF -- "$2" said; then
          echo "Of $1 the space check didn't say: $2"
          cat said
          exit 1
        fi
      }
      strays 'leftPadding: Theme.space.l' 'Surface.qml:1: leftPadding: Theme.space.l'
      strays 'spacing: Theme.space.rung.items + Theme.space.xs' 'Theme.space.xs'
      # A rung, the beat and a pitch pass, as a comment naming a step does.
      rm -rf modules
      mkdir -p modules/shell
      printf '%s\n' 'spacing: Theme.space.rung.items' 'height: Theme.space.beat * 2' \
        'lineHeight: Theme.level.note.pitch' '// once Theme.space.l, now a rung' \
        > modules/shell/Surface.qml
      ${check} modules
      ${check} ${shells}
      touch $out
    '';

  # The launcher, the held notices and a notice are one card: each takes
  # its frame from Card.qml and stands its ink on the card's content, the
  # inset from its edge, and on its columns (Columns.js), never drawing a
  # frame, reading the inset, placing ink across by hand or tinting a row's
  # ground itself (every-card-is-the-one-card.py). So it can't pass by
  # seeing nothing, a border, the whole thing's radius, a shadow, a frame
  # from Floating, a read of the inset, an x or a margin from a rung or a
  # pitch, and a ground's tint are each shown to fail first, while a card, a
  # part's radius, a column, the card's own inset and a comment pass.
  every-card-is-the-one-card =
    let
      check = "${lib.getExe pkgs.python3} ${./every-card-is-the-one-card.py}";
      cards = [
        "HeldNotices.qml"
        "Launcher.qml"
        "NoticeRow.qml"
        "Notifications.qml"
      ];
      surfaces = lib.fileset.toSource {
        root = ./.;
        fileset = lib.fileset.unions (map (name: ./. + "/${name}") cards);
      };
    in
    pkgs.runCommandLocal "every-card-is-the-one-card" { } ''
      # A card surface drawn as given, and what the check must say of it.
      strays() {
        rm -rf shell
        mkdir shell
        printf '%s\n' "$1" > shell/Surface.qml
        if ${check} shell Surface.qml > said; then
          echo "The card check passed on: $1"
          exit 1
        fi
        if ! grep -qF -- "$2" said; then
          echo "Of $1 the card check didn't say: $2"
          cat said
          exit 1
        fi
      }
      strays 'border.width: Theme.border' 'Surface.qml:1: border.width: Theme.border'
      strays 'radius: Theme.radius' 'a frame of their own'
      strays 'RectangularShadow {}' 'a frame of their own'
      strays 'Floating {' 'take Card'
      strays 'x: Theme.space.rung.inset' 'read the inset'
      strays 'x: Theme.space.rung.mark' 'Surface.qml:1: x: Theme.space.rung.mark'
      strays 'x: gutter + Theme.level.name.pitch' 'by hand'
      strays 'anchors.leftMargin: Theme.space.rung.apart' 'by hand'
      strays 'rightPadding: Theme.space.rung.items' 'by hand'
      strays 'color: Qt.alpha(Theme.selection, Theme.floating.selection)' "a row's ground"
      # A card, a part's radius, a column, the card's own inset and the inset
      # said in a comment pass.
      rm -rf shell
      mkdir shell
      printf '%s\n' 'Card {' 'radius: Theme.inner' 'x: row.columns.text' 'x: -card.inset' \
        '// once at Theme.space.rung.inset' > shell/Surface.qml
      ${check} shell Surface.qml
      ${check} ${surfaces} ${lib.concatStringsSep " " cards}
      touch $out
    '';

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
      ${lib.getExe pkgs.python3} ${../../tests/launcher-words.py} ${words} ${marks} \
        ${./launcher/words.json}
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
      language = ../keys/language.toml;
    in
    pkgs.runCommandLocal "the-launcher-names-every-action-a-command" { } ''
      ${lib.getExe pkgs.python3} ${../../tests/launcher-commands.py} ${words} ${language}
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
      cp ${../../tests/repository/glossary.py} glossary.py
      cp ${../../tests/repository/names.py} names.py
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
      cp ${../../tests/repository/glossary.py} glossary.py
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
      cp ${./launcher/KeyLayers.js} KeyLayers.js
      cp ${layers} layers.json
      cp ${words} words.json
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml ${qt}/bin/qml key-groups.qml
      cd $TMPDIR/shell
      mkdir -p runtime launcher && chmod 700 runtime
      cp ${../../tests/key-map.qml} shell.qml
      cp ${./Fuzzy.js} Fuzzy.js
      cp ${./launcher/Provider.qml} launcher/Provider.qml
      cp ${./ProgramGlyphs.js} ProgramGlyphs.js
      cp ${./launcher/Asking.js} launcher/Asking.js
      cp ${./launcher/Numbers.js} launcher/Numbers.js
      cp ${./launcher/KeyMap.qml} launcher/KeyMap.qml
      cp ${./launcher/KeyGroups.js} launcher/KeyGroups.js
      cp ${./launcher/KeyLayers.js} launcher/KeyLayers.js
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
      # The key language's commands by their whole names, each with whether
      # it changes things, ends something or starts something, as the
      # shell's Options.commands holds them.
      named =
        lib.filterAttrs (_: about: about ? command)
          (builtins.fromTOML (builtins.readFile ../keys/language.toml)).acts;
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
      cp ${../../tests/catalogue.qml} catalogue.qml
      cp ${./launcher/Catalogue.js} Catalogue.js
      cp ${commands} commands.json
      export QML_XHR_ALLOW_FILE_READ=1
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml catalogue.qml
      touch $out
    '';

  # What `launcher show` prints in a terminal is what the launcher holds:
  # a step names its row, a miss the names nearest it, and a row and its
  # drawer come as the words drawn, as tests/showing.qml holds
  # launcher/Showing.js to it in Qt's own engine.
  the-launcher-shows-a-terminal-what-it-holds =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "the-launcher-shows-a-terminal-what-it-holds" { } ''
      cp ${../../tests/showing.qml} showing.qml
      cp ${./launcher/Showing.js} Showing.js
      cp ${./launcher/Order.js} Order.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml showing.qml
      touch $out
    '';

  # A file in the launcher's chooser of files says its size and when it
  # changed as VKCF3Z set them, and its plate its kind, preview and facts,
  # one kept elsewhere, as in Google Drive, alike from what its place
  # listed, a file nothing has read is its ending's kind and a picture
  # previewed as it is, and a file in the Nix store goes by its own name
  # past its hash, as tests/file-facts.qml holds FileFacts.js to it in Qt's
  # own engine.
  a-file-says-its-size-and-when-it-changed =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "a-file-says-its-size-and-when-it-changed" { } ''
      cp ${../../tests/file-facts.qml} file-facts.qml
      cp ${./FileFacts.js} FileFacts.js
      cp ${./When.js} When.js
      cp ${./launcher/words.json} words.json
      export QML_XHR_ALLOW_FILE_READ=1
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml file-facts.qml
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

  # Recent means one thing in every place it stands: the last nine things,
  # newest first, each once, never what's open, as tests/recent.qml holds
  # launcher/Recent.js to it in Qt's own engine.
  recent-means-one-thing =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "recent-means-one-thing" { } ''
      cp ${../../tests/recent.qml} recent.qml
      cp ${./launcher/Recent.js} Recent.js
      cp ${./launcher/Recents.js} Recents.js
      cp ${./launcher/Order.js} Order.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml recent.qml
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
      cp ${../../tests/place-row.qml} place-row.qml
      cp ${./launcher/PlaceRow.js} PlaceRow.js
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
      cp ${../../tests/rows-place.qml} rows-place.qml
      cp ${./launcher/RowsPlace.js} RowsPlace.js
      cp ${../../tests/lib/Expect.js} Expect.js
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
      cp ${../../tests/glide.qml} glide.qml
      cp ${./Glide.js} Glide.js
      cp ${../../tests/lib/Expect.js} Expect.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml glide.qml
      touch $out
    '';

  # For You's :filter narrows to a kind and a source, by their words, as
  # VKCF3Z ruled it; tests/for-you-filter.qml holds launcher/ForYouFilter.js
  # to it in Qt's own engine.
  for-you-narrows-by-kind-and-source =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "for-you-narrows-by-kind-and-source" { } ''
      cp ${../../tests/for-you-filter.qml} for-you-filter.qml
      cp ${./launcher/ForYouFilter.js} ForYouFilter.js
      cp ${./launcher/Order.js} Order.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml for-you-filter.qml
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
      lint="${lib.getExe pkgs.python3} ${../../tests/every-list-keeps-its-kinds-order.py}"
      $lint ${./launcher} ${../people}
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
      cp ${../../tests/order.qml} order.qml
      cp ${./launcher/Order.js} Order.js
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
      cp ${../../tests/becoming.qml} becoming.qml
      cp ${./launcher/Becoming.js} Becoming.js
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
  # an opening weighs the foot's verbs only once its words or face change, as
  # tests/pacing.qml holds launcher/Pacing.js and launcher/Wanted.js to it in
  # Qt's own engine.
  the-launcher-paces-its-refreshes =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "the-launcher-paces-its-refreshes" { } ''
      cp ${../../tests/pacing.qml} pacing.qml
      cp ${./launcher/Order.js} Order.js
      cp ${./launcher/Pacing.js} Pacing.js
      cp ${./launcher/Wanted.js} Wanted.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml pacing.qml
      touch $out
    '';

  # A timer's span and name are read from what's typed after "timer", its
  # time left told as a clock and its span in its largest units, and timers
  # listed rung first, and what Now's Timers row says, as tests/timing.qml
  # holds launcher/Timing.js to it in Qt's own engine.
  a-timer-is-read-from-what-is-typed =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "a-timer-is-read-from-what-is-typed" { } ''
      cp ${../../tests/timing.qml} timing.qml
      cp ${./launcher/Timing.js} Timing.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml timing.qml
      touch $out
    '';

  # pi, as an agent source, sorts the node's sessions into those running,
  # parked and closed this boot, and says each as the agent source
  # contract's record, leaving out what pi didn't say, as
  # tests/pi-source.qml holds PiSource.js to it in Qt's own engine.
  pi-tells-of-its-sessions-as-an-agent-source =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "pi-tells-of-its-sessions-as-an-agent-source" { } ''
      cp ${../../tests/pi-source.qml} pi-source.qml
      cp ${./PiSource.js} PiSource.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml pi-source.qml
      touch $out
    '';

  # pi's sessions, as many and as large as Joel's, each changing its step
  # with the launcher shut and then showing them, sorted and lived again as
  # Sessions.qml does, start a garbage collection in well under one change
  # in five, as QV4's own report of each collection counts them. Told apart
  # as text they had started one nearly every change (99 in 100), each a
  # sweep that stalls the shell's main thread.
  pi-sorts-without-collecting =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "pi-sorts-without-collecting" { } ''
      cp ${../../tests/pi-source-collects.qml} pi-source-collects.qml
      cp ${./PiSource.js} PiSource.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      export QML_IMPORT_PATH=${qt}/lib/qt-6/qml QT_LOGGING_RULES=qt.qml.gc.allocatorStats=true
      ${shellFolderLint}
      timeout 120 ${qt}/bin/qml pi-source-collects.qml > said 2>&1 || { cat said; exit 1; }
      awk '/triggered by unmanaged heap:/ { n++ } END { print n + 0, "started"; exit n >= 20 }' said
      touch $out
    '';

  # A screen darkens by name where niri powers one output off, docked with
  # its hub powered, and unplugs only as the bigger step; the last one lit
  # waits for the key that asked to come up; an older niri only unplugs, as
  # tests/screens.qml holds launcher/Screens.js to it in Qt's own engine.
  the-launcher-turns-each-screen-off =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "the-launcher-turns-each-screen-off" { } ''
      cp ${../../tests/screens.qml} screens.qml
      cp ${./launcher/Screens.js} Screens.js
      cp ${./launcher/Numbers.js} Numbers.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml screens.qml
      touch $out
    '';

  # A change to a screen niri refuses says on the warning line what was
  # asked, what wasn't done and niri's own words for why, and one to a
  # screen gone says it's unplugged, as tests/screen-answers.qml holds
  # launcher/ScreenAnswers.js to it in Qt's own engine.
  the-launcher-says-what-niri-refuses =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "the-launcher-says-what-niri-refuses" { } ''
      cp ${../../tests/screen-answers.qml} screen-answers.qml
      cp ${./launcher/ScreenAnswers.js} ScreenAnswers.js
      cp ${../../tests/lib/Expect.js} Expect.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml screen-answers.qml
      touch $out
    '';

  # Machines says every machine of the realms this one links, this one
  # first, each in words with how it stands, and its drawer from what
  # moves most to the model's identifier, leaving out what no node said, as
  # tests/machines.qml holds launcher/RealmMachines.js to it in Qt's own engine.
  the-launcher-says-every-machine-of-the-realm =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "the-launcher-says-every-machine-of-the-realm" { } ''
      cp ${../../tests/machines.qml} machines.qml
      cp ${./launcher/RealmMachines.js} RealmMachines.js
      cp ${./launcher/Order.js} Order.js
      cp ${./launcher/Machine.js} Machine.js
      cp ${./launcher/Numbers.js} Numbers.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml machines.qml
      touch $out
    '';

  # A pi session says how it stands and its acts, and its drawer's times
  # are worded, as tests/session-row.qml holds launcher/SessionRow.js and
  # SessionDrawer.js to them in Qt's own engine, and Landed what they
  # landed, kept past their end (Landed.js). How Agents lays sessions out,
  # and what its drawer says, is the agents daemon's, proven in its crate.
  the-launcher-says-how-each-session-stands =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "the-launcher-says-how-each-session-stands" { } ''
      cp ${../../tests/session-row.qml} session-row.qml
      cp ${./launcher/Holders.js} Holders.js
      cp ${./launcher/SessionRow.js} SessionRow.js
      cp ${./launcher/Order.js} Order.js
      cp ${./launcher/Landed.js} Landed.js
      cp ${./launcher/SessionDrawer.js} SessionDrawer.js
      cp ${./Reading.js} Reading.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml session-row.qml
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
      cp ${../../tests/plate.qml} plate.qml
      cp ${./launcher/Plate.js} Plate.js
      cp ${./launcher/Screens.js} Screens.js
      cp ${./launcher/Backlight.js} Backlight.js
      cp ${./launcher/Numbers.js} Numbers.js
      cp ${./launcher/Order.js} Order.js
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
      cp ${../../tests/trials.qml} trials.qml
      cp ${./launcher/Trials.js} Trials.js
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
        cp ${./Trials.qml} Trials.qml
        cp ${./launcher/Trials.js} launcher/Trials.js
        cp ${../../tests/trials-kept.qml} shell.qml
        cp ${../../tests/lib/Expect.js} Expect.js
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
      cp ${../../tests/traces.qml} traces.qml
      cp ${./launcher/Traces.js} Traces.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml traces.qml
      touch $out
    '';

  # Reopen Them All puts each session back on its own screen, found by what
  # the screen is rather than its connector, a gone screen's workspaces
  # after the one in use's and to it once it's back, and each window that
  # reopens itself beside those kept with it, and begins the next session
  # only while the machine queue says there's room, as tests/places.qml
  # holds Places.js to it in Qt's own engine.
  reopening-finds-each-screen =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "reopening-finds-each-screen" { } ''
      cp ${../../tests/places.qml} places.qml
      cp ${./Places.js} Places.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml places.qml
      touch $out
    '';

  # For You weighs what needs Joel, heaviest first and ties as they came,
  # bands what needs him now apart from To Look At and To Try, the
  # heaviest first and ties the oldest first in each, says each row's
  # strongest reason, reads the looks folder's files into
  # what waits and what's history, and gathers what other sources give it,
  # as Passwords' one to-do, weighed beside the rest or done, as
  # tests/for-you.qml holds ForYou.js to it in Qt's own engine.
  for-you-weighs-what-needs-joel =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "for-you-weighs-what-needs-joel" { } ''
      cp ${../../tests/for-you.qml} for-you.qml
      cp ${./launcher/ForYou.js} ForYou.js
      cp ${./launcher/Order.js} Order.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml for-you.qml
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
      cp ${../../tests/rows-kept.qml} rows-kept.qml
      cp ${./launcher/RowsKept.js} RowsKept.js
      cp ${../../tests/lib/Expect.js} Expect.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml rows-kept.qml
      touch $out
    '';

  # Copies and Emoji say the clipboard's history isn't answering only once
  # it's been silent a second, so a restart never blinks their counts, and
  # a paste that never landed for a minute, as tests/pasting.qml holds
  # Pasting.js to it in Qt's own engine.
  the-clipboard-says-once-it-is-silent =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "the-clipboard-says-once-it-is-silent" { } ''
      cp ${../../tests/pasting.qml} pasting.qml
      cp ${./Pasting.js} Pasting.js
      cp ${../../tests/lib/Expect.js} Expect.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml pasting.qml
      touch $out
    '';

  # The machine queue is the harness's file as the node's record holds it,
  # and a write of only when it was written says nothing new, as
  # tests/jobs.qml holds Jobs.js to it in Qt's own engine.
  the-queue-is-heard-from-the-node =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "the-queue-is-heard-from-the-node" { } ''
      cp ${../../tests/jobs.qml} jobs.qml
      cp ${./Jobs.js} Jobs.js
      cp ${../../tests/lib/Expect.js} Expect.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml jobs.qml
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
  # routing tables, resolv.conf and wifid say, and how WARP's link stands from
  # the record warp prints (modules/warp/warp.json), as tests/answers.qml
  # lists them, and leaves alone what isn't one, in Qt's own JavaScript
  # engine.
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
      cp ${./launcher/Moment.js} Moment.js
      cp ${./launcher/Keeping.js} Keeping.js
      cp ${./launcher/Warmth.js} Warmth.js
      cp ${./launcher/Here.js} Here.js
      cp ${./launcher/Colours.js} Colours.js
      cp ${./launcher/Asking.js} Asking.js
      cp ${./launcher/Machine.js} Machine.js
      cp ${./launcher/Heat.js} Heat.js
      cp ${./launcher/Power.js} Power.js
      cp ${./launcher/Seen.js} Seen.js
      cp ${./launcher/Awake.js} Awake.js
      cp ${./launcher/Standing.js} Standing.js
      cp ${./launcher/Traffic.js} Traffic.js
      cp ${./launcher/Interfaces.js} Interfaces.js
      cp ${./launcher/Networkd.js} Networkd.js
      cp ${./launcher/WifiLink.js} WifiLink.js
      cp ${./launcher/Links.js} Links.js
      cp ${./launcher/Sounds.js} Sounds.js
      cp ${./launcher/Cameras.js} Cameras.js
      cp ${./launcher/Firmware.js} Firmware.js
      cp ${./launcher/Warp.js} Warp.js
      cp ${./launcher/WarpSeen.js} WarpSeen.js
      cp ${./launcher/WarpSwitch.js} WarpSwitch.js
      cp ${./launcher/HomeNetwork.js} HomeNetwork.js
      cp ${./launcher/Ago.js} Ago.js
      cp ${./launcher/words.json} words.json
      cp ${../warp/warp.json} warp.json
      cp ${./launcher/Keyboards.js} Keyboards.js
      cp ${./launcher/KeyCounts.js} KeyCounts.js
      cp ${./launcher/Order.js} Order.js
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

  # The launcher's Home Network place says the home network as the eero
  # daemon's record has it: the rows' order, what each row says, and every
  # figure the eero contract places in the internet's, a node's and a
  # device's drawer, as tests/home-network.qml says, in Qt's own engine.
  the-home-network-says-each-figure =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "the-home-network-says-each-figure" { } ''
      mkdir launcher
      cp ${../../tests/home-network.qml} home-network.qml
      cp ${./When.js} When.js
      cp ${./launcher/HomeNetwork.js} launcher/HomeNetwork.js
      cp ${./launcher/HomeNetworkAddresses.js} launcher/HomeNetworkAddresses.js
      cp ${./launcher/HomeNetworkDrawers.js} launcher/HomeNetworkDrawers.js
      cp ${./launcher/HomeNetworkHistory.js} launcher/HomeNetworkHistory.js
      cp ${./launcher/Order.js} launcher/Order.js
      cp ${./launcher/Numbers.js} launcher/Numbers.js
      cp ${./launcher/Interfaces.js} launcher/Interfaces.js
      cp ${./launcher/Machine.js} launcher/Machine.js
      cp ${../eero/home-network.json} home-network.json
      export QML_XHR_ALLOW_FILE_READ=1 TZ=UTC
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml home-network.qml
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

  # A long name slides at one reading pace between soft ends, as
  # tests/reading.qml says, in Qt's own engine.
  a-long-name-reads-at-one-pace =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "a-long-name-reads-at-one-pace" { } ''
      cp ${../../tests/reading.qml} reading.qml
      cp ${./Reading.js} Reading.js
      cp ${./Measures.js} Measures.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml reading.qml
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
  # and every key behind it forever, as
  # modules/quickshell/tests/askpass-waits.sh says. The desktop test restarts
  # a real shell, in minutes; this says it in one.
  the-askpass-outlives-its-shell =
    let
      askpass = (account graphical).home.sessionVariables.SSH_ASKPASS;
    in
    pkgs.runCommandLocal "the-askpass-outlives-its-shell" { } ''
      ${lib.getExe' pkgs.coreutils "timeout"} 300 ${lib.getExe pkgs.bash} \
        ${./tests/askpass-waits.sh} ${askpass} ||
        { echo "the askpass is still waiting, or got something wrong"; exit 1; }
      touch $out
    '';

  # A D-Bus call from the account's shells, as run0 asks for polkit's prompt
  # in, waits past systemd's own 25 seconds for an answer, as
  # modules/quickshell/tests/prompt-outlasts-the-bus.py says, on a session
  # bus of its own.
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
        ${lib.getExe python} ${./tests/prompt-outlasts-the-bus.py} \
        ${lib.escapeShellArg timeout} ${lib.getExe' pkgs.systemd "busctl"}
      touch $out
    '';

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

  # The disk piece hears the guards' status files as tests/disk.qml writes
  # them, through Quickshell itself and the machine daemon over a machine of
  # the check's own, with niri stood in for.
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
      # The piece says sizes and folders as a file does, from the shell's
      # folder above its own, so it's laid in pieces/ as the shell has it,
      # where the lint reads the whole shell.
      mkdir -p runtime/daemons shell/launcher shell/pieces && chmod 700 runtime
      mkdir -p root/run/user/1000/disk-guard root/run/disk-guard
      cd shell
      cp ${./Daemon.qml} Daemon.qml
      cp ${./Watchers.qml} Watchers.qml
      cp ${./FileFacts.js} FileFacts.js
      cp ${./When.js} When.js
      cp ${./Files.qml} Files.qml
      cp ${./pieces/Piece.qml} pieces/Piece.qml
      cp ${./pieces/Disk.qml} pieces/Disk.qml
      cp ${./pieces/Drawers.js} pieces/Drawers.js
      cp ${./Words.qml} Words.qml
      cp ${./launcher/words.json} launcher/words.json
      cp ${../../tests/disk.qml} shell.qml
      cp ${niri} Niri.qml
      cp ${(account graphical).xdg.configFile."quickshell/Theme.qml".source} Theme.qml
      export HOME=$TMPDIR XDG_RUNTIME_DIR=$TMPDIR/runtime LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen
      export MACHINE_ROOT=$TMPDIR/root FONTCONFIG_FILE=${noFonts}
      XDG_RUNTIME_DIR=/run/user/1000 \
        ${lib.getExe machine} "$TMPDIR/runtime/daemons/machine.sock" "$MACHINE_ROOT" &
      daemon=$!
      ${shellFolderLint}
      timeout 60 ${qs} -p shell.qml
      kill "$daemon"
      touch $out
    '';

  # Keeping awake holds the lid only where a shut lid would suspend, on
  # battery or on the charger: a machine with the lid module, whose lid
  # suspends on battery, keeps the lid holder and its shell says so, and
  # one whose lid is ignored everywhere has neither.
  keeping-awake-holds-the-lid-only-where-it-sleeps =
    let
      lidded = bareNixos [
        self.nixosModules.graphical
        self.nixosModules.lid
      ];
      ignored = bareNixos [
        self.nixosModules.graphical
        {
          services.logind.settings.Login = {
            HandleLidSwitch = "ignore";
            HandleLidSwitchExternalPower = "ignore";
          };
        }
      ];
      holder = machine: (account machine).systemd.user.services ? keep-awake-lid;
      says =
        machine:
        lib.hasInfix "lidSleeps: true" (account machine).xdg.configFile."quickshell/Options.qml".text;
    in
    holds "keeping-awake-holds-the-lid-only-where-it-sleeps"
      "the lid's holder and the shell's word should both follow whether a shut lid suspends, on battery or on the charger"
      (holder lidded && says lidded && !holder ignored && !says ignored);

  # The keyboard's backlight is heard when its firmware changes it, over a
  # folder laid out as the kernel's lights are, through Quickshell itself.
  # Kept in Nix because each line lays a light, copies a file Nix gives it
  # beside the cases, or sets what Quickshell runs them with.
  the-keyboard-light-is-heard = pkgs.runCommandLocal "the-keyboard-light-is-heard" { } ''
    mkdir -p runtime/leds/test::kbd_backlight runtime/leds/input0::capslock && chmod 700 runtime
    echo 3 > runtime/leds/test::kbd_backlight/max_brightness
    touch runtime/leds/test::kbd_backlight/brightness_hw_changed
    cp ${./Lights.qml} Lights.qml
    cp ${./Daemon.qml} Daemon.qml
    cp ${../../tests/lights.qml} shell.qml
    export HOME=$TMPDIR XDG_RUNTIME_DIR=$TMPDIR/runtime LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen
    export FONTCONFIG_FILE=${noFonts}
    ${shellFolderLint}
    timeout 60 ${qs} -p shell.qml
    touch $out
  '';

  # The AI spend piece, the tokens piece, the launcher's collection and its
  # budgets' drawers read what spend-watch writes, from a day of a session's turns on two models,
  # and the states tests/spend.qml writes by hand, through Quickshell
  # itself, with niri, do not disturb, the shortcuts and the sessions stood
  # in for; and what its socket tells, from a stand-in serving those
  # records (tests/spend-told.qml), a pass remaking only the rows it changed.
  the-desktop-reads-the-spend-watch =
    let
      watch = pkgs.callPackage ../spend-watch/package.nix { };
      niri = singleton "Niri.qml" ''
        property var spawned: []
        function spawn(command) { spawned = command; }
      '';
      sessions = singleton "Sessions.qml" ''
        property var running: [{
          address: "ADDR1",
          title: "The Launcher's Shape",
          presence: { session: "s-running" }
        }]
        property string steered: ""
        function steer(address, fields) { steered = address + " " + fields.act; }
      '';
      shortcuts = singleton "Shortcuts.qml" ''
        property var actions: [{ opens: "spend", chord: [{ text: "Super" }, { text: "U" }] }]
      '';
    in
    pkgs.runCommandLocal "the-desktop-reads-the-spend-watch" { } ''
      mkdir -p runtime/written runtime/spend-watch runtime/daemons sessions/thing state launcher
      chmod 700 runtime
      ${lib.getExe pkgs.bash} ${./tests/stand-ins/spend-session.sh} sessions/thing/session.jsonl
      cat > config.json <<EOF
      {"sessions": "$PWD/sessions", "runs": "$PWD/runs.db", "registry": "$PWD/registry",
       "quests": "$PWD/quests", "state": "$PWD/state", "runtime": "$PWD/runtime/written",
       "zone": "UTC", "notices": {"enable": false}}
      EOF
      ${lib.getExe watch} --config config.json import
      ${lib.getExe watch} --config config.json show > runtime/written/status.json
      ${lib.getExe watch} --config config.json show collection > runtime/written/collection.json
      ${lib.getExe watch} --config config.json budgets --json > runtime/written/budgets.json
      cp ${./pieces/Piece.qml} Piece.qml
      cp ${./pieces/Spend.qml} Spend.qml
      cp ${./pieces/Tokens.qml} Tokens.qml
      cp ${sessions} Sessions.qml
      cp ${./pieces/StatusFile.qml} StatusFile.qml
      cp ${./Fuzzy.js} Fuzzy.js
      cp ${./launcher/Provider.qml} launcher/Provider.qml
      cp ${./ProgramGlyphs.js} ProgramGlyphs.js
      cp ${./launcher/Asking.js} launcher/Asking.js
      cp ${./launcher/Numbers.js} launcher/Numbers.js
      cp ${./launcher/Spend.qml} launcher/Spend.qml
      cp ${./launcher/SpendDrawer.js} launcher/SpendDrawer.js
      cp ${(account graphical).xdg.configFile."quickshell/Theme.qml".source} Theme.qml
      cp ${./Words.qml} Words.qml
      cp ${./launcher/words.json} launcher/words.json
      cp ${./SpendWatch.qml} SpendWatch.qml
      cp ${./SpendWatch.js} SpendWatch.js
      cp ${./Daemon.qml} Daemon.qml
      cp ${../../tests/spend.qml} shell.qml
      cp ${../../tests/spend-told.qml} told.qml
      cp ${../../tests/lib/Expect.js} Expect.js
      cp ${niri} Niri.qml
      cp ${singleton "Quiet.qml" "property bool sharing: false"} Quiet.qml
      cp ${shortcuts} Shortcuts.qml
      export HOME=$TMPDIR XDG_RUNTIME_DIR=$TMPDIR/runtime LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      timeout 60 ${qs} -p shell.qml
      cp runtime/written/collection.json runtime/written/budgets.json runtime/spend-watch/
      timeout 60 ${qs} -p told.qml
      touch $out
    '';

  # The shell holds a daemon's records over one connection (Daemon.qml),
  # against the real people daemon with two sources stood in for, one
  # asked (daemon-source.sh) and one saying everyone unasked
  # (daemon-roster.sh), started, stopped and started again by
  # tests/daemon.qml; and over one connection, records of two kinds from a
  # daemon holding both, and kinds told as the mycelium node tells them
  # (daemon-kinds.py). The stand-ins are in modules/quickshell/tests/stand-ins.
  the-desktop-holds-a-daemons-records =
    let
      people = pkgs.callPackage ../people/package.nix { };
    in
    pkgs.runCommandLocal "the-desktop-holds-a-daemons-records" { } ''
      mkdir -p runtime/daemons && chmod 700 runtime
      cat > sources.json <<EOF
      {"sources": [{"name": "slack",
        "command": ["${lib.getExe pkgs.bash}", "${./tests/stand-ins/daemon-source.sh}"]},
        {"name": "tuple",
        "command": ["${lib.getExe pkgs.bash}", "${./tests/stand-ins/daemon-roster.sh}"]}]}
      EOF
      cp ${./Daemon.qml} Daemon.qml
      cp ${../../tests/daemon.qml} shell.qml
      export HOME=$TMPDIR XDG_RUNTIME_DIR=$PWD/runtime LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts} PEOPLE=${lib.getExe people} SOURCES=$PWD/sources.json
      export PYTHON=${lib.getExe pkgs.python3} KINDS=${./tests/stand-ins/daemon-kinds.py}
      ${shellFolderLint}
      timeout 90 ${qs} -p shell.qml
      touch $out
    '';

  # What was taken lately reads its folder through the programs daemon,
  # handed a source for it as modules/quickshell/home.nix hands one, and
  # finds screenshots by their words (Taken.qml), asking a stand-in for the
  # screen-text daemon
  # (tests/desktop/stand-in-screen-text.py) over one connection made on the
  # first search and let go at rest, each text once, the last few kept,
  # none for fewer than three letters, a ragged line given as one short
  # one, and finds a few thousand files by name within a frame, as
  # tests/taken.qml holds it to, with what each screenshot says in
  # modules/quickshell/tests/stand-ins/taken-texts.json.
  the-desktop-finds-screenshots-by-their-words =
    let
      standIn = pkgs.writers.writePython3 "stand-in-screen-text" {
        doCheck = false;
      } (builtins.readFile ../../tests/desktop/stand-in-screen-text.py);
    in
    pkgs.runCommandLocal "the-desktop-finds-screenshots-by-their-words" { } ''
      mkdir -p runtime/daemons shots && chmod 700 runtime
      touch shots/2026-10-{29,30,31}\ 09-00-00.png
      substitute ${./tests/stand-ins/taken-texts.json} texts.json --subst-var-by shots "$PWD/shots"
      printf '[{"name": "taken", "folders": ["%s"]}]' "$PWD/shots" > sources.json
      cp ${./Daemon.qml} Daemon.qml
      cp ${./Taken.qml} Taken.qml
      cp ${./Fuzzy.js} Fuzzy.js
      cp ${../../tests/taken.qml} shell.qml
      export HOME=$TMPDIR XDG_RUNTIME_DIR=$PWD/runtime LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts} SHOTS=$PWD/shots
      ${standIn} runtime/daemons/screen-text.sock texts.json &
      ${lib.getExe programs} runtime/daemons/programs.sock sources.json &
      ${shellFolderLint}
      timeout 60 ${qs} -p shell.qml
      touch $out
    '';

  # The launcher's quests find pi's quests in the harness's two lists, open
  # and ended, as the quests daemon makes their rows from a stand-in for
  # the realm's node, and open pi on one or go to its session
  # (tests/quests.qml), with niri and pi's sessions stood in.
  the-launcher-finds-the-quests =
    let
      quests = lib.getExe (pkgs.callPackage ../quests/package.nix { });
      niri = singleton "Niri.qml" ''
        property var spawned: []
        property var acted: null
        function spawn(command) { spawned = command; }
        function act(request) { acted = request; }
      '';
      sessions = singleton "Sessions.qml" ''
        readonly property string home: Quickshell.env("HOME")
        readonly property string nodePath: Quickshell.env("XDG_RUNTIME_DIR")
          + "/mycelium/harbour.sock"
        property var running: []
        property var parked: []
        function isOf(session) { return `window:''${session.window.id}`; }
        function goTo(session) { Niri.act({ FocusWindow: { id: session.window.id } }); }
        property var started: null
        function start(quest, folder, orchestrates) {
          started = orchestrates ? [quest, folder, "orchestrating"] : [quest, folder];
        }
        readonly property string machine: "alpha"
        property var places: []
        property string fits: ""
        readonly property bool choosesMachine: places.length > 1
        property int lookedAround: 0
        function lookAround() { lookedAround += 1; }
        property var startedOn: null
        function startOn(on, quest, folder) { startedOn = [on, quest, folder]; }
      '';
    in
    pkgs.runCommandLocal "the-launcher-finds-the-quests" { } ''
      mkdir launcher
      cp ${(account graphical).xdg.configFile."quickshell/Theme.qml".source} Theme.qml
      cp ${./Fuzzy.js} Fuzzy.js
      cp ${./launcher/Provider.qml} launcher/Provider.qml
      cp ${./ProgramGlyphs.js} ProgramGlyphs.js
      cp ${./launcher/Asking.js} launcher/Asking.js
      cp ${./launcher/Numbers.js} launcher/Numbers.js
      cp ${./launcher/Quests.qml} launcher/Quests.qml
      cp ${./launcher/ReadyRows.qml} launcher/ReadyRows.qml
      cp ${./launcher/ReadyRows.js} launcher/ReadyRows.js
      cp ${./launcher/Ago.js} launcher/Ago.js
      cp ${./launcher/Interfaces.js} launcher/Interfaces.js
      cp ${./launcher/Order.js} launcher/Order.js
      cp ${./launcher/RealmMachines.js} launcher/RealmMachines.js
      cp ${./launcher/Machine.js} launcher/Machine.js
      cp ${./When.js} When.js
      cp ${./Words.qml} Words.qml
      cp ${./launcher/words.json} launcher/words.json
      cp ${../../tests/quests.qml} shell.qml
      cp ${./Daemon.qml} Daemon.qml
      cp ${niri} Niri.qml
      cp ${sessions} Sessions.qml
      export XDG_RUNTIME_DIR=$TMPDIR/runtime LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen
      # At home here, so a folder here reads from ~.
      export HOME=$PWD
      mkdir -m 700 $XDG_RUNTIME_DIR $XDG_RUNTIME_DIR/mycelium $XDG_RUNTIME_DIR/daemons
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      # The daemon follows the node the shell stands in, trying again
      # until it listens.
      ${quests} --vocabulary ${./launcher/words.json} \
        --node $XDG_RUNTIME_DIR/mycelium/harbour.sock \
        --socket $XDG_RUNTIME_DIR/daemons/quests.sock &
      timeout 60 ${qs} -p shell.qml
      touch $out
    '';

  # The talks Agents shows (tests/talks.qml), heard from a stand-in for the
  # realm's node serving each talk as a record of its lines: by its
  # session's address, newest first, moved on by each line told, one made
  # since heard and one let go gone, with no file of the talks read here.
  the-launcher-hears-talks-from-the-node =
    let
      niri = singleton "Niri.qml" ''
        property int revision: 0
        function spawn(command) {}
        function act(request) {}
      '';
      sessions = singleton "Sessions.qml" ''
        readonly property string home: Quickshell.env("HOME")
        readonly property string state: Quickshell.env("PWD") + "/state"
        readonly property string nodePath: Quickshell.env("XDG_RUNTIME_DIR")
          + "/mycelium/harbour.sock"
        property var running: [{ address: "A003" }, { address: "A117" }]
        property var parked: []
        property var present: ({})
        function asks(session) { return false; }
        function resume(id, quest, folder, gone) {}
        function start(quest, folder) {}
        function lookForFolders(cwds) {}
        function folderGone(cwd) { return false; }
      '';
      waiting = singleton "WaitingOnYou.qml" "readonly property var standing: ({})";
      jobs = singleton "Jobs.qml" "readonly property var jobs: []";
    in
    pkgs.runCommandLocal "the-launcher-hears-talks-from-the-node" { } ''
      mkdir -p launcher state/quest-workflow
      echo '{"orchestrators":[{"address":"A003","quest":"QEST-A","title":"A"}]}' \
        > state/quest-workflow/orchestrators.json
      echo '{"quests":[]}' > state/quest-workflow/quests.json
      cp ${(account graphical).xdg.configFile."quickshell/Theme.qml".source} Theme.qml
      cp ${./Fuzzy.js} Fuzzy.js
      cp ${./launcher/Provider.qml} launcher/Provider.qml
      cp ${./ProgramGlyphs.js} ProgramGlyphs.js
      cp ${./launcher/Asking.js} launcher/Asking.js
      cp ${./launcher/Numbers.js} launcher/Numbers.js
      cp ${./launcher/Orchestrators.qml} launcher/Orchestrators.qml
      cp ${./launcher/SessionRow.js} launcher/SessionRow.js
      cp ${./launcher/Order.js} launcher/Order.js
      cp ${./Words.qml} Words.qml
      cp ${./launcher/words.json} launcher/words.json
      cp ${../../tests/talks.qml} shell.qml
      cp ${./Daemon.qml} Daemon.qml
      cp ${./AgentSources.qml} AgentSources.qml
      cp ${./PiSource.qml} PiSource.qml
      cp ${./PiSource.js} PiSource.js
      cp ${niri} Niri.qml
      cp ${sessions} Sessions.qml
      cp ${waiting} WaitingOnYou.qml
      cp ${jobs} Jobs.qml
      export HOME=$TMPDIR XDG_RUNTIME_DIR=$TMPDIR/runtime LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen
      mkdir -m 700 $XDG_RUNTIME_DIR $XDG_RUNTIME_DIR/mycelium
      # pi's source finds the node by the realm the node keeps.
      mkdir -p $HOME/.local/state/mycelium && echo harbour > $HOME/.local/state/mycelium/realm
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      timeout 60 ${qs} -p shell.qml
      touch $out
    '';

  # Now's own rows, last: the time, the date, the location, the weather
  # read as the launcher opens and the next meeting, whose Return is the
  # meeting's own (tests/now-moment.qml), with the margin's pieces, the
  # agents' waiting list, the weather and the calendar stood in. The
  # location is Location.qml's own, reading whereabouts itself over its
  # socket: here is the one it kept from before it started, written here,
  # GeoClue isn't there to say another, and the clock's zone is a link
  # that names another than the place's. For You's looks are given as
  # their files' texts, kept only while each reads the same, and what's
  # on as pieces whose rows are kept while each reads the same. For You's
  # Desktop holds the notices' own rows (launcher/PastNotices.qml), each
  # kept while its notice is the same. Lately's rows are kept as well, so
  # a notice arriving makes only Desktop's row again, and a session's move
  # that leaves what For You reads of it the same makes none.
  the-launcher-shows-the-moment-in-now = nowShown {
    name = "the-launcher-shows-the-moment-in-now";
    test = ../../tests/now-moment.qml;
  };

  # An opening into a place that shows none of For You's rows places no
  # time or date, which makes Time's and Today's rows, until Now's rows are
  # wanted, and an opening onto them places it once (tests/now-retime.qml).
  the-launcher-places-the-time-only-when-now-is-wanted = nowShown {
    name = "the-launcher-places-the-time-only-when-now-is-wanted";
    test = ../../tests/now-retime.qml;
  };

  # A look Joel acts on whose file can't be written, its folder a file
  # here, is put back as it read and said in the journal, rather than shown
  # done until it comes back at the next read (tests/now-look-unwritten.qml).
  the-launcher-puts-back-a-look-it-couldnt-write = nowShown {
    name = "the-launcher-puts-back-a-look-it-couldnt-write";
    test = ../../tests/now-look-unwritten.qml;
    before = ''
      mkdir -p $HOME/.local/state/pi/agentic-harness.pi
      touch $HOME/.local/state/pi/agentic-harness.pi/looks
    '';
  };

  # The launcher's weather answers now, its parts and the rain by the hour
  # from one forecast, and the rain's own words answer the next rain
  # (tests/weather.qml), with what Open-Meteo would say kept beforehand, so
  # nothing leaves the sandbox; and here is where the laptop is, as each
  # case gives Location's here, ahead of the place set and the time zone.
  the-launcher-answers-the-rain = pkgs.runCommandLocal "the-launcher-answers-the-rain" { } ''
    mkdir launcher
    cp ${./Fuzzy.js} Fuzzy.js
    cp ${./launcher/Provider.qml} launcher/Provider.qml
    cp ${./ProgramGlyphs.js} ProgramGlyphs.js
    cp ${./launcher/Asking.js} launcher/Asking.js
    cp ${./launcher/Numbers.js} launcher/Numbers.js
    cp ${./launcher/Moment.js} launcher/Moment.js
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
        property var running: []
        function tell(words, folder, program) {}
        function talk(address, words) { return true; }
      '';
    in
    pkgs.runCommandLocal "the-launcher-asks-a-fresh-pi" { } ''
      mkdir -p launcher runtime && chmod 700 runtime
      cp ${(account graphical).xdg.configFile."quickshell/Theme.qml".source} Theme.qml
      cp ${./Fuzzy.js} Fuzzy.js
      cp ${./launcher/Provider.qml} launcher/Provider.qml
      cp ${./ProgramGlyphs.js} ProgramGlyphs.js
      cp ${./launcher/Asking.js} launcher/Asking.js
      cp ${./launcher/Numbers.js} launcher/Numbers.js
      cp ${./launcher/Ask.qml} launcher/Ask.qml
      cp ${./launcher/SessionRow.js} launcher/SessionRow.js
      cp ${./launcher/Order.js} launcher/Order.js
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
  # it writes one, the test writing it in turn and the machine daemon
  # telling it over a machine of the check's own.
  the-launcher-says-why-the-desktop-lags =
    pkgs.runCommandLocal "the-launcher-says-why-the-desktop-lags" { }
      ''
        mkdir -p launcher runtime/daemons root/run/user/1000 && chmod 700 runtime
        cp ${(account graphical).xdg.configFile."quickshell/Theme.qml".source} Theme.qml
        cp ${./Daemon.qml} Daemon.qml
        cp ${./Watchers.qml} Watchers.qml
        cp ${./Fuzzy.js} Fuzzy.js
        cp ${./launcher/Provider.qml} launcher/Provider.qml
        cp ${./ProgramGlyphs.js} ProgramGlyphs.js
        cp ${./launcher/Asking.js} launcher/Asking.js
        cp ${./launcher/Numbers.js} launcher/Numbers.js
        cp ${./launcher/Waits.qml} launcher/Waits.qml
        cp ${./Words.qml} Words.qml
        cp ${./launcher/words.json} launcher/words.json
        cp ${../../tests/waits.qml} shell.qml
        export HOME=$TMPDIR XDG_RUNTIME_DIR=$PWD/runtime LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen
        export MACHINE_ROOT=$PWD/root FONTCONFIG_FILE=${noFonts}
        XDG_RUNTIME_DIR=/run/user/1000 \
          ${lib.getExe machine} "$PWD/runtime/daemons/machine.sock" "$MACHINE_ROOT" &
        daemon=$!
        ${shellFolderLint}
        timeout 60 ${qs} -p shell.qml
        kill "$daemon"
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
        cp ${./Fuzzy.js} Fuzzy.js
        cp ${./launcher/Provider.qml} launcher/Provider.qml
        cp ${./ProgramGlyphs.js} ProgramGlyphs.js
        cp ${./launcher/Asking.js} launcher/Asking.js
        cp ${./launcher/Numbers.js} launcher/Numbers.js
        cp ${./launcher/Zones.js} launcher/Zones.js
        cp ${./launcher/Ago.js} launcher/Ago.js
        cp ${./launcher/Addresses.js} launcher/Addresses.js
        cp ${./launcher/Calculator.qml} launcher/Calculator.qml
        cp ${./launcher/Clocks.qml} launcher/Clocks.qml
        cp ${./launcher/Keeping.js} launcher/Keeping.js
        cp ${./launcher/Today.qml} launcher/Today.qml
        cp ${./launcher/Moment.js} launcher/Moment.js
        cp ${./launcher/Waits.qml} launcher/Waits.qml
        cp ${./launcher/Vendors.qml} launcher/Vendors.qml
        cp ${./launcher/Interfaces.js} launcher/Interfaces.js
        cp ${./launcher/Networkd.js} launcher/Networkd.js
        cp ${./launcher/Subnets.js} launcher/Subnets.js
        cp ${./launcher/Networks.qml} launcher/Networks.qml
        cp ${./launcher/Order.js} launcher/Order.js
        cp ${./launcher/Characters.js} launcher/Characters.js
        cp ${./launcher/Characters.qml} launcher/Characters.qml
        cp ${./Daemon.qml} Daemon.qml
        cp ${import ./launcher/names.nix { inherit lib pkgs; }} names.txt
        cp ${(account graphical).xdg.configFile."quickshell/Theme.qml".source} Theme.qml
        cp ${./Words.qml} Words.qml
        cp ${./launcher/words.json} launcher/words.json
        cp ${import ./launcher/zones.nix { inherit lib pkgs; }} zones.jsonl
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
          property var reading: (${builtins.readFile ../../tests/timekeeping-reading.json})
        ''} Timekeeping.qml
        cp ${singleton "Calendars.qml" ''
          property string asked: ""
          property bool refused: false
          // A day's reading as timekeeping gives it, said of today, whichever
          // day the check runs on.
          property var reading: Object.assign((${builtins.readFile ./timekeeping/calendars-reading.json}),
            { day: Qt.formatDate(new Date(), "yyyy-MM-dd") })
          function ask(day) { asked = day; }
        ''} Calendars.qml
        cp ${../../tests/part-names.qml} shell.qml
        export HOME=$TMPDIR XDG_RUNTIME_DIR=$TMPDIR/runtime LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen
        export TZ=America/Toronto TZDIR=${pkgs.tzdata}/share/zoneinfo
        export FONTCONFIG_FILE=${noFonts}
        ${shellFolderLint}
        timeout 60 ${qs} -p shell.qml
        touch $out
      '';

  # The launcher's Machine answers the processors, the memory and the
  # uptime from the machine daemon, run over the sandbox's own /proc,
  # without making their parts, the memory's and the uptime's from the
  # rows it serves ready in the launcher's words, and asks it for the
  # programs only once a part saying them is looked at (tests/machine.qml),
  # the queue, niri and the telemetry stood in for.
  the-machine-answers-from-its-daemon =
    pkgs.runCommandLocal "the-machine-answers-from-its-daemon" { }
      ''
        mkdir -p launcher runtime/daemons && chmod -R 700 runtime
        cp ${./Daemon.qml} Daemon.qml
        cp ${./Fuzzy.js} Fuzzy.js
        cp ${./launcher/Provider.qml} launcher/Provider.qml
        cp ${./ProgramGlyphs.js} ProgramGlyphs.js
        cp ${./launcher/Asking.js} launcher/Asking.js
        cp ${./launcher/Numbers.js} launcher/Numbers.js
        cp ${./launcher/Heat.js} launcher/Heat.js
        cp ${./launcher/Interfaces.js} launcher/Interfaces.js
        cp ${./launcher/Machine.js} launcher/Machine.js
        cp ${./launcher/Machine.qml} launcher/Machine.qml
        cp ${./launcher/Placed.js} launcher/Placed.js
        cp ${./When.js} When.js
        cp ${./launcher/Order.js} launcher/Order.js
        cp ${./launcher/HomeNetwork.js} launcher/HomeNetwork.js
        cp ${./launcher/Holders.js} launcher/Holders.js
        cp ${./launcher/SessionRow.js} launcher/SessionRow.js
        cp ${(account graphical).xdg.configFile."quickshell/Theme.qml".source} Theme.qml
        cp ${./Words.qml} Words.qml
        cp ${./launcher/words.json} launcher/words.json
        cp ${singleton "Jobs.qml" "property var said: null"} Jobs.qml
        cp ${singleton "Niri.qml" "function act(action) {}"} Niri.qml
        cp ${singleton "Facts.qml" "property var all: []"} Facts.qml
        cp ${../../tests/machine.qml} shell.qml
        export HOME=$TMPDIR XDG_RUNTIME_DIR=$TMPDIR/runtime LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen
        export FONTCONFIG_FILE=${noFonts}
        ${lib.getExe machine} "$XDG_RUNTIME_DIR/daemons/machine.sock" \
          --words ${./launcher/words.json} &
        daemon=$!
        ${shellFolderLint}
        timeout 60 ${qs} -p shell.qml
        kill "$daemon"
        touch $out
      '';

  # The launcher's Heat makes again only the chips the machine daemon
  # tells anew and only the rows whose part reads differently
  # (tests/heat.qml), over a stand-in for the daemon.
  heat-remakes-only-what-moved =
    # Kept in Nix because each line copies a file Nix gives it beside the
    # cases, or sets what Quickshell runs them with.
    pkgs.runCommandLocal "heat-remakes-only-what-moved" { } ''
      mkdir -p launcher runtime/daemons && chmod -R 700 runtime
      cp ${../../tests/lib/Expect.js} Expect.js
      cp ${../../tests/lib/Watch.js} Watch.js
      cp ${./Daemon.qml} Daemon.qml
      cp ${./Fuzzy.js} Fuzzy.js
      cp ${./PiSource.js} PiSource.js
      cp ${./ProgramGlyphs.js} ProgramGlyphs.js
      cp ${./launcher/Provider.qml} launcher/Provider.qml
      cp ${./launcher/Heat.qml} launcher/Heat.qml
      cp ${./launcher/Asking.js} launcher/Asking.js
      cp ${./launcher/Numbers.js} launcher/Numbers.js
      cp ${./launcher/Heat.js} launcher/Heat.js
      cp ${./launcher/Machine.js} launcher/Machine.js
      cp ${./launcher/Order.js} launcher/Order.js
      cp ${./launcher/Seen.js} launcher/Seen.js
      cp ${(account graphical).xdg.configFile."quickshell/Theme.qml".source} Theme.qml
      cp ${./Words.qml} Words.qml
      cp ${./launcher/words.json} launcher/words.json
      cp ${singleton "ProcessorTelemetry.qml" ''
        property int watching: 0
        property var reading: null
        property bool refused: false
        property bool missing: false
      ''} ProcessorTelemetry.qml
      cp ${../../tests/heat.qml} shell.qml
      export HOME=$TMPDIR XDG_RUNTIME_DIR=$TMPDIR/runtime LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      timeout 60 ${qs} -p shell.qml
      touch $out
    '';

  # The launcher's Power makes again only the Charger's rows or the
  # battery's, as the supplies they're made from change (tests/power.qml),
  # over a stand-in for the machine daemon.
  power-remakes-only-what-moved =
    # Kept in Nix because each line copies a file Nix gives it beside the
    # cases, or sets what Quickshell runs them with.
    pkgs.runCommandLocal "power-remakes-only-what-moved" { } ''
      mkdir -p launcher runtime/daemons && chmod -R 700 runtime
      cp ${../../tests/lib/Expect.js} Expect.js
      cp ${../../tests/lib/Watch.js} Watch.js
      cp ${./Daemon.qml} Daemon.qml
      cp ${./Fuzzy.js} Fuzzy.js
      cp ${./PiSource.js} PiSource.js
      cp ${./ProgramGlyphs.js} ProgramGlyphs.js
      cp ${./launcher/Provider.qml} launcher/Provider.qml
      cp ${./launcher/Power.qml} launcher/Power.qml
      cp ${./launcher/Asking.js} launcher/Asking.js
      cp ${./launcher/Numbers.js} launcher/Numbers.js
      cp ${./launcher/Power.js} launcher/Power.js
      cp ${./launcher/Interfaces.js} launcher/Interfaces.js
      cp ${./launcher/Machine.js} launcher/Machine.js
      cp ${./launcher/Order.js} launcher/Order.js
      cp ${./launcher/Seen.js} launcher/Seen.js
      cp ${(account graphical).xdg.configFile."quickshell/Theme.qml".source} Theme.qml
      cp ${./Words.qml} Words.qml
      cp ${./launcher/words.json} launcher/words.json
      cp ${../../tests/power.qml} shell.qml
      export HOME=$TMPDIR XDG_RUNTIME_DIR=$TMPDIR/runtime LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen
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
      cp ${./ProgramGlyphs.js} ProgramGlyphs.js
      cp ${./launcher/Asking.js} launcher/Asking.js
      cp ${./launcher/Numbers.js} launcher/Numbers.js
      cp ${./launcher/Sounds.js} launcher/Sounds.js
      cp ${./launcher/Targets.js} launcher/Targets.js
      cp ${./launcher/Plate.js} launcher/Plate.js
      cp ${./launcher/Sound.qml} launcher/Sound.qml
      cp ${./launcher/Order.js} launcher/Order.js
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
        runtimeInputs = [
          pkgs.coreutils
          pkgs.util-linux
        ];
        text = ''
          state=$XDG_RUNTIME_DIR/litra-state
          feed=$XDG_RUNTIME_DIR/litra-feed
          # Each change waits its turn, as litra's own does under a lock on
          # the light's node (modules/litra/litra.rs): the shell sets
          # brightness and warmth at once, and two changes read at the same
          # time would each write back the other's field as it was.
          if [ "$1" != watch ]; then
            exec 9> "$XDG_RUNTIME_DIR/litra-lock"
            flock 9
          fi
          [ -e "$state" ] || echo "true 40 4000" > "$state"
          read -r on percent kelvin < "$state"
          # A light as litra says it: its lumens are a Glow's, 20 to 250
          # across its share, and it names itself as its HID device does.
          said() {
            printf '[{"node":"/dev/hidraw1","on":%s,"percent":%s,"lumens":%s,"kelvin":%s,%s}]\n' \
              "$on" "$percent" "$((20 + 230 * percent / 100))" "$kelvin" \
              '"dimmest":20,"brightest":250,"name":"Litra Glow stand-in","serial":"stand-in"' \
              >> "$feed"
          }
          case "$1" in
            watch)
              said
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
          said
        '';
      };
    in
    pkgs.runCommandLocal "the-launcher-finds-the-desk-light" { } ''
      mkdir launcher
      cp ${(account graphical).xdg.configFile."quickshell/Theme.qml".source} Theme.qml
      cp ${./Fuzzy.js} Fuzzy.js
      cp ${./DeskLight.qml} DeskLight.qml
      cp ${./launcher/Provider.qml} launcher/Provider.qml
      cp ${./ProgramGlyphs.js} ProgramGlyphs.js
      cp ${./launcher/Asking.js} launcher/Asking.js
      cp ${./launcher/Numbers.js} launcher/Numbers.js
      cp ${./launcher/Light.qml} launcher/Light.qml
      cp ${./launcher/Power.js} launcher/Power.js
      cp ${./launcher/Order.js} launcher/Order.js
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
      cp ${./Fuzzy.js} Fuzzy.js
      cp ${./launcher/Provider.qml} launcher/Provider.qml
      cp ${./ProgramGlyphs.js} ProgramGlyphs.js
      cp ${./launcher/Asking.js} launcher/Asking.js
      cp ${./launcher/Numbers.js} launcher/Numbers.js
      cp ${./launcher/Applications.qml} launcher/Applications.qml
      cp ${./launcher/HeldWindows.js} launcher/HeldWindows.js
      cp ${./launcher/Media.js} launcher/Media.js
      cp ${./launcher/Program.js} launcher/Program.js
      cp ${./FileFacts.js} FileFacts.js
      cp ${./When.js} When.js
      cp ${./Daemon.qml} Daemon.qml
      cp ${./Music.js} Music.js
      cp ${./launcher/Order.js} launcher/Order.js
      cp ${./launcher/Recents.qml} launcher/Recents.qml
      cp ${./launcher/Recents.js} launcher/Recents.js
      cp ${./launcher/Recent.js} launcher/Recent.js
      cp ${./launcher/Captured.js} launcher/Captured.js
      cp ${./launcher/PastNotices.qml} launcher/PastNotices.qml
      cp ${./launcher/Notice.js} launcher/Notice.js
      cp ${./NoticeBody.js} NoticeBody.js
      cp ${./launcher/Ago.js} launcher/Ago.js
      cp ${./Words.qml} Words.qml
      cp ${./Files.qml} Files.qml
      cp ${./launcher/words.json} launcher/words.json
      cp ${../../tests/program-controls.qml} shell.qml
      cp ${../../tests/lib/HeardWindows.js} HeardWindows.js
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

  # The launcher chooses the files a program opens or saves, as open-with
  # asks it (tests/file-chooser.qml): in folders made here, with the files
  # opened lately listed by a stand-in for recent-files, the desktop's
  # kinds of file by a few of shared-mime-info's lines, a place kept
  # elsewhere by a stand-in for Google's Drive, file-preview by one that
  # notes which process each thing was asked of, and niri by one whose
  # window with the keys is a text editor's, the program that asks.
  the-launcher-chooses-files-as-asked =
    let
      niri = singleton "Niri.qml" ''
        property var windows: ({ "7": { id: 7, app_id: "org.gnome.TextEditor" } })
        property var current: 7
      '';
      # A stand-in for file-preview, whether kept for the ask (serve) or
      # started for each thing asked: it says there's nothing to show and
      # notes each thing asked beside the process it was asked of.
      previewer = pkgs.writeShellScript "file-preview" ''
        answer() {
          printf '%s %s\n' "$$" "$1" >> "$HOME/previewed"
          case $1 in
            show\ *) printf '{"error": "a stand-in"}\n' ;;
            *) printf '{}\n' ;;
          esac
        }
        if [ "$1" = serve ]; then
          while IFS= read -r line; do answer "$line"; done
        else
          answer "$*"
        fi
      '';
    in
    pkgs.runCommandLocal "the-launcher-chooses-files-as-asked" { } ''
      mkdir -p launcher kinds
      cp ${niri} Niri.qml
      cp ${(account graphical).xdg.configFile."quickshell/Theme.qml".source} Theme.qml
      cp ${./Fuzzy.js} Fuzzy.js
      cp ${./launcher/Provider.qml} launcher/Provider.qml
      cp ${./ProgramGlyphs.js} ProgramGlyphs.js
      cp ${./launcher/Asking.js} launcher/Asking.js
      cp ${./launcher/Numbers.js} launcher/Numbers.js
      cp ${../open-with/FileChooser.qml} launcher/FileChooser.qml
      cp ${./launcher/Recents.js} launcher/Recents.js
      cp ${./Words.qml} Words.qml
      cp ${./Files.qml} Files.qml
      cp ${./FileFacts.js} FileFacts.js
      cp ${./When.js} When.js
      cp ${./launcher/words.json} launcher/words.json
      cp ${../../tests/file-chooser.qml} shell.qml
      export HOME=$TMPDIR XDG_RUNTIME_DIR=$TMPDIR/runtime LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen
      mkdir -m 700 $XDG_RUNTIME_DIR
      mkdir $XDG_RUNTIME_DIR/open-with
      export FONTCONFIG_FILE=${noFonts}
      # A folder of notes, two hidden things in it, and a file opened lately
      # elsewhere beside one of its own; notes.txt is the largest, then b.png.
      # Two folders deep elsewhere go from one to the other on ]b. The list
      # opened lately is as recent-files prints it, its time last, empty
      # where it's not known.
      mkdir -p $HOME/notes/sub $HOME/notes/.secret $HOME/elsewhere/deep/one $HOME/elsewhere/deep/two
      touch $HOME/notes/a.md $HOME/notes/.hidden.txt
      printf 'twelve bytes' > $HOME/notes/notes.txt
      printf 'png' > $HOME/notes/b.png
      touch $HOME/elsewhere/old.md
      printf 'Notes\t%s\t1700000000\nNotes\t%s\t\n' "$HOME/elsewhere/old.md" "$HOME/notes/a.md" \
        > $HOME/recent
      export RECENT_LIST=${pkgs.writeShellScript "recent-list" ''cat "$HOME/recent"''}
      printf '50:text/markdown:*.md\n50:image/png:*.png\n50:text/plain:*.txt\n' > kinds/globs2
      printf 'text/markdown text/plain\n' > kinds/subclasses
      export KINDS=$PWD/kinds PREVIEWER=${previewer}
      ${shellFolderLint}
      timeout 60 ${qs} -p shell.qml
      asked_of() { grep -m1 " show $1$" $HOME/previewed | cut -d' ' -f1; }
      a=$(asked_of $HOME/notes/a.md)
      if [ -z "$a" ] || [ "$(asked_of $HOME/elsewhere/old.md)" != "$a" ]; then
        echo "the files selected in one ask weren't asked of one file-preview:"
        cat $HOME/previewed
        exit 1
      fi
      [ -d $HOME/staged/plans ] || { echo "saving into a place kept elsewhere made no folder"; exit 1; }
      from=$HOME/staged/plans/.new.md.from
      printf org.gnome.TextEditor | cmp -s - $from || { echo "a save staged elsewhere doesn't say beside it which program asked: $(cat $from 2>&1)"; exit 1; }
      touch $out
    '';

  # Every program an account lists in the launcher, each desktop entry it
  # shows, wears a mark of its own in ProgramGlyphs.js's table, its logo or
  # the theme's mark for it, never the program mark the rest share, and
  # each theme mark it names is one the theme has. Each program a notice's
  # sender is named as (Notices.qml's senders) is one the table knows, so
  # its notices wear the mark its row does.
  every-program-wears-its-own-mark =
    let
      home = account graphical;
      marks = pkgs.writeText "marks" (
        lib.concatMapStrings (name: "${name}\n") (lib.attrNames home.jitsusama.theme.marks)
      );
    in
    pkgs.runCommandLocal "every-program-wears-its-own-mark" { } ''
      sed -n '/^const table = {/,/^};/p' ${./ProgramGlyphs.js} > table
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
      grep -o 'program: "[^"]*"' ${./Notices.qml} | cut -d '"' -f 2 > senders
      [ -s senders ] || { echo "Notices.qml names no sender's program"; exit 1; }
      while read -r program; do
        grep -q -x -F "$program" known \
          || { echo "A notice from $program would wear the bell: no mark is known for it"; exit 1; }
      done < senders
      touch $out
    '';

  # The launcher's runs find pi's fleets and review rounds in their records
  # as the realm's mycelium node serves them, and go to the session that
  # started one or read a participant's answer (tests/runs.qml), with niri,
  # pi's sessions and the screen's sharing stood in, and no node, so the
  # case gives the records itself.
  the-launcher-finds-the-runs =
    let
      niri = singleton "Niri.qml" ''
        property var spawned: []
        property var acted: null
        function spawn(command) { spawned = command; }
        function act(request) { acted = request; }
      '';
      sessions = singleton "Sessions.qml" ''
        readonly property string nodePath: ""
        property var running: []
        property var parked: []
        function goTo(session) { Niri.act({ FocusWindow: { id: session.window.id } }); }
      '';
    in
    pkgs.runCommandLocal "the-launcher-finds-the-runs" { } ''
      mkdir launcher
      cp ${./Daemon.qml} Daemon.qml
      cp ${(account graphical).xdg.configFile."quickshell/Theme.qml".source} Theme.qml
      cp ${./Fuzzy.js} Fuzzy.js
      cp ${./launcher/Provider.qml} launcher/Provider.qml
      cp ${./ProgramGlyphs.js} ProgramGlyphs.js
      cp ${./launcher/Asking.js} launcher/Asking.js
      cp ${./launcher/Numbers.js} launcher/Numbers.js
      cp ${./launcher/Runs.qml} launcher/Runs.qml
      cp ${./PiSource.js} PiSource.js
      cp ${./launcher/Order.js} launcher/Order.js
      cp ${./Words.qml} Words.qml
      cp ${./launcher/words.json} launcher/words.json
      cp ${../../tests/runs.qml} shell.qml
      cp ${niri} Niri.qml
      cp ${sessions} Sessions.qml
      cp ${singleton "AgentSources.qml" "readonly property var runs: []"} AgentSources.qml
      cp ${singleton "Quiet.qml" "property bool sharing: false"} Quiet.qml
      export HOME=$TMPDIR XDG_RUNTIME_DIR=$TMPDIR/runtime LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen
      mkdir -m 700 $XDG_RUNTIME_DIR
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      timeout 60 ${qs} -p shell.qml
      touch $out
    '';

  # The launcher's windows list the one Joel's in as This Window, then the
  # rest the one used last first, and when the focus moves make again only
  # the rows of the windows it moved between, as Capture's do
  # (tests/windows.qml), with what the windows daemon tells given as its
  # lines, and niri, pi's sessions, the recorder, the shortcuts, what was
  # taken and file-preview stood in.
  the-launcher-remakes-only-the-windows-that-changed =
    let
      # file-preview stood in, saying which process answered, and one that
      # stops before answering, each kept by a programs daemon of its own.
      previewer = pkgs.writeShellScript "file-preview" (builtins.readFile ../../tests/file-preview.sh);
      silent = pkgs.writeShellScript "file-preview" "exec ${previewer} silent";
      programs = pkgs.callPackage ../programs/package.nix { };
      sources = pkgs.writeText "sources.json" "[]";
      # What a window's verbs read of niri as they're asked.
      niri = singleton "Niri.qml" ''
        property var windows: ({
          "11": { id: 11, app_id: "tester", title: "Logbook", workspace_id: 1, is_focused: true,
            focus_timestamp: { secs: 200, nanos: 0 }, layout: { pos_in_scrolling_layout: [1, 1] } },
          "12": { id: 12, app_id: "tester", title: "Draft", workspace_id: 2, is_focused: false,
            focus_timestamp: { secs: 100, nanos: 0 }, layout: { pos_in_scrolling_layout: [1, 1] } },
          "13": { id: 13, app_id: "tester", title: "Notes", workspace_id: 1, is_focused: false,
            focus_timestamp: { secs: 50, nanos: 0 }, layout: { pos_in_scrolling_layout: [2, 1] } }
        })
        property var workspaces: ({
          "1": { id: 1, idx: 1, is_focused: true, active_window_id: 11 },
          "2": { id: 2, idx: 2, is_focused: false, active_window_id: 12 }
        })
        property var focused: 11
        property int revision: 0
        readonly property var current: { void revision; return focused; }
        function workspace(id) {
          return id === undefined ? Object.values(workspaces).find(one => one.is_focused) ?? null
            : workspaces[id] ?? null;
        }
        function act(action) {}
      '';
      sessions = singleton "Sessions.qml" ''
        property string waitingWindows: ""
        function aboutOf(id) { return ""; }
      '';
      recorder = singleton "Recorder.qml" ''
        property bool recording: false
        property string what: ""
      '';
      shortcuts = singleton "Shortcuts.qml" ''
        property var actions: []
        property var layers: []
      '';
      # Nothing taken, so Capture's Recent holds no file.
      taken = singleton "Taken.qml" ''
        property var files: []
        function want(folder) {}
        function read() {}
      '';
      files = singleton "Files.qml" "function file(path, thing, acts) { return null; }";
    in
    pkgs.runCommandLocal "the-launcher-remakes-only-the-windows-that-changed" { } ''
      mkdir launcher
      cp ${(account graphical).xdg.configFile."quickshell/Theme.qml".source} Theme.qml
      cp ${./Fuzzy.js} Fuzzy.js
      cp ${./launcher/Provider.qml} launcher/Provider.qml
      cp ${./ProgramGlyphs.js} ProgramGlyphs.js
      cp ${./launcher/Asking.js} launcher/Asking.js
      cp ${./launcher/Numbers.js} launcher/Numbers.js
      cp ${./launcher/KeyLayers.js} launcher/KeyLayers.js
      cp ${./launcher/Windows.qml} launcher/Windows.qml
      cp ${./launcher/HeldWindows.js} launcher/HeldWindows.js
      cp ${./Daemon.qml} Daemon.qml
      cp ${../../tests/lib/Expect.js} Expect.js
      cp ${./launcher/Capture.qml} launcher/Capture.qml
      cp ${./launcher/Previewer.qml} launcher/Previewer.qml
      cp ${./launcher/Captured.js} launcher/Captured.js
      cp ${./launcher/Recent.js} launcher/Recent.js
      cp ${./launcher/Order.js} launcher/Order.js
      cp ${./FileFacts.js} FileFacts.js
      cp ${./When.js} When.js
      cp ${./Words.qml} Words.qml
      cp ${./launcher/words.json} launcher/words.json
      cp ${../../tests/windows.qml} shell.qml
      cp ${niri} Niri.qml
      cp ${sessions} Sessions.qml
      cp ${recorder} Recorder.qml
      cp ${shortcuts} Shortcuts.qml
      cp ${taken} Taken.qml
      cp ${files} Files.qml
      export HOME=$TMPDIR XDG_RUNTIME_DIR=$TMPDIR/runtime LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen
      mkdir -m 700 $XDG_RUNTIME_DIR
      export FONTCONFIG_FILE=${noFonts}
      mkdir $XDG_RUNTIME_DIR/daemons
      PROGRAMS_PREVIEW=${previewer} \
        ${programs}/bin/programs $XDG_RUNTIME_DIR/daemons/programs.sock ${sources} &
      daemon=$!
      PROGRAMS_PREVIEW=${silent} \
        ${programs}/bin/programs $XDG_RUNTIME_DIR/daemons/silent.sock ${sources} &
      mute=$!
      ${shellFolderLint}
      timeout 60 ${qs} -p shell.qml
      kill $daemon $mute
      touch $out
    '';

  # The shell's niri, told niri's events as its stream would, moves the keys
  # and says a window used without making any list again, and still lists
  # a window retitled, opened or closed (tests/niri-focus.qml).
  niri-moving-the-keys-makes-no-list-again =
    # Kept in Nix because each line copies a file Nix gives it beside the
    # cases, or sets what Quickshell runs them with.
    pkgs.runCommandLocal "niri-moving-the-keys-makes-no-list-again" { } ''
      cp ${./Niri.qml} Niri.qml
      cp ${./NiriRecords.js} NiriRecords.js
      cp ${./Daemon.qml} Daemon.qml
      cp ${../../tests/lib/Expect.js} Expect.js
      cp ${../../tests/niri-focus.qml} shell.qml
      export HOME=$TMPDIR XDG_RUNTIME_DIR=$TMPDIR/runtime LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen
      mkdir -m 700 $XDG_RUNTIME_DIR
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      timeout 60 ${qs} -p shell.qml
      touch $out
    '';

  # The launcher's Screens, asked of niri anew at every workspace change and
  # opening, makes no screen's row again while niri's answer reads the same,
  # and draws a screen that changed as it now stands (tests/displays.qml),
  # with niri stood in.
  the-launcher-remakes-only-the-screens-that-changed =
    let
      # A panel and a monitor, answered anew at each ask, as niri does.
      niri = ./tests/stand-ins/displays-niri.qml;
    in
    # Kept in Nix because each line copies a file Nix gives it beside the
    # cases, or sets what Quickshell runs them with.
    pkgs.runCommandLocal "the-launcher-remakes-only-the-screens-that-changed" { } ''
      mkdir -p launcher
      cp ${(account graphical).xdg.configFile."quickshell/Theme.qml".source} Theme.qml
      cp ${./Daemon.qml} Daemon.qml
      cp ${./Fuzzy.js} Fuzzy.js
      cp ${./ProgramGlyphs.js} ProgramGlyphs.js
      cp ${./launcher/Provider.qml} launcher/Provider.qml
      cp ${./launcher/Asking.js} launcher/Asking.js
      cp ${./launcher/Backlight.js} launcher/Backlight.js
      cp ${./launcher/Catalogue.js} launcher/Catalogue.js
      cp ${./launcher/Displays.qml} launcher/Displays.qml
      cp ${./launcher/Numbers.js} launcher/Numbers.js
      cp ${./launcher/Order.js} launcher/Order.js
      cp ${./launcher/Plate.js} launcher/Plate.js
      cp ${./launcher/Power.js} launcher/Power.js
      cp ${./launcher/ScreenAnswers.js} launcher/ScreenAnswers.js
      cp ${./launcher/Screens.js} launcher/Screens.js
      cp ${./launcher/Seen.js} launcher/Seen.js
      cp ${./launcher/Warmth.js} launcher/Warmth.js
      cp ${./Words.qml} Words.qml
      cp ${./launcher/words.json} launcher/words.json
      cp ${../../tests/displays.qml} shell.qml
      cp ${../../tests/lib/Expect.js} Expect.js
      cp ${niri} Niri.qml
      export HOME=$TMPDIR XDG_RUNTIME_DIR=$TMPDIR/runtime LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen
      mkdir -m 700 $XDG_RUNTIME_DIR
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      timeout 60 ${qs} -p shell.qml
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
      cp ${./ProgramGlyphs.js} ProgramGlyphs.js
      cp ${./launcher/Asking.js} launcher/Asking.js
      cp ${./launcher/Numbers.js} launcher/Numbers.js
      cp ${./launcher/Queue.qml} launcher/Queue.qml
      cp ${./launcher/Order.js} launcher/Order.js
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

  # pi's sessions are known by their own records as the machine's node
  # holds them, never by a window's title, and each one's window by the app
  # id it opened with, or for one opened without, through the kitty it runs
  # in (tests/sessions.qml), with the node, niri and kitten stood in over
  # what modules/quickshell/tests/stand-ins/sessions-state.sh lays out.
  a-session-is-known-by-its-own-record =
    let
      # Three windows of the same title, one of kitty 500's and two of 600's,
      # the second opened as pi-session.7 and laid out in workspace 3's
      # second column, each with the app id it opened with kept, as Niri.qml
      # keeps it; the windows daemon tells of the same three
      # (tests/sessions.qml).
      niri = singleton "Niri.qml" ''
        property var windows: ({
          "10": { id: 10, pid: 500, title: "\u03c0 - Lighthouse", opened: "kitty" },
          "20": { id: 20, pid: 600, title: "\u03c0 - Lighthouse", opened: "kitty" },
          "21": { id: 21, pid: 600, title: "\u03c0 - Lighthouse", opened: "pi-session.7",
            workspace_id: 3, layout: { pos_in_scrolling_layout: [2, 1] } }
        })
        property int revision: 0
        property var current: null
        property var acted: null
        property var spawned: []
        property var spawns: []
        function act(request) { acted = request; }
        // Asked anything, as Sessions asks for the outputs, it never
        // answers, as niri doesn't a request it refused.
        function request(asked, answer) {}
        // No workspace is known, the focused one included.
        function workspace(id) { return null; }
        function spawn(command) {
          spawned = command;
          spawns = spawns.concat([command.join(" ")]);
        }
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
      export HOME=$TMPDIR XDG_STATE_HOME=$PWD/state XDG_DATA_HOME=$PWD/data
      ${lib.getExe pkgs.bash} ${./tests/stand-ins/sessions-state.sh} $XDG_STATE_HOME
      export PATH=$XDG_STATE_HOME/bin:$PATH
      cp ${./Sessions.qml} Sessions.qml
      cp ${./SessionWindows.js} SessionWindows.js
      cp ${./PiSource.qml} PiSource.qml
      cp ${./PiSource.js} PiSource.js
      cp ${./Daemon.qml} Daemon.qml
      cp ${./Waiting.js} Waiting.js
      cp ${./Places.js} Places.js
      cp ${waiting} WaitingOnYou.qml
      cp ${report} Report.qml
      cp ${../../tests/sessions.qml} shell.qml
      cp ${niri} Niri.qml
      cp ${options} Options.qml
      cp ${quiet} Quiet.qml
      cp ${notices} Notices.qml
      export XDG_RUNTIME_DIR=$TMPDIR/runtime LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen
      mkdir -m 700 $XDG_RUNTIME_DIR $XDG_RUNTIME_DIR/mycelium
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      timeout 60 ${qs} -p shell.qml
      touch $out
    '';

  # A session handed over while the screen is locked, beside its sender in
  # the sessions' kitty, waits once the lock is gone until the node has
  # said which window is the sender's, then goes to the sender's workspace
  # (tests/sessions-behind-the-lock.qml), with niri stood in.
  a-session-behind-the-lock-waits-for-its-kin =
    let
      # The sender's kitty's two windows, on the second workspace, the
      # second opened as pi-session.3, each with the app id it opened with
      # kept, as Niri.qml keeps it.
      niri = singleton "Niri.qml" ''
        property var windows: ({
          "20": { id: 20, pid: 600, workspace_id: 2, opened: "kitty" },
          "21": { id: 21, pid: 600, workspace_id: 2, opened: "pi-session.3" }
        })
        property int revision: 0
        property var current: null
        property var focused: null
        property var acts: []
        property var spawns: []
        signal windowOpenedOrChanged(var window)
        function act(request) { acts = acts.concat([request]); }
        function request(asked, answer) {}
        function workspace(id) {
          return ({ 1: { id: 1, idx: 1, output: "Virtual-1" },
            2: { id: 2, idx: 2, output: "Virtual-1" } })[id] ?? null;
        }
        function spawn(command) { spawns = spawns.concat([command.join(" ")]); }
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
    pkgs.runCommandLocal "a-session-behind-the-lock-waits-for-its-kin" { } ''
      export HOME=$TMPDIR XDG_STATE_HOME=$PWD/state XDG_DATA_HOME=$PWD/data
      ${lib.getExe pkgs.bash} ${./tests/stand-ins/sessions-behind-the-lock-state.sh} $XDG_STATE_HOME
      cp ${./Sessions.qml} Sessions.qml
      cp ${./SessionWindows.js} SessionWindows.js
      cp ${./PiSource.qml} PiSource.qml
      cp ${./PiSource.js} PiSource.js
      cp ${./Daemon.qml} Daemon.qml
      cp ${./Waiting.js} Waiting.js
      cp ${./Places.js} Places.js
      cp ${waiting} WaitingOnYou.qml
      cp ${report} Report.qml
      cp ${../../tests/sessions-behind-the-lock.qml} shell.qml
      cp ${niri} Niri.qml
      cp ${options} Options.qml
      cp ${quiet} Quiet.qml
      cp ${notices} Notices.qml
      export XDG_RUNTIME_DIR=$TMPDIR/runtime LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen
      mkdir -m 700 $XDG_RUNTIME_DIR $XDG_RUNTIME_DIR/mycelium
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      timeout 60 ${qs} -p shell.qml
      touch $out
    '';

  # The launcher's People, with Tuple as its source, reads Tuple's contacts
  # from the real people daemon, which runs the real tuple-people reading
  # a stand-in for Tuple's follow here, and calls, joins or favours one
  # (tests/people.qml), with niri stood in. A listener stands in for
  # Tuple's daemon's socket, and the shell never asks Tuple anything
  # itself, since it reaches Tuple through niri alone.
  the-launcher-finds-the-people =
    let
      niri = singleton "Niri.qml" ''
        property var spawned: []
        function spawn(command) { spawned = command; }
      '';
      report = singleton "Report.qml" "function say(text) {}";
      people = pkgs.callPackage ../people/package.nix { };
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
      # and goes on following. Anything else asked of it is written down,
      # since the shell asks Tuple through niri alone.
      tuple = pkgs.writeShellScriptBin "tuple" ''
        if [ "$*" != "--format json contacts follow" ]; then
          echo "$*" >> "$XDG_RUNTIME_DIR/asked"
          exit 0
        fi
        cat ${snapshot "online"}
        while [ ! -e "$XDG_RUNTIME_DIR/next" ]; do sleep 0.1; done
        cat ${snapshot "busy"}
        while :; do sleep 1; done
      '';
    in
    pkgs.runCommandLocal "the-launcher-finds-the-people" { } ''
      mkdir -p launcher/people
      cp ${(account graphical).xdg.configFile."quickshell/Theme.qml".source} Theme.qml
      cp ${./Fuzzy.js} Fuzzy.js
      cp ${./launcher/Provider.qml} launcher/Provider.qml
      cp ${./ProgramGlyphs.js} ProgramGlyphs.js
      cp ${./launcher/Asking.js} launcher/Asking.js
      cp ${./launcher/Numbers.js} launcher/Numbers.js
      cp ${../people/People.qml} launcher/People.qml
      cp ${./launcher/Placed.js} launcher/Placed.js
      cp ${./launcher/Recent.js} launcher/Recent.js
      cp ${../people/Person.js} launcher/Person.js
      cp ${./launcher/Order.js} launcher/Order.js
      cp ${./launcher/Zones.js} launcher/Zones.js
      cp ${import ./launcher/zones.nix { inherit lib pkgs; }} zones.jsonl
      cp ${../people/PersonSource.qml} PersonSource.qml
      cp ${../tuple/TupleSource.qml} launcher/people/tuple.qml
      cp ${../../tests/people-page-source.qml} launcher/people/pages.qml
      cp ${../tuple/TuplePeople.qml} TuplePeople.qml
      cp ${./Daemon.qml} Daemon.qml
      cp ${./Words.qml} Words.qml
      cp ${./launcher/words.json} launcher/words.json
      cp ${../../tests/people.qml} shell.qml
      cp ${niri} Niri.qml
      cp ${report} Report.qml
      export HOME=$TMPDIR XDG_RUNTIME_DIR=$TMPDIR/runtime LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen
      # Here is UTC, so how far a zone is from here is known.
      export TZ=UTC
      mkdir -m 700 $XDG_RUNTIME_DIR
      mkdir $XDG_RUNTIME_DIR/daemons
      ${lib.getExe pkgs.socat} UNIX-LISTEN:$XDG_RUNTIME_DIR/tuple-0,fork,unlink-close=0 /dev/null &
      while [ ! -S $XDG_RUNTIME_DIR/tuple-0 ]; do sleep 0.1; done
      cat > sources.json <<EOF
      {"sources": [{"name": "tuple", "stands": $(cat ${../tuple/stands.json}),
        "command": ["${helper}/bin/tuple-people", "people", "${tuple}/bin/tuple", "tuple-0"]}]}
      EOF
      export PEOPLE=${lib.getExe people} SOURCES=$PWD/sources.json WORDS=${./launcher/words.json}
      $PEOPLE $XDG_RUNTIME_DIR/daemons/people.sock $SOURCES $WORDS &
      # The last steps stop the daemon and start it again, to see People
      # say so and then stop saying so.
      echo $! > $XDG_RUNTIME_DIR/people.pid
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      timeout 60 ${qs} -p shell.qml
      if [ -e $XDG_RUNTIME_DIR/asked ]; then
        echo "the shell asked Tuple itself:"; cat $XDG_RUNTIME_DIR/asked; exit 1
      fi
      touch $out
    '';

  # The launcher's Documents make no row while it's shut, however the clock
  # and the google daemon's rows move, and make them once as it opens, in
  # the daemon's order (tests/documents.qml), through the real Drive.qml
  # and Daemon.qml, with Google's account, the clock, the mail and the
  # saves stood in.
  a-shut-launcher-makes-no-document-rows =
    let
      # Kept in Nix because it only stands in for what Drive.qml reads of
      # Google's, answering nothing.
      google = singleton "Google.qml" ''
        property var documents: []
        property var drive: ({ places: {}, kept: {}, listings: [], downloads: [] })
        property var account: null
        function list(folder) {}
        function listing(folder, since) { return null; }
        function download(file, how, to) {}
        function downloading(file, since) { return null; }
      '';
      calendar = singleton "Calendar.qml" ''property real now: Date.parse("2026-11-10T12:00:00Z")'';
      mail = singleton "Mail.qml" "property var dates: ({})";
      saves = singleton "Saves.qml" "property var launcher: null";
      niri = singleton "Niri.qml" "function spawn(command) {}";
    in
    # Kept in Nix because each line copies a file Nix gives it beside the
    # cases, or sets what Quickshell runs them with.
    pkgs.runCommandLocal "a-shut-launcher-makes-no-document-rows" { } ''
      mkdir -p launcher
      cp ${../../tests/lib/Expect.js} Expect.js
      cp ${(account graphical).xdg.configFile."quickshell/Theme.qml".source} Theme.qml
      cp ${./Fuzzy.js} Fuzzy.js
      cp ${./ProgramGlyphs.js} ProgramGlyphs.js
      cp ${./Words.qml} Words.qml
      cp ${./FileFacts.js} FileFacts.js
      cp ${./When.js} When.js
      cp ${./launcher/Provider.qml} launcher/Provider.qml
      cp ${./launcher/Asking.js} launcher/Asking.js
      cp ${./launcher/Numbers.js} launcher/Numbers.js
      cp ${./launcher/words.json} launcher/words.json
      cp ${./Daemon.qml} Daemon.qml
      cp ${./launcher/Placed.js} launcher/Placed.js
      cp ${../google/Documents.qml} launcher/Documents.qml
      cp ${../google/Drive.qml} Drive.qml
      cp ${../google/Drive.js} Drive.js
      cp ${../google/DrivePlaces.js} DrivePlaces.js
      cp ${../google/Mail.js} Mail.js
      cp ${google} Google.qml
      cp ${calendar} Calendar.qml
      cp ${mail} Mail.qml
      cp ${saves} Saves.qml
      cp ${niri} Niri.qml
      cp ${../../tests/documents.qml} shell.qml
      export HOME=$TMPDIR XDG_RUNTIME_DIR=$TMPDIR/runtime LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen
      export TZ=UTC FONTCONFIG_FILE=${noFonts}
      mkdir -m 700 $XDG_RUNTIME_DIR
      ${shellFolderLint}
      timeout 60 ${qs} -p shell.qml
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

  # A file reloaded while it's still being read is read once more as that
  # read lands, so the view holds what was written last, as
  # tests/reloaded.qml drives it in the Quickshell the machine runs
  # (fileview-reads-again.patch): a status file taken away mid-read stayed
  # said, as the-desktop-reads-the-spend-watch once saw on a busy builder.
  a-file-reloaded-mid-read-is-read-again =
    pkgs.runCommandLocal "a-file-reloaded-mid-read-is-read-again" { }
      ''
        mkdir runtime && chmod 700 runtime
        cp ${../../tests/reloaded.qml} shell.qml
        export HOME=$TMPDIR XDG_RUNTIME_DIR=$TMPDIR/runtime LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen
        export FONTCONFIG_FILE=${noFonts}
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

  # The desktop entries are read again only when a folder holding them
  # changes, as tests/desktop-entries.qml drives them in the Quickshell the
  # machine runs (desktop-entries-heard-only-when-theirs.patch): a file
  # written beside them, as a hidden file in the home folder is, reads nothing,
  # while an entry added and a link on the way pointed elsewhere each read
  # them once, counted from Quickshell's log of each scan.
  desktop-entries-are-read-only-when-theirs-change =
    pkgs.runCommandLocal "desktop-entries-are-read-only-when-theirs-change" { }
      ''
        entry() { printf '[Desktop Entry]\nType=Application\nName=%s\nExec=true\n' "$1"; }
        mkdir -m 700 runtime
        mkdir -p home data/applications entries-first/share/applications \
          entries-next/share/applications
        entry First > data/applications/first.desktop
        entry System > entries-first/share/applications/system.desktop
        entry Next > entries-next/share/applications/next.desktop
        ln -s "$PWD/entries-first" entries
        cp ${../../tests/desktop-entries.qml} shell.qml
        export HOME=$PWD/home XDG_DATA_HOME=$PWD/data XDG_DATA_DIRS=$PWD/entries/share
        export ENTRIES_LINK=$PWD/entries XDG_RUNTIME_DIR=$PWD/runtime LANG=C.UTF-8
        export QT_QPA_PLATFORM=offscreen FONTCONFIG_FILE=${noFonts}
        export PATH=${pkgs.coreutils}/bin:$PATH
        ${shellFolderLint}
        timeout 60 ${qs} -p shell.qml --log-rules 'quickshell.desktopentry.debug=true' \
          > said 2>&1 || { cat said; exit 1; }
        cat said
        scans=$(grep -c 'Directory change detected, performing full rescan' said || true)
        [ "$scans" = 2 ] || {
          echo "the desktop entries were read $scans times for 2 changes of their own"
          exit 1
        }
        touch $out
      '';

  # A row's graph is ten braille characters over a dim track of every dot,
  # a bar filling from the left, a spark from the right with the newest last
  # and a strip a reading for each member, in the warning colour at 95% or
  # when it says it warns; rows' graphs stand in one column, and a figure
  # wears no glyph, as tests/graphs.qml draws them through Quickshell in the
  # theme's own face and sizes, at a panel's width.
  the-launchers-graphs-draw-in-braille =
    let
      machine = graphical.config;
    in
    pkgs.runCommandLocal "the-launchers-graphs-draw-in-braille" { } ''
      cp ${(account graphical).xdg.configFile."quickshell/Theme.qml".source} Theme.qml
      cp ${./Choice.qml} Choice.qml
      cp ${./Ground.qml} Ground.qml
      cp ${./ReadsOn.qml} ReadsOn.qml
      cp ${./Wash.qml} Wash.qml
      cp ${./Reading.js} Reading.js
      cp ${./Measures.js} Measures.js
      cp ${./MarkSize.js} MarkSize.js
      cp ${./Mark.qml} Mark.qml
      cp ${./Columns.js} Columns.js
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

  # A floating surface standing over another, as the launcher's drawer over
  # its card on a narrow screen, meets it with a square foot, its region
  # alike, its top and the card's foot staying round, as tests/floating.qml
  # reads them back from what Quickshell draws.
  the-drawer-over-the-card-meets-it-square =
    pkgs.runCommandLocal "the-drawer-over-the-card-meets-it-square" { }
      ''
        cp ${(account graphical).xdg.configFile."quickshell/Theme.qml".source} Theme.qml
        cp ${./Floating.qml} Floating.qml
        cp ${../../tests/floating.qml} shell.qml
        export HOME=$TMPDIR XDG_RUNTIME_DIR=$TMPDIR/runtime LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen
        export FONTCONFIG_FILE=${noFonts}
        mkdir -m 700 $XDG_RUNTIME_DIR
        ${shellFolderLint}
        timeout 60 ${qs} -p shell.qml
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
        cp ${./Floating.qml} Floating.qml
        cp ${./Card.qml} Card.qml
        cp ${./NoticeRow.qml} NoticeRow.qml
        cp ${./Mark.qml} Mark.qml
        cp ${./MarkSize.js} MarkSize.js
        cp ${./Columns.js} Columns.js
        cp ${./Signature.qml} Signature.qml
        cp ${./Cover.qml} Cover.qml
        cp ${../../tests/notice-columns.qml} shell.qml
        ${shellFolderLint}
        QT_SCALE_FACTOR=$scale timeout 60 ${qs} -p shell.qml > said 2>&1 || { cat said; exit 1; }
        ${measure} ${./tests/notice-columns.py} . said
        cd ..
      done
      touch $out
    '';

  # A palette's row and its field keep their words apart and inside their
  # edges however long they run, ending what doesn't fit in an ellipsis, as
  # tests/rows.qml draws them through Quickshell in the theme's own face and
  # sizes, at a panel's width.
  the-launchers-rows-keep-their-words-apart =
    let
      machine = graphical.config;
      lights = pkgs.writeText "Lights.qml" ''
        pragma Singleton
        import Quickshell
        import QtQuick
        Singleton { property bool caps: false }
      '';
    in
    pkgs.runCommandLocal "the-launchers-rows-keep-their-words-apart" { } ''
      cp ${(account graphical).xdg.configFile."quickshell/Theme.qml".source} Theme.qml
      cp ${./Choice.qml} Choice.qml
      cp ${./Ground.qml} Ground.qml
      cp ${./Gutter.qml} Gutter.qml
      cp ${./ReadsOn.qml} ReadsOn.qml
      cp ${./Wash.qml} Wash.qml
      cp ${./Reading.js} Reading.js
      cp ${./Measures.js} Measures.js
      cp ${./MarkSize.js} MarkSize.js
      cp ${./Mark.qml} Mark.qml
      cp ${./Columns.js} Columns.js
      cp ${./Graph.qml} Graph.qml
      cp ${./Chord.qml} Chord.qml
      cp ${./Prompt.qml} Prompt.qml
      cp ${./Bands.js} Bands.js
      cp ${./Confirm.js} Confirm.js
      cp ${./KeyNames.js} KeyNames.js
      cp ${./QueryKeys.js} QueryKeys.js
      cp ${./Operators.js} Operators.js
      cp ${./Finding.js} Finding.js
      cp ${./Typing.js} Typing.js
      cp ${./LineEditing.js} LineEditing.js
      cp ${./Editing.js} Editing.js
      cp ${./KeyHints.qml} KeyHints.qml
      cp ${./PromptFoot.qml} PromptFoot.qml
      cp ${./PromptHead.qml} PromptHead.qml
      cp ${./Hairline.qml} Hairline.qml
      cp ${./StatusLine.qml} StatusLine.qml
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
  # the shell's lights stood in for. Unplugged, as the kernel's list and
  # keyboard-lights' say, the laptop's own layout, keyd's, takes its place.
  # The machine daemon tells the keyboards and the layer from its files,
  # under a root of its own.
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
      # No shortcuts, and no key faces: the boards here count nothing.
      shortcuts = singleton "Shortcuts.qml" ''
        property var actions: []
        property var faces: ({})
      '';
      # The laptop's own layout, as keyd's module describes it.
      laptop =
        pkgs.writeText "laptop.json"
          (import ../keyd/layout.nix { inherit lib; }).environment.etc."keyboard/layout.json".text;
    in
    pkgs.runCommandLocal "the-launcher-finds-the-keyboards-keys" { } ''
      mkdir -p runtime/daemons config/ergodox launcher && chmod 700 runtime
      mkdir -p root/proc/bus/input root/run/keyboard-lights root/run/user/1000
      echo "Numb locked" > root/run/user/1000/keyboard-layer
      printf '1\n9\n' > root/run/keyboard-lights/keyboards
      cp ${firmware}/shell.json config/ergodox/layout.json
      cp ${laptop} laptop.json
      cp ${./ConnectedKeyboards.qml} ConnectedKeyboards.qml
      cp ${./Daemon.qml} Daemon.qml
      cp ${../../tests/keyboard.qml} shell.qml
      cp ${./KeyboardLayout.qml} KeyboardLayout.qml
      cp ${./KeyboardLayer.qml} KeyboardLayer.qml
      cp ${./Fuzzy.js} Fuzzy.js
      cp ${./launcher/Provider.qml} launcher/Provider.qml
      cp ${./ProgramGlyphs.js} ProgramGlyphs.js
      cp ${./launcher/Asking.js} launcher/Asking.js
      cp ${./launcher/Numbers.js} launcher/Numbers.js
      cp ${./launcher/Keyboard.qml} launcher/Keyboard.qml
      cp ${./launcher/Keyboards.qml} launcher/Keyboards.qml
      cp ${./launcher/Keyboards.js} launcher/Keyboards.js
      cp ${./launcher/KeyCounts.js} launcher/KeyCounts.js
      cp ${./launcher/Order.js} launcher/Order.js
      cp ${(account graphical).xdg.configFile."quickshell/Theme.qml".source} Theme.qml
      cp ${./Words.qml} Words.qml
      cp ${./launcher/words.json} launcher/words.json
      cp ${niri} Niri.qml
      cp ${shortcuts} Shortcuts.qml
      cp ${lights} Lights.qml
      export HOME=$TMPDIR XDG_RUNTIME_DIR=$TMPDIR/runtime XDG_CONFIG_HOME=$TMPDIR/config
      export MACHINE_ROOT=$TMPDIR/root LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      XDG_RUNTIME_DIR=/run/user/1000 \
        ${lib.getExe machine} "$TMPDIR/runtime/daemons/machine.sock" "$MACHINE_ROOT" &
      daemon=$!
      ${shellFolderLint}
      timeout 60 ${qs} -p shell.qml
      kill "$daemon"
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
  # A stand-in Facts says the headphones are running low, as the devices
  # piece would. A stand-in adapter and launcher hold Nearby to looking
  # only while it's open, with devices heard as the shell's patch to
  # Quickshell says them (bluetooth-rssi-and-failures.patch).
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
        cp ${./Devices.js} Devices.js
        cp ${./Crossing.js} Crossing.js
        cp ${./launcher/Provider.qml} launcher/Provider.qml
        cp ${./ProgramGlyphs.js} ProgramGlyphs.js
        cp ${./launcher/Asking.js} launcher/Asking.js
        cp ${./launcher/Numbers.js} launcher/Numbers.js
        cp ${./launcher/Accessories.qml} launcher/Accessories.qml
        cp ${./launcher/Order.js} launcher/Order.js
        cp ${./launcher/Links.js} launcher/Links.js
        cp ${./launcher/Nearby.js} launcher/Nearby.js
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
        # The headphones running low, as the devices piece would say them.
        cp ${pkgs.writeText "Facts.qml" ''
          pragma Singleton
          import Quickshell
          import QtQuick
          Singleton {
            readonly property var all: [
              { entry: { mark: "bluetooth:00:1B:66:AA:BB:CC" }, piece: { place: "bluetooth" } }
            ]
          }
        ''} Facts.qml
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

  # A media player's drawer says all its player tells MPRIS, in the order
  # every drawer keeps, and draws its art only from where it may, as
  # tests/media.qml holds launcher/Media.js and Pictures.js to players
  # made up there.
  a-player-says-all-it-knows =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "a-player-says-all-it-knows" { } ''
      mkdir launcher
      cp ${../../tests/media.qml} media.qml
      cp ${./Music.js} Music.js
      cp ${./Pictures.js} Pictures.js
      cp ${./launcher/Media.js} launcher/Media.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml media.qml
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
      cp ${../../tests/program-drawer.qml} program-drawer.qml
      cp ${./launcher/Program.js} launcher/Program.js
      cp ${./launcher/Numbers.js} launcher/Numbers.js
      cp ${./FileFacts.js} FileFacts.js
      cp ${./When.js} When.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml program-drawer.qml
      touch $out
    '';

  # The launcher keeps the windows the windows daemon tells of in the order
  # it lists them, the one used last first, placing each window a change
  # names and letting those gone go, as tests/held-windows.qml holds
  # launcher/HeldWindows.js to records made up there.
  the-launcher-places-each-window-it-hears =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "the-launcher-places-each-window-it-hears" { } ''
      mkdir launcher
      cp ${../../tests/held-windows.qml} held-windows.qml
      cp ${./launcher/HeldWindows.js} launcher/HeldWindows.js
      cp ${../../tests/lib/Expect.js} Expect.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml held-windows.qml
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
      cp ${../../tests/placed.qml} placed.qml
      cp ${./launcher/Placed.js} launcher/Placed.js
      cp ${../../tests/lib/Expect.js} Expect.js
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
      cp ${../../tests/ready-rows.qml} ready-rows.qml
      cp ${./launcher/ReadyRows.js} launcher/ReadyRows.js
      cp ${./launcher/Ago.js} launcher/Ago.js
      cp ${./launcher/Interfaces.js} launcher/Interfaces.js
      cp ${./When.js} When.js
      cp ${../../tests/lib/Expect.js} Expect.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml ready-rows.qml
      touch $out
    '';

  # The shell's niri holds what the windows daemon tells in niri's own
  # shape, each window told made again in place, and says to its readers
  # what niri's events said, a window used or a tile moving making no list
  # again (tests/niri-records.qml over NiriRecords.js).
  niri-is-held-as-the-windows-daemon-tells-it =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "niri-is-held-as-the-windows-daemon-tells-it" { } ''
      cp ${../../tests/niri-records.qml} niri-records.qml
      cp ${./NiriRecords.js} NiriRecords.js
      cp ${../../tests/lib/Expect.js} Expect.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml niri-records.qml
      touch $out
    '';

  # Now's own pieces say in their drawers all their source holds, in the
  # order every drawer keeps, as tests/piece-drawers.qml holds
  # pieces/Drawers.js to what the disk guards, Notices, lag-watch, niri,
  # the recorder and Quiet say.
  a-piece-says-all-it-knows =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "a-piece-says-all-it-knows" { } ''
      mkdir pieces
      cp ${../../tests/piece-drawers.qml} piece-drawers.qml
      cp ${./pieces/Drawers.js} pieces/Drawers.js
      cp ${./FileFacts.js} FileFacts.js
      cp ${./When.js} When.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml piece-drawers.qml
      touch $out
    '';

  # What Capture took says where it is and what kind, when it was taken,
  # its size and a picture's dimensions or a recording's length, as
  # tests/captured.qml holds launcher/Captured.js to what Taken.qml reads
  # and file-preview says.
  a-capture-says-what-it-took =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "a-capture-says-what-it-took" { } ''
      mkdir launcher
      cp ${../../tests/captured.qml} captured.qml
      cp ${./launcher/Captured.js} launcher/Captured.js
      cp ${./launcher/Recent.js} launcher/Recent.js
      cp ${./launcher/Order.js} launcher/Order.js
      cp ${./FileFacts.js} FileFacts.js
      cp ${./When.js} When.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml captured.qml
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

  # A notice's body read once as it arrives, its links kept and every tag
  # but the spec's escaped, as tests/notice-body.qml holds NoticeBody.js.
  a-notice-keeps-its-links =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "a-notice-keeps-its-links" { } ''
      cp ${../../tests/notice-body.qml} notice-body.qml
      cp ${./NoticeBody.js} NoticeBody.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml notice-body.qml
      touch $out
    '';

  # For You's Lately holds what's unread from each source until that source
  # says it's done, and History says how, as tests/lately.qml holds
  # Lately.js.
  lately-follows-each-source =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "lately-follows-each-source" { } ''
      cp ${../../tests/lately.qml} lately.qml
      cp ${./launcher/Lately.js} Lately.js
      cp ${./launcher/Order.js} Order.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml lately.qml
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
        property var reworded: []
        property var raisedIds: []
        function mark(id, read) { marked = marked.concat([id]); }
        function reword(id, summary, body) { reworded = reworded.concat([id]); }
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
  # within each, as tests/now-groups.qml holds launcher/NowGroups.js to it.
  now-lists-by-group =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "now-lists-by-group" { } ''
      cp ${../../tests/now-groups.qml} now-groups.qml
      cp ${./launcher/NowGroups.js} NowGroups.js
      cp ${./launcher/Order.js} Order.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml now-groups.qml
      touch $out
    '';

  # Every glyph in the margin stands the same height on the gap's middle
  # line, however tall its font draws it, and they line up from the corner
  # by how much each asks, in the colour that says so, and For You's tray
  # is lit by what For You holds, as tests/marks.qml holds them.
  the-margins-marks-stand-level =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "the-margins-marks-stand-level" { } ''
      cp ${../../tests/marks.qml} marks.qml
      cp ${./Marks.js} Marks.js
      cp ${./pieces/Holding.js} Holding.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml marks.qml
      touch $out
    '';

  # For You's to-dos hold only what Joel can do, now or later, in the order he
  # does them, and a session's want is his only while it's true, as
  # tests/waiting-on-you.qml holds Waiting.js to the agents' own files, as
  # the mycelium node serves them.
  needs-you-is-true =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "needs-you-is-true" { } ''
      cp ${../../tests/waiting-on-you.qml} waiting-on-you.qml
      cp ${./Waiting.js} Waiting.js
      cp ${./PiSource.js} PiSource.js
      cp ${./launcher/NeedsYou.js} NeedsYou.js
      cp ${./launcher/Ago.js} Ago.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml waiting-on-you.qml
      touch $out
    '';

  # For You weighs what waits on Joel again only when the node says something
  # new of it, not each time a file of it is written again, as
  # tests/waiting-kept.qml holds the real WaitingOnYou.qml to, through
  # Quickshell, with a stand-in for the node and the rest of the shell.
  for-you-weighs-again-only-for-news =
    let
      folder = pkgs.linkFarm "for-you-weighs-again-only-for-news" {
        "WaitingOnYou.qml" = ./WaitingOnYou.qml;
        "Waiting.js" = ./Waiting.js;
        "PiSource.js" = ./PiSource.js;
        "Daemon.qml" = ./Daemon.qml;
        "Expect.js" = ../../tests/lib/Expect.js;
        "shell.qml" = ../../tests/waiting-kept.qml;
        "Sessions.qml" = singleton "Sessions.qml" ''
          readonly property string nodePath: Quickshell.env("XDG_RUNTIME_DIR") + "/node.sock"
          property var present: ({})
          property var running: []
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

  # Every mark the margin can show, each kind a device running low is drawn
  # as among them, is the theme's drawing of it, standing the grid's full
  # height and centred across it as Qt draws it, so a row of them is level
  # and each has the same room either side (tests/drawings.qml).
  the-margins-marks-are-drawn-level =
    let
      qt = pkgs.qt6.qtdeclarative;
      # With Slack too, whose piece only a machine that brings Slack has.
      home = account (bareNixos [
        self.nixosModules.graphical
        { home-manager.sharedModules = [ self.homeModules.slack ]; }
      ]);
      marks = lib.unique (
        lib.filter (mark: mark != null) (
          lib.mapAttrsToList (_: piece: piece.mark) home.jitsusama.quickshell.margin.pieces
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
      cp ${./Devices.js} Devices.js
      cp ${./Crossing.js} Crossing.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml drawings.qml
      touch $out
    '';

  # Vim's command line is read as Vim reads it, as tests/ex.qml holds Ex.js
  # to: ranges, names by their short forms, a bang, :g and :v, the rows a
  # range names, and Vim's own errors.
  ex-reads-vims-command-lines =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "ex-reads-vims-command-lines" { } ''
      cp ${../../tests/ex.qml} ex.qml
      cp ${./Ex.js} Ex.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml ex.qml
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
      cp ${../../tests/notice-keys.qml} notice-keys.qml
      cp ${./NoticeKeys.js} NoticeKeys.js
      cp ${./LineEditing.js} LineEditing.js
      cp ${./Search.js} Search.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml notice-keys.qml
      touch $out
    '';

  # The launcher's command line lists every command it can take, the cursor
  # row's acts first, narrows as it's typed, chooses as his cmp does and
  # runs exactly what's typed, with Vim's ranges and :g, and reads :set as
  # Vim does, as tests/command-line.qml holds Commands.js and SetWords.js
  # to, with the Ex.js they read ranges by and CommandBinding.js, which
  # binds a line to things and asks by d's rule, in Operators.js.
  the-command-line-lists-every-command =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "the-command-line-lists-every-command" { } ''
      cp ${../../tests/command-line.qml} command-line.qml
      cp ${./Commands.js} Commands.js
      cp ${./SetWords.js} SetWords.js
      cp ${./Ex.js} Ex.js
      cp ${./CommandBinding.js} CommandBinding.js
      cp ${./Operators.js} Operators.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml command-line.qml
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
      cp ${../../tests/status-line.qml} status-line.qml
      cp ${./StatusLine.js} StatusLine.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml status-line.qml
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
      cp ${../../tests/notice-list.qml} notice-list.qml
      cp ${./NoticeList.js} NoticeList.js
      mkdir launcher
      cp ${./launcher/Order.js} launcher/Order.js
      cp ${./Changes.js} Changes.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml notice-list.qml
      touch $out
    '';

  # The launcher's query line takes vi's two modes from one table, as
  # tests/query-keys.qml holds QueryKeys.js and Editing.js to: insert's
  # keys, normal's ways back into insert, its moves through the rows with counts and
  # where each lands, its closing keys and ex line, its place used last,
  # sequences, and where the caret goes, which Typing.js says; ListedKeys.js
  # to the keys Space ? lists; the drawer as a window, Space and the arrows
  # into it and back, with DrawerPage.js to where its keys take its cursor
  # and view and DrawerFigure.js to where a fact's figure stands; Operators.js to what d and y do to the rows they take; and
  # Confirm.js to how the launcher asks once, as Vim's ask_yesno does.
  the-query-field-takes-vi-modes =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "the-query-field-takes-vi-modes" { } ''
      cp ${../../tests/query-keys.qml} query-keys.qml
      cp ${./KeyNames.js} KeyNames.js
      cp ${./QueryKeys.js} QueryKeys.js
      cp ${./ModeMarks.js} ModeMarks.js
      cp ${./Editing.js} Editing.js
      cp ${./ListedKeys.js} ListedKeys.js
      cp ${./Typing.js} Typing.js
      cp ${./Operators.js} Operators.js
      cp ${./Finding.js} Finding.js
      cp ${./Confirm.js} Confirm.js
      cp ${./DrawerPage.js} DrawerPage.js
      cp ${./DrawerFigure.js} DrawerFigure.js
      cp ${./Identifier.js} Identifier.js
      cp ${./Search.js} Search.js
      cp ${./Reading.js} Reading.js
      cp ${./LineEditing.js} LineEditing.js
      cp ${./launcher/words.json} words.json
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts} QML_XHR_ALLOW_FILE_READ=1
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml query-keys.qml
      touch $out
    '';

  # A piece standing for several things is a fact for each, known by its
  # own mark, as tests/facts.qml holds Facts.js to.
  a-piece-may-stand-for-several =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "a-piece-may-stand-for-several" { } ''
      cp ${../../tests/facts.qml} facts.qml
      cp ${./Facts.js} Facts.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml facts.qml
      touch $out
    '';

  # A battery that reports only its charge, as a Bluetooth device's does,
  # is low and then critical once each, as tests/crossing.qml holds.
  a-low-charge-is-said-once =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "a-low-charge-is-said-once" { } ''
      cp ${../../tests/crossing.qml} crossing.qml
      cp ${./Crossing.js} Crossing.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml crossing.qml
      touch $out
    '';

  # Each Bluetooth device running low is a thing of its own, by its own
  # charge, name and kind, as tests/devices.qml holds Devices.js to with
  # plain objects standing in for BlueZ's devices, since the desktop VM has
  # no Bluetooth.
  a-low-device-is-its-own-notice =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "a-low-device-is-its-own-notice" { } ''
      cp ${../../tests/devices.qml} devices.qml
      cp ${./Devices.js} Devices.js
      cp ${./Crossing.js} Crossing.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml devices.qml
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

  # How the shell says a Tuple call, who's in it and for how long, a
  # call's link in what was copied, and Tuple's Recent, its order and each
  # call's words (tests/tuple.qml, over Tuple.js and the When.js it reads).
  the-desktop-follows-tuple-calls =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "the-desktop-follows-tuple-calls" { } ''
      cp ${../../tests/tuple.qml} tuple.qml
      cp ${../tuple/Tuple.js} Tuple.js
      cp ${./When.js} When.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml tuple.qml
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
      openNow = pkgs.writeShellScript "open-now" (builtins.readFile ../../tests/neovim-open-now.sh);
      # file-preview stood in, saying which process answered.
      previewer = pkgs.writeShellScript "file-preview" (builtins.readFile ../../tests/file-preview.sh);
      paste = pkgs.writeShellScriptBin "wl-paste" "exec sleep 300";
      programs = pkgs.callPackage ../programs/package.nix { };
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
        repository far/away/too origin git@github.com:someone/too.git 1700000950
        # A document outside them, for zathura's act.
        printf '%%PDF-1.4\n' > $HOME/report.pdf
      '';
    in
    pkgs.runCommandLocal "the-launcher-acts-only-on-what-it-can" { } ''
      mkdir cases shell
      cd $TMPDIR/cases
      cp ${../../tests/quick-acts.qml} quick-acts.qml
      cp ${./Copied.js} Copied.js
      cp ${./launcher/Repositories.js} Repositories.js
      cp ${../neovim/NeovimOpening.js} NeovimOpening.js
      cp ${../chrome/ChromeDownloads.js} ChromeDownloads.js
      cp ${../neovim/Neovim.js} Neovim.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml quick-acts.qml
      cd $TMPDIR/shell
      mkdir -p launcher
      cp ${(account graphical).xdg.configFile."quickshell/Theme.qml".source} Theme.qml
      cp ${./Fuzzy.js} Fuzzy.js
      cp ${./ProgramGlyphs.js} ProgramGlyphs.js
      cp ${./Words.qml} Words.qml
      cp ${./Files.qml} Files.qml
      cp ${./FileFacts.js} FileFacts.js
      cp ${./When.js} When.js
      cp ${./Copied.qml} Copied.qml
      cp ${./Copied.js} Copied.js
      cp ${niri} Niri.qml
      cp ${./launcher/words.json} launcher/words.json
      cp ${./launcher/Provider.qml} launcher/Provider.qml
      cp ${./launcher/Asking.js} launcher/Asking.js
      cp ${./launcher/Numbers.js} launcher/Numbers.js
      cp ${./launcher/Ago.js} launcher/Ago.js
      cp ${./launcher/Repositories.qml} launcher/Repositories.qml
      cp ${./launcher/Repositories.js} launcher/Repositories.js
      cp ${../neovim/NeovimProgram.qml} launcher/NeovimProgram.qml
      cp ${./launcher/Previewer.qml} launcher/Previewer.qml
      cp ${./launcher/Captured.js} launcher/Captured.js
      cp ${./launcher/Recent.js} launcher/Recent.js
      cp ${./launcher/Order.js} launcher/Order.js
      cp ${../neovim/NeovimOpening.js} launcher/NeovimOpening.js
      cp ${../neovim/Neovim.js} launcher/Neovim.js
      cp ${../chrome/ChromeProgram.qml} launcher/ChromeProgram.qml
      cp ${../chrome/ChromeDownloads.js} launcher/ChromeDownloads.js
      cp ${./Daemon.qml} Daemon.qml
      cp ${./launcher/Listed.qml} launcher/Listed.qml
      cp ${../reader/ReaderProgram.qml} launcher/ReaderProgram.qml
      cp ${../zathura/ZathuraProgram.qml} launcher/ZathuraProgram.qml
      cp ${../zathura/Zathura.js} launcher/Zathura.js
      cp ${../reader/Reader.js} launcher/Reader.js
      cp ${../gimp/CopiedPicture.qml} launcher/CopiedPicture.qml
      cp ${../../tests/program-acts.qml} shell.qml
      cp ${../../tests/lib/Frames.js} Frames.js
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

  # How the call Joel's in stands comes from tuple-call, the real daemon,
  # over its socket (tests/tuple-call.qml): it follows a stand-in tuple
  # whose stream says each line the test writes, and whose other commands
  # are kept, while a listener stands in for Tuple's daemon answering. The
  # shell tells who comes and goes through a stand-in notify-send that keeps
  # what it's asked. The daemon's own reading of Tuple is its tests', run as
  # it builds (tuple-call-holds-the-call).
  the-desktop-hears-tuples-call =
    let
      calls = pkgs.callPackage ../tuple/call/package.nix { };
      niri = pkgs.writeText "Niri.qml" ''
        pragma Singleton
        import Quickshell
        import QtQuick
        Singleton {
          property var windows: ({})
          property var casts: ({})
          property var spawned: []
          function spawn(command) { spawned = spawned.concat([command]); }
          function act(request) { }
        }
      '';
      # Tuple's stream says each line of the runtime folder's stream as
      # it's written; `new` opens a room, and every other command is kept.
      tuple = pkgs.writeShellScript "tuple" ''
        if [ "$3 $4" = "state follow" ]; then
          exec ${pkgs.coreutils}/bin/tail -n +1 -f "$XDG_RUNTIME_DIR/stream"
        fi
        echo "$*" >> "$XDG_RUNTIME_DIR/asked"
        if [ "$*" = new ]; then echo "Joining https://tuple.app/c/opened"; fi
      '';
      # Tuple's state as its stream opens: someone signed in, one contact,
      # and Joel's own rooms, the newest of which is his. Tuple names a
      # line's type first, which is all tuple-call reads of a line it
      # skips, so the type isn't left to toJSON's order.
      state = ''{"type":"state","data":${
        builtins.toJSON {
          sign_in = "signed_in";
          contacts = [
            {
              id = 7;
              full_name = "Ada Brightwater";
              email = "ada@example.com";
            }
          ];
          rooms.personal = [
            {
              http_value = "https://tuple.app/c/older";
              created_at = "2025-04-11T12:00:00Z";
            }
            {
              http_value = "https://tuple.app/c/listed";
              created_at = "2026-07-31T12:00:00Z";
            }
          ];
        }
      }}'';
      notify = pkgs.writeShellApplication {
        name = "notify-send";
        text = ''
          printf '%s\n' "$*" >> "$XDG_RUNTIME_DIR/told"
          echo 1
        '';
      };
      paste = pkgs.writeShellScriptBin "wl-paste" "exec sleep 300";
    in
    pkgs.runCommandLocal "the-desktop-hears-tuples-call" { } ''
      cp ${../tuple/Tuple.qml} Tuple.qml
      cp ${./Daemon.qml} Daemon.qml
      # The copied link Tuple offers to join is read from the shell's one
      # watcher of the clipboard.
      cp ${./Copied.qml} Copied.qml
      cp ${./Copied.js} Copied.js
      # Tuple's rows are named as the launcher's vocabulary says.
      mkdir launcher
      cp ${(account graphical).xdg.configFile."quickshell/Theme.qml".source} Theme.qml
      cp ${./Words.qml} Words.qml
      cp ${./launcher/words.json} launcher/words.json
      cp ${../tuple/Tuple.js} Tuple.js
      cp ${./When.js} When.js
      cp ${../../tests/tuple-call.qml} shell.qml
      cp ${niri} Niri.qml
      export HOME=$TMPDIR XDG_RUNTIME_DIR=$TMPDIR/runtime XDG_DATA_HOME=$TMPDIR/data
      export XDG_STATE_HOME=$TMPDIR/state LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen
      export PATH=${notify}/bin:${paste}/bin:$PATH
      mkdir -m 700 $XDG_RUNTIME_DIR $XDG_RUNTIME_DIR/daemons
      mkdir -p $XDG_DATA_HOME/tuple/0
      touch $XDG_DATA_HOME/tuple/0/log.txt $XDG_RUNTIME_DIR/told
      printf '%s\n' ${lib.escapeShellArg state} > $XDG_RUNTIME_DIR/stream
      ${lib.getExe pkgs.socat} UNIX-LISTEN:$XDG_RUNTIME_DIR/tuple-0,fork /dev/null &
      # Tuple's daemon answers before tuple-call first looks, which would
      # otherwise look again only five seconds on.
      timeout 10 sh -c 'until [ -S "$0" ]; do sleep 0.05; done' $XDG_RUNTIME_DIR/tuple-0
      ${lib.getExe calls} $XDG_RUNTIME_DIR/daemons/tuple.sock ${tuple} &
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      timeout 60 ${qs} -p shell.qml
      touch $out
    '';

  # Shape is a language (docs/design.md, Shape and Kinds): a border's width
  # says "this is a thing" and a radius says whole or part, so no surface
  # picks either for itself. Every corner is the theme's radius, or a part's
  # radius the theme derives from it, or a floating surface's foot or top,
  # which is the theme's radius or square where it meets another
  # (Floating.qml's footRadius and topRadius), and every border is the
  # theme's.
  quickshell-takes-its-shape-from-the-theme =
    let
      dir = ../.;
      files = everyShellFile;
      allowed = {
        radius = [
          "Theme.radius"
          "Theme.inner"
          "floating.footRadius"
          "floating.topRadius"
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

  # Space is one scale (docs/design.md, Space): a padding, a margin, a gap or
  # a line names a step, or sums and multiples of steps, so every surface
  # lands on the same few values. A raw number added to a step, a step
  # divided, or a raw padding, margin or spacing is a nudge off it, the way
  # the ink was once evened by hand. A count taken from (`length - 1`) and a
  # share of the screen (`parent.height / 12`) aren't steps, so they pass.
  quickshell-keeps-to-the-space-scale =
    let
      dir = ../.;
      step = "Theme\\.(space\\.[a-z]+|inset|border|gap)";
      spacing = "([pP]adding|[mM]argins?|spacing)[[:space:]]*:";
      # Each raw number taken from or added to something, with what comes
      # before it and whether a step multiplies it.
      added = "([a-zA-Z_]*)[[:space:]]*[-+][[:space:]]*([0-9]+(\\.[0-9]+)?)([[:space:]]*[*])?";
      divided = "(${step}|[0-9])[[:space:]]*/[[:space:]]*[0-9]+";
      nudged =
        line:
        let
          adds = lib.filter lib.isList (builtins.split added line);
          raw = lib.any (
            found:
            lib.elemAt found 3 == null
            && !(lib.elem (lib.head found) [
              "length"
              "count"
              "index"
            ])
          ) adds;
        in
        builtins.match "[[:space:]]*//.*" line == null
        && (builtins.match ".*(${step}|${spacing}).*" line != null)
        && (
          raw
          || builtins.match ".*${divided}.*" line != null
          || builtins.match ".*${spacing}[[:space:]]*-?[1-9][0-9]*[[:space:]]*;?[[:space:]]*" line != null
        );
      culprits = lib.concatMap (
        name:
        map (line: "${name}: ${lib.trim line}") (
          lib.filter nudged (lib.splitString "\n" (builtins.readFile (dir + "/${name}")))
        )
      ) everyShellFile;
    in
    holds "quickshell-keeps-to-the-space-scale" ''
      These nudge a step, or set a padding, margin or spacing, by a number off the scale:
      ${lib.concatStringsSep "\n" culprits}'' (everyShellFile != [ ] && culprits == [ ]);

  # The shell follows dictation as its service says (tests/dictation-watch.py
  # stands in for it): each state said, a service that stops watched again
  # once its socket starts it, and a socket nothing listens on tried once,
  # never in a loop.
  the-desktop-follows-dictation =
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
    pkgs.runCommandLocal "the-desktop-follows-dictation" { } ''
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

  # clippy finds nothing in launcher-say, its tests included.
  launcher-say-passes-clippy = passesClippy "launcher-say-passes-clippy" (
    pkgs.callPackage ./launcher-say/package.nix { }
  );

  # A chord's program asks the loader for no library, so a chord on a
  # loaded machine reads only its one file, which launcher-say-resident
  # holds in memory (launcher-say/src/main.rs).
  a-chord-reads-only-its-own-program =
    let
      say = pkgs.callPackage ./launcher-say/package.nix { };
    in
    # Kept in Nix because it is one readelf call over the built program,
    # with nothing in it to test alone.
    pkgs.runCommandLocal "a-chord-reads-only-its-own-program" { } ''
      if ${pkgs.binutils}/bin/readelf -l ${lib.getExe say} | grep -q INTERP; then
        echo "launcher-say asks for a loader, so it reads libraries as it starts:" >&2
        ${pkgs.binutils}/bin/readelf -d ${lib.getExe say} >&2
        exit 1
      fi
      touch $out
    '';

  # clippy finds nothing in the window recorder, its tests included.
  record-window-passes-clippy = passesClippy "record-window-passes-clippy" (
    pkgs.callPackage ./record-window/package.nix { }
  );

  # clippy finds nothing in clock-follows, its tests included.
  clock-follows-passes-clippy = passesClippy "clock-follows-passes-clippy" (
    pkgs.callPackage ./clock-follows/package.nix { }
  );

  # launcher's tests run as it builds: what the shell shows is laid out to
  # the terminal's width, a count and then a side giving way by whole
  # items and a name cut only at a word, inked only where the terminal
  # takes colour, and a miss said in the launcher's words.
  launcher-fits-what-it-shows-to-the-terminal = pkgs.callPackage ./in-a-terminal/package.nix { };

  # clippy finds nothing in launcher, the launcher in a terminal, its tests
  # included.
  launcher-passes-clippy = passesClippy "launcher-passes-clippy" (
    pkgs.callPackage ./in-a-terminal/package.nix { }
  );

  # launcher completes in zsh, typed in a real line editor with the real
  # launcher and the completion it writes, asking a stand-in qs what each
  # place holds: its command, each row a step names as it's typed, the
  # rows inside the steps typed so far, flags anywhere, and nothing after a
  # step that names nothing (in-a-terminal/tests/launcher-completion.zsh).
  launcher-completes-in-zsh =
    let
      launcher = pkgs.callPackage ./in-a-terminal/package.nix { };
    in
    pkgs.runCommandLocal "launcher-completes-in-zsh"
      {
        nativeBuildInputs = [
          launcher
          pkgs.zsh
        ];
        LAUNCHER_QS = pkgs.writeShellScript "qs" (builtins.readFile ./in-a-terminal/tests/qs);
        LAUNCHER_WORDS = ./launcher/words.json;
      }
      ''
        export HOME=$PWD
        launcher completion zsh > completion.zsh
        zsh -f ${./in-a-terminal/tests/launcher-completion.zsh} $PWD/completion.zsh
        touch $out
      '';

  # clippy finds nothing in timekeeping, its tests included.
  timekeeping-passes-clippy = passesClippy "timekeeping-passes-clippy" (
    pkgs.callPackage ./timekeeping/package.nix { }
  );

  # The timers daemon's tests run as it builds: a timer started is told to
  # every watch and rings as a critical notice once its end passes, the
  # notice's Again sets it going afresh and putting it away ends it, ending
  # one rung takes its notice back, a ring with no server to take it is
  # said once one comes, what's kept outlasts the daemon, one that ended
  # meanwhile ringing late, and an ask it can't do is refused.
  timers-are-kept-and-rung = pkgs.callPackage ./timers/package.nix { };

  # clippy finds nothing in the timers daemon, its tests included.
  timers-passes-clippy = passesClippy "timers-passes-clippy" (
    pkgs.callPackage ./timers/package.nix { }
  );

  # clippy finds nothing in whereabouts, its tests included.
  whereabouts-passes-clippy = passesClippy "whereabouts-passes-clippy" (
    pkgs.callPackage ./whereabouts/package.nix { }
  );
}
