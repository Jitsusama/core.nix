# The checks of the launcher's Machine: the processor, memory, disk and the
# realm's other machines, which the folder's checks.nix gathers with every other
# place's; its opening comment says why each place keeps its own.
{
  account,
  graphical,
  lib,
  machine,
  noFonts,
  pkgs,
  qs,
  shellFolderLint,
  singleton,
  ...
}:
{
  # Machines says every machine of the realms this one links, this one
  # first, each in words with how it stands, and its drawer from what
  # moves most to the model's identifier, leaving out what no node said, as
  # tests/machines.qml holds launcher/RealmMachines.js to it in Qt's own engine.
  the-launcher-says-every-machine-of-the-realm =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "the-launcher-says-every-machine-of-the-realm" { } ''
      cp ${../../../tests/machines.qml} machines.qml
      cp ${../launcher/RealmMachines.js} RealmMachines.js
      cp ${../launcher/Order.js} Order.js
      cp ${../launcher/Machine.js} Machine.js
      cp ${../launcher/Numbers.js} Numbers.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml machines.qml
      touch $out
    '';

  # The disk piece hears the guards' status files as tests/disk.qml writes
  # them, through Quickshell itself and the machine daemon over a machine of
  # the check's own, with niri stood in for, and systemd too, on a session
  # bus and a system bus of the check's own (tests/systemd-units.py), so the
  # daemon asks a silent guard's unit as it does on a machine.
  the-disk-piece-reads-the-guards =
    let
      python = pkgs.python3.withPackages (python: [ python.jeepney ]);
      dbus = lib.getExe' pkgs.dbus "dbus-daemon";
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
      cp ${../Daemon.qml} Daemon.qml
      cp ${../Hearing.js} Hearing.js
      cp ${../Watchers.qml} Watchers.qml
      cp ${../FileFacts.js} FileFacts.js
      cp ${../When.js} When.js
      cp ${../Files.qml} Files.qml
      cp ${../pieces/Piece.qml} pieces/Piece.qml
      cp ${../pieces/Disk.qml} pieces/Disk.qml
      cp ${../pieces/Drawers.js} pieces/Drawers.js
      cp ${../Words.qml} Words.qml
      cp ${../launcher/words.json} launcher/words.json
      cp ${../../../tests/disk.qml} shell.qml
      cp ${niri} Niri.qml
      cp ${(account graphical).xdg.configFile."quickshell/Theme.qml".source} Theme.qml
      export HOME=$TMPDIR XDG_RUNTIME_DIR=$TMPDIR/runtime LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen
      export MACHINE_ROOT=$TMPDIR/root FONTCONFIG_FILE=${noFonts}
      # Each bus is up as dbus-daemon forks, and the stand-in owns systemd's
      # name on both before it writes its fifo.
      for bus in session system; do
        ${dbus} --config-file=${pkgs.dbus}/share/dbus-1/session.conf \
          --address="unix:path=$TMPDIR/$bus-bus" --fork --print-pid=3 3>> "$TMPDIR/buses"
      done
      export DBUS_SESSION_BUS_ADDRESS="unix:path=$TMPDIR/session-bus"
      export DBUS_SYSTEM_BUS_ADDRESS="unix:path=$TMPDIR/system-bus"
      mkfifo "$TMPDIR/ready"
      ${lib.getExe python} ${../tests/systemd-units.py} "$TMPDIR/ready" \
        "$DBUS_SESSION_BUS_ADDRESS" "$XDG_RUNTIME_DIR/account-unit" \
        "$DBUS_SYSTEM_BUS_ADDRESS" "$XDG_RUNTIME_DIR/machine-unit" &
      units=$!
      read -r _ < "$TMPDIR/ready"
      XDG_RUNTIME_DIR=/run/user/1000 \
        ${lib.getExe machine} "$TMPDIR/runtime/daemons/machine.sock" "$MACHINE_ROOT" &
      daemon=$!
      ${shellFolderLint}
      timeout 60 ${qs} -p shell.qml
      kill "$daemon" "$units" $(< "$TMPDIR/buses")
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
        cp ${../Daemon.qml} Daemon.qml
        cp ${../Hearing.js} Hearing.js
        cp ${../Watchers.qml} Watchers.qml
        cp ${../Fuzzy.js} Fuzzy.js
        cp ${../launcher/Provider.qml} launcher/Provider.qml
        cp ${../ProgramGlyphs.js} ProgramGlyphs.js
        cp ${../launcher/Asking.js} launcher/Asking.js
        cp ${../launcher/Numbers.js} launcher/Numbers.js
        cp ${../launcher/Waits.qml} launcher/Waits.qml
        cp ${../Words.qml} Words.qml
        cp ${../launcher/words.json} launcher/words.json
        cp ${../../../tests/waits.qml} shell.qml
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
        cp ${../Daemon.qml} Daemon.qml
        cp ${../Hearing.js} Hearing.js
        cp ${../Fuzzy.js} Fuzzy.js
        cp ${../launcher/Provider.qml} launcher/Provider.qml
        cp ${../ProgramGlyphs.js} ProgramGlyphs.js
        cp ${../launcher/Asking.js} launcher/Asking.js
        cp ${../launcher/Numbers.js} launcher/Numbers.js
        cp ${../launcher/Interfaces.js} launcher/Interfaces.js
        cp ${../launcher/Machine.qml} launcher/Machine.qml
        cp ${../launcher/ReadyRows.qml} launcher/ReadyRows.qml
        cp ${../launcher/ReadyRows.js} launcher/ReadyRows.js
        cp ${../launcher/Ago.js} launcher/Ago.js
        cp ${../When.js} When.js
        cp ${../launcher/Order.js} launcher/Order.js
        cp ${../launcher/HomeNetwork.js} launcher/HomeNetwork.js
        cp ${../launcher/Holders.js} launcher/Holders.js
        cp ${../launcher/SessionRow.js} launcher/SessionRow.js
        cp ${(account graphical).xdg.configFile."quickshell/Theme.qml".source} Theme.qml
        cp ${../Words.qml} Words.qml
        cp ${../launcher/words.json} launcher/words.json
        cp ${singleton "Jobs.qml" "property var said: null"} Jobs.qml
        cp ${singleton "Niri.qml" "function act(action) {}"} Niri.qml
        cp ${singleton "Facts.qml" "property var all: []"} Facts.qml
        cp ${../../../tests/machine.qml} shell.qml
        export HOME=$TMPDIR XDG_RUNTIME_DIR=$TMPDIR/runtime LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen
        export FONTCONFIG_FILE=${noFonts}
        ${lib.getExe machine} "$XDG_RUNTIME_DIR/daemons/machine.sock" \
          --words ${../launcher/words.json} &
        daemon=$!
        ${shellFolderLint}
        timeout 60 ${qs} -p shell.qml
        kill "$daemon"
        touch $out
      '';
}
