# The checks of the shell as a whole: how it is built, linted and installed, and
# the patches Quickshell carries, which the folder's checks.nix gathers with
# every other place's; its opening comment says why each place keeps its own.
{
  account,
  copied,
  graphical,
  holds,
  launcherOnNiri,
  lib,
  noFonts,
  pkgs,
  qs,
  quickshellTypes,
  shellAsGiven,
  shellFolderLint,
  shellOnNiri,
  ...
}:
{
  # A collection's row made again from a row made keeps every field it's given,
  # as modules/quickshell/tests/member.qml holds launcher/Member.js to, in Qt's
  # engine.
  a-row-made-again-keeps-every-field =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    # Kept in Nix because each line copies a file Nix gives it beside the
    # cases, or sets what Qt's engine runs them with.
    pkgs.runCommandLocal "a-row-made-again-keeps-every-field" { } ''
      mkdir launcher
      cp ${../tests/member.qml} member.qml
      ${copied [ "launcher/Member.js" ]}
      cp ${../../../tests/lib/Expect.js} Expect.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml member.qml
      touch $out
    '';

  # The shell's QML is given an activation token by niri when it asks
  # (modules/quickshell/tests/activation-token-on-niri.sh), as the notices
  # daemon will need it to hand a program with an action chosen, having no
  # window to ask with.
  the-shell-is-given-a-token-on-niri = shellOnNiri {
    name = "the-shell-is-given-a-token-on-niri";
    away = 3600;
    root = ../tests/activation-token.qml;
    command = "${lib.getExe pkgs.bash} ${../tests/activation-token-on-niri.sh} quickshell logs";
  };

  # The account's whole shell, a notice up, loses its only screen and is
  # given one back, and Quickshell never warns of a dangling screen, on
  # sway, whose outputs can go
  # (modules/quickshell/tests/shell-loses-its-screen.sh).
  the-shell-warns-of-no-dangling-screen-as-its-screen-goes = shellOnNiri {
    name = "the-shell-warns-of-no-dangling-screen-as-its-screen-goes";
    # Away only after an hour, so the notices are shown as they come.
    away = 3600;
    inputs = [ pkgs.libnotify ];
    command = "${lib.getExe pkgs.bash} ${../tests/shell-loses-its-screen.sh} quickshell logs ${../../../tests/lib/shell-says.sh}";
  };

  # A window drawn flat holds no depth or stencil buffer, and still draws what's
  # set over its picture over it, in the Quickshell the machine runs
  # (window-drawn-flat.patch,
  # modules/quickshell/tests/flat-windows-hold-no-depth-on-niri.sh), on a screen
  # of Joel's size.
  flat-windows-hold-no-depth-on-niri = shellOnNiri {
    name = "flat-windows-hold-no-depth-on-niri";
    away = 3600;
    size = "2880x1800";
    root = ../tests/flat-windows-hold-no-depth.qml;
    inputs = [ pkgs.imagemagick ];
    command = "${lib.getExe pkgs.bash} ${../tests/flat-windows-hold-no-depth-on-niri.sh} quickshell logs ${../../../tests/lib/Memory.js}";
  };

  # The four windows the shell keeps mapped beside the launcher, listed here by
  # hand, are drawn flat, so none holds a depth and stencil buffer as large as
  # itself the whole day, which flat-windows-hold-no-depth-on-niri shows a flat
  # window does without, drawing what these draw the same. A window kept up
  # later is added to the list. The launcher keeps its own buffers, since its
  # first openings were slower drawn flat. Each file holds one window, so its
  # one line is that window's. So it can't pass by reading nothing or a near
  # miss, a window without the line, one set false, one commented out and a
  # second window in the file are each shown to fail first. Kept in Nix
  # because the windows it reads are the files Nix gives it.
  the-windows-always-up-are-drawn-flat =
    let
      # Each by its name, a file of its own in the store, which the build can
      # read.
      windows = lib.genAttrs [ "Margin" "NoticeLine" "TalkLine" "Wallpaper" ] (
        name: ../. + "/${name}.qml"
      );
    in
    pkgs.runCommandLocal "the-windows-always-up-are-drawn-flat" { } ''
      flat() {
        [ "$(grep -c 'Window {' "$1")" = 1 ] && grep -q '^ *surfaceFormat\.flat: true$' "$1"
      }
      unflat() {
        printf '%b' "$1" > unflat.qml
        if flat unflat.qml; then
          echo "A window was read as flat that isn't: $1"
          exit 1
        fi
      }
      unflat 'PanelWindow {\n    color: "transparent"\n}\n'
      unflat 'PanelWindow {\n    surfaceFormat.flat: false\n}\n'
      unflat 'PanelWindow {\n    // surfaceFormat.flat: true\n}\n'
      unflat 'PanelWindow {\n    surfaceFormat.flat: true\n}\nPopupWindow {\n}\n'
      printf 'PanelWindow {\n    surfaceFormat.flat: true\n}\n' > one.qml
      if ! flat one.qml; then
        echo "A window drawn flat wasn't read as flat"
        exit 1
      fi
      checked=0
      ${lib.concatStrings (
        lib.mapAttrsToList (name: file: ''
          if ! flat ${file}; then
            echo "${name}.qml isn't drawn flat, or holds more than one window"
            exit 1
          fi
          checked=$((checked + 1))
        '') windows
      )}
      if [ "$checked" != ${toString (lib.length (lib.attrNames windows))} ]; then
        echo "Only $checked windows were read"
        exit 1
      fi
      touch $out
    '';

  # A picture read uncached from a file, as the wallpaper's is, lets the file's
  # pixels go once it's a texture, in the shell's own Qt
  # (qtdeclarative-uncached-pictures-let-go.patch), and is read again for a
  # window it moves to
  # (modules/quickshell/tests/uncached-pictures-let-go-on-niri.sh).
  uncached-pictures-let-go-on-niri = shellOnNiri {
    name = "uncached-pictures-let-go-on-niri";
    away = 3600;
    root = ../tests/uncached-pictures-let-go.qml;
    inputs = [ pkgs.imagemagick ];
    command = "${lib.getExe pkgs.bash} ${../tests/uncached-pictures-let-go-on-niri.sh} quickshell logs";
  };

  # The wallpaper's picture is read at its screen's own pixels, by niri's scale
  # for the output, as modules/quickshell/tests/wallpaper.qml holds Wallpaper.js
  # to, in Qt's engine.
  the-wallpaper-is-read-at-its-screens-pixels =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    # Kept in Nix because each line copies a file Nix gives it beside the
    # cases, or sets what Qt's engine runs them with.
    pkgs.runCommandLocal "the-wallpaper-is-read-at-its-screens-pixels" { } ''
      cp ${../tests/wallpaper.qml} wallpaper.qml
      ${copied [ "Wallpaper.js" ]}
      cp ${../../../tests/lib/Expect.js} Expect.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml wallpaper.qml
      touch $out
    '';

  # The wallpaper's picture follows its screen as it goes and comes back,
  # read once each time and without a binding loop, in the Quickshell the
  # machine runs, as modules/quickshell/tests/screen-picture/shell.qml
  # holds ScreenPicture.qml to, with a niri that never answers and a
  # picture of 1 by 1.
  screen-picture-follows-its-screen =
    # Kept in Nix because each line copies a file Nix gives it beside the
    # case, or sets what Quickshell runs it with.
    pkgs.runCommandLocal "screen-picture-follows-its-screen" { } ''
      mkdir runtime && chmod 700 runtime
      cp ${../tests/screen-picture/shell.qml} shell.qml
      cp ${../tests/screen-picture/Niri.qml} Niri.qml
      ${copied [ "ScreenPicture.qml" ]}
      base64 -d > picture.png <<< \
        iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==
      export HOME=$TMPDIR XDG_RUNTIME_DIR=$TMPDIR/runtime LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen
      ${shellFolderLint}
      timeout 60 ${qs} -p shell.qml > log 2>&1 || { cat log; exit 1; }
      cat log
      if grep -q 'WARN' log; then
        echo "the picture warned as its screen came and went"; exit 1
      fi
      touch $out
    '';

  # Quickshell is restarted when any file it reads changes, so the shell on
  # screen is never the one from before the switch for long: the switch
  # leaves it running, and every switch starts its companion, which brings
  # it onto the files once the launcher is idle.
  quickshell-restarts-with-its-files =
    let
      home = account graphical;
      services = home.systemd.user.services;
      files = lib.filterAttrs (name: _: lib.hasPrefix "quickshell/" name) home.xdg.configFile;
      triggers = services.quickshell.Unit.X-Restart-Triggers or [ ];
      takesNewFiles = services.quickshell-takes-new-files.Service.ExecStart or [ ];
      companion = lib.concatStringsSep " " takesNewFiles;
      reload = lib.concatStringsSep " " (lib.toList (services.quickshell.Service.ExecReload or [ ]));
      # The list the companion compares with, which a handover links as a start does, or every
      # switch after would hand over again with nothing changed.
      # Compared as text, since a search can't carry what the paths depend on.
      list = builtins.unsafeDiscardStringContext (lib.last (lib.splitString " " companion));
      reloadSaid = builtins.unsafeDiscardStringContext reload;
    in
    holds "quickshell-restarts-with-its-files"
      "A Quickshell file isn't among its unit's triggers, the switch restarts the shell itself, nothing brings it onto its new files after, or its reload doesn't hand it over and link its files."
      (
        files != { }
        && lib.all (file: lib.elem "${file.source}" triggers) (lib.attrValues files)
        && (services.quickshell.Unit.X-SwitchMethod or null) == "keep-old"
        && lib.hasInfix "takes-new-files.sh" companion
        && lib.hasInfix "quickshell-hands-over" reload
        && lib.hasInfix "ln -sfn ${list} %t/quickshell-files" reloadSaid
        && home.home.activation ? quickshellTakesNewFiles
      );

  # The companion takes a switch whose shell differs from the one running by
  # the list each start links, so a new Quickshell or a new setting in its
  # environment has to change the list too: a switch bringing a new
  # Quickshell with the same files left the shell on the old build
  # (2026-11-14, 04:23).
  the-shells-list-names-what-it-runs =
    let
      services = (account graphical).systemd.user.services;
      companion = lib.toList (services.quickshell-takes-new-files.Service.ExecStart or [ ]);
      list = lib.last (lib.splitString " " (lib.concatStringsSep " " companion));
      shell = services.quickshell.Service;
      runs = lib.toList (shell.ExecStart or [ ]) ++ lib.toList (shell.Environment or [ ]);
    in
    # Kept in Nix because the lines it looks for are the unit's own, which only
    # Nix holds, and a script beside it would take them all as arguments anyway.
    pkgs.runCommandLocal "the-shells-list-names-what-it-runs" { } ''
      for line in ${lib.escapeShellArgs runs}; do
        if ! grep -qxF -- "$line" ${list}; then
          echo "the list the companion compares doesn't name what the shell runs: $line"
          exit 1
        fi
      done
      touch $out
    '';

  # A switch leaves the shell running and its companion hands it over onto the
  # new files once the launcher has been shut 30 s, or at its bound however
  # busy the launcher is, restarting it only if the handover refuses, and
  # leaves a shell already on its files, or one stopped, be
  # (modules/quickshell/tests/takes-new-files.sh).
  the-shell-takes-a-switchs-files-at-idle =
    pkgs.runCommandLocal "the-shell-takes-a-switchs-files-at-idle" { }
      ''
        ${lib.getExe' pkgs.coreutils "timeout"} 60 ${lib.getExe pkgs.bash} \
          ${../tests/takes-new-files.sh} ${../takes-new-files.sh}
        touch $out
      '';

  # The handover waits for the old shell's launcher and Super+S's line to be shut before
  # ending it, a chord heard just after they were found shut included, and one sent before
  # but read late by a busy shell (../tests/late-chord.py), restarts instead on
  # a locked screen, and leaves the old shell the unit's when
  # the new one fails as it loads (modules/quickshell/tests/hands-over.sh).
  the-shell-hands-over-only-once-its-surfaces-are-shut =
    pkgs.runCommandLocal "the-shell-hands-over-only-once-its-surfaces-are-shut"
      {
        nativeBuildInputs = [
          pkgs.jq
          pkgs.procps
          pkgs.python3
          pkgs.util-linux
        ];
      }
      ''
        ${lib.getExe' pkgs.coreutils "timeout"} 120 ${lib.getExe pkgs.bash} \
          ${../tests/hands-over.sh} ${../shell-hands-over.sh} ${../tests/late-chord.py}
        touch $out
      '';

  # The shell's allocator keeps one arena rather than one for each of its
  # threads, since each arena holds freed memory of its own that it hands
  # back only slowly (the unit's Environment says how much it saves).
  quickshell-allocates-from-one-arena =
    let
      home = account graphical;
      environment = home.systemd.user.services.quickshell.Service.Environment or [ ];
    in
    holds "quickshell-allocates-from-one-arena"
      "Quickshell's unit doesn't hold jemalloc to one arena, so its threads keep freed memory apart."
      (lib.elem "MALLOC_CONF=narenas:1" (lib.toList environment));

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
      ${lib.getExe pkgs.bash} ${../tests/qml-compiles.sh} ${qt}/libexec/qmlcachegen qs
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
      lint = lib.getExe (pkgs.callPackage ../../../tests/lib/qmllint-counted.nix { });
    in
    # In UTF-8, which Qt otherwise switches to and says so.
    pkgs.runCommandLocal "quickshell-passes-qmllint" { LC_ALL = "C.UTF-8"; } ''
      ${shellAsGiven}
      # Quickshell finds a folder's files as a module by its path; qmllint
      # needs a qmldir saying so (modules/quickshell/tests/qmldir.sh).
      ${lib.getExe pkgs.bash} ${../tests/qmldir.sh} qs
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
        ${lib.getExe pkgs.bash} ${../tests/imports-installed.sh} qs
        touch $out
      '';

  # The shell hands every program it opens to niri, which gives it a scope of
  # its own. One the shell started itself would leave its helpers in the
  # shell's service, and restarting the shell would kill them.
  quickshell-opens-programs-through-niri =
    let
      dir = ../.;
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

  # The shell holds a daemon's records over one connection (Daemon.qml), against
  # the real people daemon with two sources stood in for, one asked
  # (daemon-source.sh) and one saying everyone unasked (daemon-roster.sh),
  # started, stopped and started again by modules/quickshell/tests/daemon.qml; and
  # over one connection, records of two kinds from a daemon holding both, kinds
  # told as the mycelium node tells them, and a watch answered whole of only some
  # keys, as the passwords child answers its asks' rows (daemon-kinds.py); and a
  # line's handlers read when it was heard (Hearing.js). The stand-ins are in
  # modules/quickshell/tests/stand-ins.
  the-desktop-holds-a-daemons-records =
    let
      people = pkgs.callPackage ../../people/package.nix { };
    in
    pkgs.runCommandLocal "the-desktop-holds-a-daemons-records" { } ''
      mkdir -p runtime/daemons && chmod 700 runtime
      cat > sources.json <<EOF
      {"sources": [{"name": "slack",
        "command": ["${lib.getExe pkgs.bash}", "${../tests/stand-ins/daemon-source.sh}"]},
        {"name": "tuple",
        "command": ["${lib.getExe pkgs.bash}", "${../tests/stand-ins/daemon-roster.sh}"]}]}
      EOF
      ${copied [ "Daemon.qml" ]}
      cp ${../tests/daemon.qml} shell.qml
      export HOME=$TMPDIR XDG_RUNTIME_DIR=$PWD/runtime LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts} PEOPLE=${lib.getExe people} SOURCES=$PWD/sources.json
      export PYTHON=${lib.getExe pkgs.python3} KINDS=${../tests/stand-ins/daemon-kinds.py}
      ${shellFolderLint}
      timeout 90 ${qs} -p shell.qml
      touch $out
    '';

  # A change saying only what differs mends the record a daemon's client holds,
  # a field said deep only where it moved inside it, as
  # modules/quickshell/tests/mending.qml holds Mending.js to it in Qt's own
  # engine.
  a-change-mends-only-what-moved =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "a-change-mends-only-what-moved" { } ''
      cp ${../tests/mending.qml} mending.qml
      cp ${../../../tests/lib/Expect.js} Expect.js
      ${copied [ "Mending.js" ]}
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml mending.qml
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
    cp ${../tests/ipc-replies.qml} shell.qml
    export HOME=$TMPDIR XDG_RUNTIME_DIR=$TMPDIR/runtime LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen
    export FONTCONFIG_FILE=${noFonts} QS=${qs} PATH=${pkgs.coreutils}/bin:$PATH
    ${shellFolderLint}
    timeout 60 ${qs} -p shell.qml
    touch $out
  '';

  # A field turned into a getter in one object leaves every other object's
  # fields whole, in the Qt the shell is built with
  # (qtdeclarative-accessor-keeps-its-own-table.patch): in Qt 6.12.0 the
  # second of two such turns, in shapes grown one from the other, put its
  # setter over a field, which crashed the shell on 2026-11-11. Run in qs,
  # so it's the shell's own engine that's judged.
  a-getter-turned-leaves-other-shapes-whole =
    pkgs.runCommandLocal "a-getter-turned-leaves-other-shapes-whole" { }
      ''
        mkdir runtime && chmod 700 runtime
        cp ${../tests/getters-keep-their-own-table.qml} shell.qml
        export HOME=$TMPDIR XDG_RUNTIME_DIR=$TMPDIR/runtime LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen
        ${shellFolderLint}
        timeout 60 ${qs} -p shell.qml
        touch $out
      '';

  # Date.parse reads what it read, in the shell's own Qt, which refuses a
  # string without a digit at once rather than after trying every form
  # (qtdeclarative-dates-without-digits-refused.patch). In UTC, so a form
  # read as local time reads the moment the case names.
  dates-read-as-before = pkgs.runCommandLocal "dates-read-as-before" { } ''
    mkdir runtime && chmod 700 runtime
    cp ${../tests/dates-read-as-before.qml} shell.qml
    export HOME=$TMPDIR XDG_RUNTIME_DIR=$TMPDIR/runtime LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen TZ=UTC
    ${shellFolderLint}
    timeout 60 ${qs} -p shell.qml
    touch $out
  '';

  # JSON.parse keeps each string value once per parse, in the shell's own
  # Qt, and still means what it did (qtdeclarative-json-strings-kept-once.patch):
  # the daemons' answers say the same words thousands of times, and a
  # string apiece held twice what one for each word does.
  json-strings-kept-once = pkgs.runCommandLocal "json-strings-kept-once" { } ''
    mkdir runtime && chmod 700 runtime
    cp ${../tests/json-strings-kept-once.qml} shell.qml
    cp ${../../../tests/lib/Memory.js} Memory.js
    export HOME=$TMPDIR XDG_RUNTIME_DIR=$TMPDIR/runtime LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen
    export QML_XHR_ALLOW_FILE_READ=1
    ${shellFolderLint}
    timeout 60 ${qs} -p shell.qml
    touch $out
  '';

  # JSON.parse holds each string value at its own length, in the shell's own
  # Qt (qtdeclarative-json-strings-at-their-length.patch): Qt 6.12.0 grew each
  # one a character at a time and kept the room it grew to, up to twice what it
  # needs, for every string the daemons' answers hold.
  json-strings-at-their-length = pkgs.runCommandLocal "json-strings-at-their-length" { } ''
    mkdir runtime && chmod 700 runtime
    cp ${../tests/json-strings-at-their-length.qml} shell.qml
    cp ${../../../tests/lib/Memory.js} Memory.js
    export HOME=$TMPDIR XDG_RUNTIME_DIR=$TMPDIR/runtime LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen
    export QML_XHR_ALLOW_FILE_READ=1
    ${shellFolderLint}
    timeout 60 ${qs} -p shell.qml
    touch $out
  '';

  # One long call keeps the heap near what it keeps, in the shell's own Qt
  # (qtdeclarative-collector-keeps-up.patch): Qt 6.12.0 stepped a collection
  # only from the event loop, so the shell's hearing of its Emoji at the start
  # left the heap 30 MB heavier for the rest of its run.
  the-collector-keeps-up = pkgs.runCommandLocal "the-collector-keeps-up" { } ''
    mkdir runtime && chmod 700 runtime
    cp ${../tests/collector-keeps-up.qml} shell.qml
    cp ${../../../tests/lib/Memory.js} Memory.js
    export HOME=$TMPDIR XDG_RUNTIME_DIR=$TMPDIR/runtime LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen
    export QML_XHR_ALLOW_FILE_READ=1
    ${shellFolderLint}
    timeout 60 ${qs} -p shell.qml
    touch $out
  '';

  # A Map and a Set mean what the language says, in the shell's own Qt,
  # whose tables find string and number keys by hash
  # (qtdeclarative-maps-find-keys-by-hash.patch).
  maps-keep-their-meaning = pkgs.runCommandLocal "maps-keep-their-meaning" { } ''
    mkdir runtime && chmod 700 runtime
    cp ${../tests/maps-keep-their-meaning.qml} shell.qml
    export HOME=$TMPDIR XDG_RUNTIME_DIR=$TMPDIR/runtime LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen
    ${shellFolderLint}
    timeout 60 ${qs} -p shell.qml
    touch $out
  '';

  # The shell's own files are compiled once and kept, and only a file that
  # changed is compiled again, though Quickshell serves them on its own qs:
  # scheme, which stock Qt never caches, and none of them waits for the disk
  # as it is written (modules/quickshell/tests/compiled-once.sh, over
  # qtdeclarative-caches-what-is-served.patch and
  # qtdeclarative-caches-without-syncing.patch). Kept in Nix because each
  # line sets up the sandbox or hands the script a path Nix gives it, and
  # the script holds the test.
  the-shell-compiles-once = pkgs.runCommandLocal "the-shell-compiles-once" { } ''
    mkdir runtime && chmod 700 runtime
    cp -r ${../tests/compiled-once} shell && chmod -R u+w shell
    export HOME=$TMPDIR XDG_RUNTIME_DIR=$TMPDIR/runtime LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen
    export FONTCONFIG_FILE=${noFonts}
    (cd shell && ${shellFolderLint})
    ${lib.getExe pkgs.bash} ${../tests/compiled-once.sh} ${qs} shell ${lib.getExe pkgs.strace}
    touch $out
  '';

  # A file reloaded while it's still being read is read once more as that read
  # lands, so the view holds what was written last, as
  # modules/quickshell/tests/reloaded.qml drives it in the Quickshell the
  # machine runs (fileview-reads-again.patch): a status file taken away mid-read
  # stayed said, as the-desktop-reads-the-spend-watch once saw on a busy
  # builder.
  a-file-reloaded-mid-read-is-read-again =
    pkgs.runCommandLocal "a-file-reloaded-mid-read-is-read-again" { }
      ''
        mkdir runtime && chmod 700 runtime
        cp ${../tests/reloaded.qml} shell.qml
        export HOME=$TMPDIR XDG_RUNTIME_DIR=$TMPDIR/runtime LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen
        export FONTCONFIG_FILE=${noFonts}
        ${shellFolderLint}
        timeout 60 ${qs} -p shell.qml
        touch $out
      '';

  # A status file is heard again once its folder is made again, and when its
  # folder is made only after the shell starts, as
  # modules/quickshell/tests/watched.qml drives it, in the Quickshell the
  # machine runs (fileview-watch-nearest.patch): a guard's runtime folder goes
  # each time systemd restarts the guard. A view of the folder itself hears a
  # file renamed into it or removed (fileview-hears-a-folder.patch). A view
  # moved to another file, or watching again, keeps the one watcher it made
  # (fileview-keeps-its-watcher.patch), counted from Quickshell's log: each one
  # dropped cost the shell's thread a millisecond, 130 of them as it started.
  # Kept in Nix because each line copies a file Nix gives it beside the shell,
  # or runs the Quickshell it builds.
  a-watched-file-comes-back = pkgs.runCommandLocal "a-watched-file-comes-back" { } ''
    mkdir runtime && chmod 700 runtime
    cp ${../pieces/StatusFile.qml} StatusFile.qml
    cp ${../tests/watched.qml} shell.qml
    export HOME=$TMPDIR XDG_RUNTIME_DIR=$TMPDIR/runtime LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen
    export FONTCONFIG_FILE=${noFonts}
    ${shellFolderLint}
    timeout 60 ${qs} -p shell.qml --log-rules 'quickshell.io.fileview.debug=true' \
      > said 2>&1 || { cat said; exit 1; }
    grep -av 'quickshell.io.fileview' said || true
    made=$(grep -c 'Creating watcher for' said || true)
    [ "$made" = 3 ] || {
      echo "$made watchers were made for the 3 views that watch"
      exit 1
    }
    touch $out
  '';

  # The desktop entries are read again only when a folder holding them changes,
  # as modules/quickshell/tests/desktop-entries.qml drives them in the
  # Quickshell the machine runs (desktop-entries-heard-only-when-theirs.patch):
  # a file written beside them, as a hidden file in the home folder is, reads
  # nothing, while an entry added and a link on the way pointed elsewhere each
  # read them once, counted from Quickshell's log of each scan.
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
        cp ${../tests/desktop-entries.qml} shell.qml
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

  # Every place opened again, round after round, keeps no more of the
  # shell's JavaScript heap live than the round before, as Qt's collector
  # says at each closing's collection, which only its allocatorStats at
  # debug say (modules/quickshell/tests/shell-stays-light-on-niri-test.py).
  the-shell-stays-light-on-niri = launcherOnNiri {
    name = "the-shell-stays-light-on-niri";
    tests = ../tests/shell-stays-light-on-niri-test.py;
    given = [ "QS_GIVEN=--log-rules qt.qml.gc.allocatorStats.debug=true" ];
  };

  # A meter's moment lasts from its first frame shown, or the last change once
  # seen, and at most so long unseen, as modules/quickshell/tests/meter-moment.qml
  # holds Moment.qml to, in Qt's engine.
  a-meter-stays-its-moment-once-seen =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    # Kept in Nix because each line copies a file Nix gives it beside the
    # cases, or sets what Qt's engine runs them with.
    pkgs.runCommandLocal "a-meter-stays-its-moment-once-seen" { } ''
      cp ${../tests/meter-moment.qml} meter-moment.qml
      ${copied [ "Moment.qml" ]}
      cp ${../../../tests/lib/Expect.js} Expect.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml meter-moment.qml
      touch $out
    '';

  # A meter picked while niri can't draw is drawn once it can, and stays its
  # moment from then (modules/quickshell/tests/meter-on-niri-test.py).
  a-meter-stays-its-moment-once-drawn-on-niri = launcherOnNiri {
    name = "a-meter-stays-its-moment-once-drawn-on-niri";
    tests = ../tests/meter-on-niri-test.py;
  };
}
