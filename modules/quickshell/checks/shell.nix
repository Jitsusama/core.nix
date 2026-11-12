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
  # The shell's QML is given an activation token by niri when it asks
  # (tests/activation-token-on-niri.sh), as the notices daemon will need it
  # to hand a program with an action chosen, having no window to ask with.
  the-shell-is-given-a-token-on-niri = shellOnNiri {
    name = "the-shell-is-given-a-token-on-niri";
    away = 3600;
    root = ../../../tests/activation-token.qml;
    command = "${lib.getExe pkgs.bash} ${../../../tests/activation-token-on-niri.sh} quickshell logs";
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

  # The shell holds a daemon's records over one connection (Daemon.qml),
  # against the real people daemon with two sources stood in for, one
  # asked (daemon-source.sh) and one saying everyone unasked
  # (daemon-roster.sh), started, stopped and started again by
  # tests/daemon.qml; and over one connection, records of two kinds from a
  # daemon holding both, and kinds told as the mycelium node tells them
  # (daemon-kinds.py); and a line's handlers read when it was heard
  # (Hearing.js). The stand-ins are in modules/quickshell/tests/stand-ins.
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
      cp ${../../../tests/daemon.qml} shell.qml
      export HOME=$TMPDIR XDG_RUNTIME_DIR=$PWD/runtime LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts} PEOPLE=${lib.getExe people} SOURCES=$PWD/sources.json
      export PYTHON=${lib.getExe pkgs.python3} KINDS=${../tests/stand-ins/daemon-kinds.py}
      ${shellFolderLint}
      timeout 90 ${qs} -p shell.qml
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
    cp ${../../../tests/ipc-replies.qml} shell.qml
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
        cp ${../../../tests/getters-keep-their-own-table.qml} shell.qml
        export HOME=$TMPDIR XDG_RUNTIME_DIR=$TMPDIR/runtime LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen
        ${shellFolderLint}
        timeout 60 ${qs} -p shell.qml
        touch $out
      '';

  # A Map and a Set mean what the language says, in the shell's own Qt,
  # whose tables find string and number keys by hash
  # (qtdeclarative-maps-find-keys-by-hash.patch).
  maps-keep-their-meaning = pkgs.runCommandLocal "maps-keep-their-meaning" { } ''
    mkdir runtime && chmod 700 runtime
    cp ${../../../tests/maps-keep-their-meaning.qml} shell.qml
    export HOME=$TMPDIR XDG_RUNTIME_DIR=$TMPDIR/runtime LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen
    ${shellFolderLint}
    timeout 60 ${qs} -p shell.qml
    touch $out
  '';

  # The shell's own files are compiled once and kept, and only a file that
  # changed is compiled again, though Quickshell serves them on its own qs:
  # scheme, which stock Qt never caches, and none of them waits for the disk
  # as it is written (tests/compiled-once.sh, over
  # qtdeclarative-caches-what-is-served.patch and
  # qtdeclarative-caches-without-syncing.patch). Kept in Nix because each
  # line sets up the sandbox or hands the script a path Nix gives it, and
  # the script holds the test.
  the-shell-compiles-once = pkgs.runCommandLocal "the-shell-compiles-once" { } ''
    mkdir runtime && chmod 700 runtime
    cp -r ${../../../tests/compiled-once} shell && chmod -R u+w shell
    export HOME=$TMPDIR XDG_RUNTIME_DIR=$TMPDIR/runtime LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen
    export FONTCONFIG_FILE=${noFonts}
    (cd shell && ${shellFolderLint})
    ${lib.getExe pkgs.bash} ${../../../tests/compiled-once.sh} ${qs} shell ${lib.getExe pkgs.strace}
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
        cp ${../../../tests/reloaded.qml} shell.qml
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
    cp ${../pieces/StatusFile.qml} StatusFile.qml
    cp ${../../../tests/watched.qml} shell.qml
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
        cp ${../../../tests/desktop-entries.qml} shell.qml
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
  # debug say (tests/shell-stays-light-on-niri-test.py).
  the-shell-stays-light-on-niri = launcherOnNiri {
    name = "the-shell-stays-light-on-niri";
    tests = ../../../tests/shell-stays-light-on-niri-test.py;
    given = [ "QS_GIVEN=--log-rules qt.qml.gc.allocatorStats.debug=true" ];
  };
}
