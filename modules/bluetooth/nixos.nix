# Bluetooth for headphones, mice and keyboards. PipeWire's session manager
# picks up headphones by itself once BlueZ runs. bluetui pairs devices from a
# terminal.
{
  config,
  lib,
  pkgs,
  ...
}:
let
  # WirePlumber gives a headset's microphone a priority above every other,
  # so it becomes the default the moment the headset connects, and the
  # first program to open the default microphone drops the headset from
  # music quality (AAC, 48 kHz stereo) to call quality (16 kHz mono) both
  # ways. Its script, with that priority set below the machine's own
  # microphones, takes the place of WirePlumber's: the headset's microphone
  # is used only once it's chosen, and choosing it switches the headset to
  # call quality by itself, and choosing another switches it back.
  loopback = "share/wireplumber/scripts/monitors/bluez/create-loopback-node.lua";
  headsetMicrophoneSecond = pkgs.runCommand "wireplumber-headset-microphone-second" { } ''
    mkdir -p $out/${dirOf loopback}
    substitute ${config.services.pipewire.wireplumber.package}/${loopback} $out/${loopback} \
      --replace-fail '["priority.session"] = 2010' '["priority.session"] = 1000'
  '';
  # BlueZ 5.87 never loads the endpoints it cached for a device, so each
  # time headphones come on and set up their audio themselves, BlueZ asks
  # for their endpoints again in the middle of it. Bose headphones then
  # never open the audio they set up, and play nothing until they're
  # reconnected from here (bluez/bluez#2285). The fix is upstream, unreleased.
  cachedEndpoints = pkgs.fetchpatch {
    name = "bluez-a2dp-load-cached-endpoints.patch";
    url = "https://github.com/bluez/bluez/commit/b7d71e5067856b0daf2f1f73b3fc95483236610e.patch";
    hash = "sha256-fx3PQNcIwHitVp2kPE9QUFeFqV3N7cDLeLagN5iNSFk=";
  };
  bluez =
    if pkgs.bluez.version == "5.87" then
      pkgs.bluez.overrideAttrs (old: {
        patches = (old.patches or [ ]) ++ [ cachedEndpoints ];
      })
    else
      pkgs.bluez;
  # bt-agent's daemon forked once it was on the bus, which left it unable
  # to answer the bus or see a signal, so every shutdown waited out its
  # stop timeout. It stays in the foreground, under systemd.
  bluez-tools = pkgs.bluez-tools.overrideAttrs (old: {
    patches = (old.patches or [ ]) ++ [ ./bt-agent-foreground.patch ];
  });
in
{
  services.pipewire.wireplumber.configPackages = [ headsetMicrophoneSecond ];

  hardware.bluetooth = {
    enable = true;
    package = bluez;
    powerOnBoot = true;
    # BlueZ's experimental interfaces report headphones' battery levels.
    settings.General.Experimental = true;
    # The kernel's ISO sockets, which LE Audio headphones stream over; BlueZ
    # turns them on with this UUID, and only them.
    settings.General.KernelExperimental = "6fbaf188-05e0-496a-9885-d6ddfdb4e03e";
    # Paired headphones reconnect in about a second rather than several, for
    # a little more of the radio's power while it listens.
    settings.General.FastConnectable = true;
  };

  # BlueZ asks an agent about every pairing and every service a device
  # starts, and with none registered it refuses them all: even a pairing
  # asked for here fails, and headphones hang up when their audio is
  # refused. This one asks nobody. BlueZ completes a pairing asked for here
  # by itself once an agent exists, and the agent lets a paired device start
  # its services, refuses pairings nobody here asked for, and refuses a
  # passkey, which bluetoothctl is for.
  systemd.services.bluetooth-agent = {
    description = "Bluetooth agent answering for paired devices";
    after = [ "bluetooth.service" ];
    bindsTo = [ "bluetooth.service" ];
    wantedBy = [ "bluetooth.service" ];
    serviceConfig = {
      Type = "exec";
      ExecStart = lib.concatStringsSep " " [
        (lib.getExe' bluez-tools "bt-agent")
        "--capability=NoInputNoOutput"
        # Answers without asking anyone, where it would otherwise ask on
        # a terminal it hasn't got.
        "--daemon"
      ];
      Restart = "on-failure";
      DynamicUser = true;
      CapabilityBoundingSet = "";
      NoNewPrivileges = true;
      PrivateDevices = true;
      PrivateNetwork = true;
      ProtectHome = true;
      ProtectSystem = "strict";
      RestrictAddressFamilies = [ "AF_UNIX" ];
      SystemCallFilter = [ "@system-service" ];
    };
  };

  environment.systemPackages = [ pkgs.bluetui ];
}
