# Bluetooth for headphones, mice and keyboards. PipeWire's session manager
# picks up headphones by itself once BlueZ runs. bluetui pairs devices from a
# terminal.
{ lib, pkgs, ... }:
{
  hardware.bluetooth = {
    enable = true;
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
      Type = "forking";
      ExecStart = lib.concatStringsSep " " [
        (lib.getExe' pkgs.bluez-tools "bt-agent")
        "--capability=NoInputNoOutput"
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
