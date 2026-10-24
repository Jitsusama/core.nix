# Creative's Pebble X Plus speakers, tuned to a published measurement, which
# tuning.conf describes, with their lighting dark while nobody uses them.
#
# The tuning runs as a PipeWire client of its own in every account: it
# starts when PipeWire does, after WirePlumber, which links it to the
# speakers whenever they're plugged in, and stops and restarts with it. It
# runs in the sound server's slice, which comes before the programs'
# (modules/slices), since a stall in it is a gap in what the speakers play.
#
# pebble-lights, run as the person at the machine, keeps the speakers'
# lighting on while anything plays to them and for 15 minutes after, and puts
# it out after that, when the screen locks with nothing playing, and when
# the session ends; lights.rs says how. Their small status light can't be
# switched from the computer, so it stays lit. It speaks to the speakers'
# control channel, which their firmware offers from 1.27 on; on older
# firmware it waits, doing nothing, and Creative's app updates it. udev opens
# that channel's node to the person at the seat.
{ config, pkgs, ... }:
let
  # The standard library alone, so it builds with rustc and no crates.
  lights = pkgs.runCommandCC "pebble-lights" { nativeBuildInputs = [ pkgs.rustc ]; } ''
    mkdir -p $out/bin
    rustc --edition 2024 -C opt-level=3 -D warnings -o $out/bin/pebble-lights ${./lights.rs}
  '';
  # Before systemd's 73-seat-late.rules, which hands a device tagged uaccess
  # to whoever sits at the seat, so the tag has to be on by then.
  rules = pkgs.writeTextDir "lib/udev/rules.d/70-pebble-x-plus.rules" ''
    SUBSYSTEM=="hidraw", ATTRS{idVendor}=="041e", ATTRS{idProduct}=="329a", TAG+="uaccess"
  '';
in
{
  imports = [ ../pipewire/nixos.nix ];

  services.udev.packages = [ rules ];

  systemd.user.services.pebble-x-plus-tuning = {
    description = "The Pebble X Plus speakers' tuning";
    after = [
      "pipewire.service"
      "wireplumber.service"
    ];
    requires = [ "pipewire.service" ];
    wants = [ "wireplumber.service" ];
    partOf = [ "pipewire.service" ];
    wantedBy = [ "pipewire.service" ];
    serviceConfig = {
      Slice = "session.slice";
      ExecStart = "${config.services.pipewire.package}/bin/pipewire -c ${./tuning.conf}";
      Restart = "on-failure";
      RestartSec = 2;
    };
  };

  systemd.user.services.pebble-lights = {
    description = "The Pebble X Plus's lighting, dark while nobody uses them";
    wantedBy = [ "default.target" ];
    serviceConfig = {
      ExecStart = "${lights}/bin/pebble-lights %t/quickshell-locked";
      Restart = "on-failure";
      RestartSec = 5;
      NoNewPrivileges = true;
      PrivateNetwork = true;
      RestrictAddressFamilies = "none";
      ProtectSystem = "strict";
      # Where it says what the lighting does.
      ReadWritePaths = [ "%t" ];
      ProtectHome = "read-only";
      PrivateTmp = true;
      CapabilityBoundingSet = "";
      RestrictNamespaces = true;
      RestrictRealtime = true;
      LockPersonality = true;
      MemoryDenyWriteExecute = true;
      SystemCallArchitectures = "native";
      SystemCallFilter = [
        "@system-service"
        "~@privileged @resources"
      ];
    };
  };
}
