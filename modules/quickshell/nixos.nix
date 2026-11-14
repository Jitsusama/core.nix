# What Quickshell needs from the system: a PAM service of its own for the
# lock screen, which checks the account's password and nothing else (its
# polkit prompt goes through polkit's own service), a watcher that says
# whether caps lock is on, which nothing else tells it, GeoClue, for where
# the laptop is, and its build.
#
# Quickshell 0.3.1's notification server never sends a program the
# activation token a compositor asks before it lets the program bring its
# window forward, so a notice's action reached the program but couldn't
# take Joel to the conversation or tab it was about. The patch
# (activation-token.patch says how) asks the compositor for one tied to the
# click or key that chose the action, and sends it first, as version 1.2 of
# the notifications specification has it. It lets the shell's QML ask for
# one too, to hand a notices server of its own that has no window to ask
# with. A FileView told not to print
# errors still warned of a file that opened and then refused to be read,
# as sysfs files do, so fileview-quiet.patch keeps it to its word. A
# FileView watching a file lost it for good once the file's directory went
# or if it wasn't there yet, as a service's runtime directory is each time
# systemd restarts the service, so fileview-watch-nearest.patch watches the
# nearest directory that is. A FileView reloaded while it was still reading
# the file started no read of its own, so a change heard mid-read was lost
# until the file changed again, and a status file taken away stayed said;
# fileview-reads-again.patch reads once more as that read lands. A FileView
# of a folder never said it changed, so the launcher's looks each held a
# watch of their own, and an open file, 406 of them past the shell's limit
# as it started; fileview-hears-a-folder.patch says a folder changed as a
# file in it is made, renamed in or removed. A FileView made a watcher anew
# each time its path or watching changed, and closing the one dropped held
# the shell's thread a millisecond, 130 times as it started, so
# fileview-keeps-its-watcher.patch keeps one and moves what it watches.
# An IPC call's connection was deleted as soon
# as its reply was handed over, cutting off a reply longer than the
# socket's buffer, so ipc-replies-whole.patch closes it only once the reply
# is written. The desktop entries were read again whenever anything changed
# in a folder above the ones holding them, as the home folder does with
# every hidden file written, so desktop-entries-heard-only-when-theirs.patch
# hears a folder above only for the entry leading down to them. A sound
# node said neither what pipewire says it's doing nor the format it moves
# audio in, which a device's drawer shows, so
# pipewire-node-state-and-format.patch reads both from the node's info and
# its Format param, which opens no stream. A frame Qt drew isn't one niri
# showed, and Qt binds no presentation-time to say when niri did, so
# frame-presentation.patch adds FramePresentation, which asks niri for each
# frame's presentation feedback and says when it was shown: the launcher
# times Joel's every opening to it (Launcher.qml). It times each frame's
# sync, render and swap on the render thread too, since the GUI thread
# hears a swap only once it's free. Each start read every line of every
# QML file, 12 percent of the start's main thread under callgrind
# (2026-11-13), though past a file's header only a preprocessor block
# matters, so scanner-reads-headers.patch stops at the header of a file
# that has none. All twelve are laid over the machine's packages, so the
# shell, the greeter and the lock all run them.
#
# wf-recorder, which records the screen (Recorder.qml), bound the
# compositor's linux-dmabuf at version 4 whatever it offered, a protocol
# error where it offers less, as niri nested in a window does;
# wf-recorder-dmabuf-version.patch binds what's offered, so the VM tests can
# record the screen too. Where 4 is offered, as on a machine's own screen,
# it does as before. Stopped over a still screen, it went on until something
# moved, up to a minute, since its wait for a frame heard no signal;
# wf-recorder-stops-at-once.patch has that wait give way to one at once, and
# the stop keep every frame taken and one last of the screen as it stopped.
#
# The launcher works out sums and measures with libqalculate, in the shell's
# own process rather than a calculator program started for each key, as a
# QML module of its own (maths/), built with the Qt Quickshell is built
# with. Quickshell's wrapper puts it on the shell's QML import path, so
# every shell the machine runs can import Maths.
#
# The watcher (keyboard-lights.rs) reads the keyboards, which is what a
# keylogger needs, so it gets nothing else: an account of its own, the input
# group, read access to input devices alone, no network and a read-only
# system but for its directory. It asks the kernel for the caps lock light
# and nothing more, so a keystroke never reaches it. It says too which
# keyboards it holds, so the shell hears one plugged in or out at once and
# shows only those connected (Keyboards.qml).
#
# Its directory is made at boot and stays until shutdown. A runtime
# directory of systemd's would go each time the watcher restarted, taking
# the file with it, and one it keeps, with an account it makes, moves where
# Quickshell can't read.
#
# GeoClue says where the laptop is, so the launcher's weather for here
# follows it (whereabouts/, which home.nix runs). It finds the laptop from
# the Wi-Fi networks in reach, through BeaconDB, since Mozilla's location
# service, GeoClue's old default, has shut down. whereabouts is allowed
# without asking, as a browser is: it only ever asks to a city's accuracy,
# and what it learns stays in the account's runtime directory, leaving
# only to a hundredth of a degree: to Nominatim, for its town, and to
# Open-Meteo, for its zone and its weather.
{ pkgs, ... }:
let
  # The Qt the shell is built with: nixpkgs' own, its JavaScript engine
  # patched. Qt 6.12.0's turns a field into a getter by writing where the
  # getter's setter lives into a table other objects' shapes share, so a
  # later such turn in a longer shape stored its setter over one of its
  # fields, or past the object's end, which crashed the shell on
  # 2026-11-11 (qtdeclarative-accessor-keeps-its-own-table.patch). The
  # three fixes to its collector that Qt's 6.12 branch has since 6.12.0
  # come with it, and a Map or a Set finds a string or a number key by
  # its hash rather than comparing it with every key it holds
  # (qtdeclarative-maps-find-keys-by-hash.patch). Quickshell serves the
  # shell's folder on its own qs: scheme, which stock Qt never keeps
  # compiled, so each restart compiled all of it and each place again at
  # its first opening; a unit served so is kept in the disk cache now,
  # checked by its content (qtdeclarative-caches-what-is-served.patch),
  # and written without waiting for the disk, since a restart onto changed
  # files writes hundreds of units and syncing each made it slower than
  # compiling (qtdeclarative-caches-without-syncing.patch). Its heap keeps
  # a free list for each size up to 2 KB, since with eight the last held
  # tens of thousands of runs as the shell started, walked again by each
  # row it made (qtdeclarative-heap-bins-for-each-size.patch). A date
  # string without a digit is refused at once, since trying every form Qt
  # reads took about 2 ms for each empty one, and the shell sorts by times
  # some items never have (qtdeclarative-dates-without-digits-refused.patch).
  # JSON.parse keeps each string value once per parse, since the shell
  # holds its daemons' answers parsed and they say the same words thousands
  # of times (qtdeclarative-json-strings-kept-once.patch).
  # Its Wayland client focuses the window a sync asks for even while an
  # earlier change is still queued, since a leave and an enter close
  # together left the launcher holding the keys yet never active
  # (qtbase-focus-follows-what-was-asked.patch).
  # Only the shell and the
  # modules it loads are built with it, so nothing else on the machine is
  # rebuilt for it.
  shellQt =
    prev:
    prev.qt6.overrideScope (
      _: qt: {
        qtbase = qt.qtbase.overrideAttrs (old: {
          patches = (old.patches or [ ]) ++ [ ./qtbase-focus-follows-what-was-asked.patch ];
        });
        qtdeclarative = qt.qtdeclarative.overrideAttrs (old: {
          # In the order Qt's branch took them, as each collector fix is
          # written over the one before it.
          patches = (old.patches or [ ]) ++ [
            ./qtdeclarative-accessor-keeps-its-own-table.patch
            ./qtdeclarative-sweep-keeps-what-it-allocates.patch
            ./qtdeclarative-generator-frames-marked.patch
            ./qtdeclarative-generator-frames-checked.patch
            ./qtdeclarative-maps-find-keys-by-hash.patch
            ./qtdeclarative-caches-what-is-served.patch
            ./qtdeclarative-caches-without-syncing.patch
            ./qtdeclarative-heap-bins-for-each-size.patch
            ./qtdeclarative-dates-without-digits-refused.patch
            ./qtdeclarative-json-strings-kept-once.patch
          ];
        });
      }
    );
  # The standard library alone, so it builds with rustc and no crates.
  keyboardLights = pkgs.runCommandCC "keyboard-lights" { nativeBuildInputs = [ pkgs.rustc ]; } ''
    mkdir -p $out/bin
    rustc --edition 2024 -C opt-level=3 -D warnings \
      -o $out/bin/keyboard-lights ${./keyboard-lights.rs}
  '';
in
{
  nixpkgs.overlays = [
    (_: prev: {
      quickshell =
        let
          qt6 = shellQt prev;
          # Built with the shell's own Qt, as a QML module loaded in its
          # process must be.
          maths = prev.callPackage ./maths/package.nix { inherit qt6; };
          keyTimes = prev.callPackage ./key-times/package.nix { inherit qt6; };
          heap = prev.callPackage ./heap/package.nix { inherit qt6; };
        in
        (prev.quickshell.override { inherit qt6; }).overrideAttrs (old: {
          patches = (old.patches or [ ]) ++ [
            ./activation-token.patch
            ./bluetooth-finds-bluez-late.patch
            ./bluetooth-rssi-and-failures.patch
            ./desktop-entries-heard-only-when-theirs.patch
            ./fileview-quiet.patch
            ./fileview-watch-nearest.patch
            ./fileview-reads-again.patch
            ./fileview-hears-a-folder.patch
            ./fileview-keeps-its-watcher.patch
            ./frame-presentation.patch
            ./ipc-replies-whole.patch
            ./pipewire-node-state-and-format.patch
            ./scanner-reads-headers.patch
          ];
          qtWrapperArgs = (old.qtWrapperArgs or [ ]) ++ [
            "--prefix"
            "NIXPKGS_QT6_QML_IMPORT_PATH"
            ":"
            "${maths}/${prev.qt6.qtbase.qtQmlPrefix}"
            "--prefix"
            "NIXPKGS_QT6_QML_IMPORT_PATH"
            ":"
            "${keyTimes}/${prev.qt6.qtbase.qtQmlPrefix}"
            "--prefix"
            "NIXPKGS_QT6_QML_IMPORT_PATH"
            ":"
            "${heap}/${prev.qt6.qtbase.qtQmlPrefix}"
          ];
        });
      wf-recorder = prev.wf-recorder.overrideAttrs (old: {
        patches = (old.patches or [ ]) ++ [
          ./wf-recorder-dmabuf-version.patch
          ./wf-recorder-stops-at-once.patch
        ];
      });
    })
  ];

  security.pam.services.quickshell = { };

  # The clock's zone follows where the laptop is (home.nix's
  # clock-follows), set through timedated, which asks polkit. An
  # administrator could already set it with their password, and a zone
  # changes only how the time is shown, never the time itself, so letting
  # wheel set it without one gives nothing more away. A machine that fixes
  # its zone in its configuration (`time.timeZone`) keeps it: timedated
  # refuses to change a zone the system owns.
  security.polkit.enable = true;
  security.polkit.extraConfig = builtins.readFile ./set-timezone.rules;

  services.geoclue2 = {
    enable = true;
    geoProviderUrl = "https://api.beacondb.net/v1/geolocate";
    appConfig.whereabouts = {
      isAllowed = true;
      isSystem = false;
    };
  };

  users.users.keyboard-lights = {
    isSystemUser = true;
    group = "keyboard-lights";
  };
  users.groups.keyboard-lights = { };

  systemd.tmpfiles.settings.keyboard-lights."/run/keyboard-lights".d = {
    mode = "0755";
    user = "keyboard-lights";
    group = "keyboard-lights";
  };

  systemd.services.keyboard-lights = {
    description = "Says whether caps lock is on, for Quickshell";
    wantedBy = [ "multi-user.target" ];
    serviceConfig = {
      ExecStart = "${keyboardLights}/bin/keyboard-lights /run/keyboard-lights/capslock /run/keyboard-lights/keyboards";
      Restart = "on-failure";
      User = "keyboard-lights";
      Group = "keyboard-lights";
      SupplementaryGroups = [ "input" ];
      ReadWritePaths = [ "/run/keyboard-lights" ];
      RemoveIPC = true;
      DevicePolicy = "closed";
      DeviceAllow = [ "char-input r" ];
      CapabilityBoundingSet = "";
      NoNewPrivileges = true;
      PrivateNetwork = true;
      RestrictAddressFamilies = "none";
      IPAddressDeny = "any";
      ProtectSystem = "strict";
      ProtectHome = true;
      PrivateTmp = true;
      ProtectKernelTunables = true;
      ProtectKernelModules = true;
      ProtectKernelLogs = true;
      ProtectControlGroups = true;
      ProtectClock = true;
      ProtectHostname = true;
      ProtectProc = "invisible";
      ProcSubset = "pid";
      RestrictNamespaces = true;
      RestrictRealtime = true;
      RestrictSUIDSGID = true;
      LockPersonality = true;
      MemoryDenyWriteExecute = true;
      SystemCallArchitectures = "native";
      SystemCallFilter = [
        "@system-service"
        "~@privileged @resources"
      ];
      UMask = "0022";
    };
  };
}
