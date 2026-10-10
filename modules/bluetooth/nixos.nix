# Bluetooth for headphones, mice and keyboards. PipeWire's session manager
# picks up headphones by itself once BlueZ runs. bluetui pairs devices from a
# terminal.
{ pkgs, ... }:
{
  hardware.bluetooth = {
    enable = true;
    powerOnBoot = true;
    # BlueZ's experimental interfaces report headphones' battery levels.
    settings.General.Experimental = true;
    # The kernel's ISO sockets, which LE Audio headphones stream over; BlueZ
    # turns them on with this UUID, and only them.
    settings.General.KernelExperimental = "6fbaf188-05e0-496a-9885-d6ddfdb4e03e";
  };

  environment.systemPackages = [ pkgs.bluetui ];
}
