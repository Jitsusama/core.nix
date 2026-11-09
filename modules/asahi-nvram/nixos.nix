# The tools that read and write an Apple silicon Mac's NVRAM from Linux,
# from Asahi's asahi-nvram project. asahi-bless picks which system the Mac
# starts next, so getting back to macOS needs no held power button.
# asahi-btsync copies the Bluetooth pairings macOS keeps there into BlueZ's,
# so a keyboard paired in macOS works here without pairing again. And
# asahi-nvram reads and writes the variables themselves. Each writes through
# the Mac's NVRAM device, so each runs through run0. Its asahi-wifisync is
# left out: it writes only iwd's networks, and core doesn't run iwd.
{ pkgs, ... }:
{
  environment.systemPackages = [
    pkgs.asahi-bless
    pkgs.asahi-btsync
    pkgs.asahi-nvram
  ];
}
