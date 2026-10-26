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
# the notifications specification has it. A FileView told not to print
# errors still warned of a file that opened and then refused to be read,
# as sysfs files do, so fileview-quiet.patch keeps it to its word. A
# FileView watching a file lost it for good once the file's directory went
# or if it wasn't there yet, as a service's runtime directory is each time
# systemd restarts the service, so fileview-watch-nearest.patch watches the
# nearest directory that is. An IPC call's connection was deleted as soon
# as its reply was handed over, cutting off a reply longer than the
# socket's buffer, so ipc-replies-whole.patch closes it only once the reply
# is written. All four are laid over the machine's packages, so the shell,
# the greeter and the lock all run them.
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
# and nothing more, so a keystroke never reaches it.
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
# only as the weather's question to Open-Meteo, to a hundredth of a degree.
{ pkgs, ... }:
let
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
      quickshell = prev.quickshell.overrideAttrs (old: {
        patches = (old.patches or [ ]) ++ [
          ./activation-token.patch
          ./fileview-quiet.patch
          ./fileview-watch-nearest.patch
          ./ipc-replies-whole.patch
        ];
        qtWrapperArgs = (old.qtWrapperArgs or [ ]) ++ [
          "--prefix"
          "NIXPKGS_QT6_QML_IMPORT_PATH"
          ":"
          "${prev.callPackage ./maths/package.nix { }}/${prev.qt6.qtbase.qtQmlPrefix}"
        ];
      });
    })
  ];

  security.pam.services.quickshell = { };

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
      ExecStart = "${keyboardLights}/bin/keyboard-lights /run/keyboard-lights/capslock";
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
