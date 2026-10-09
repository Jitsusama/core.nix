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
  };

  environment.systemPackages = [ pkgs.bluetui ];
}
