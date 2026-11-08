# Reviving or restoring an Apple silicon Mac from this machine, over a
# USB-C cable, when the Mac won't start: what a second Mac's Finder does,
# done from Linux. irecovery says whether the Mac is in DFU mode, and
# idevicerestore sends it Apple's firmware, keeping its files unless told
# to erase them.
#
# usbmuxd is installed but not run as a service. Its daemon claims every
# Apple device it sees and checks pairing first, which has failed restores
# of M2 to M4 Macs, so a revive starts its own, without that check, on a
# socket of its own and only for as long as the revive takes
# (idevicerestore issues 782 and 755). Both run through run0, since the
# Mac in DFU mode is a raw USB device only root may open.
{ pkgs, ... }:
{
  environment.systemPackages = [
    pkgs.idevicerestore
    pkgs.libirecovery
    pkgs.usbmuxd
  ];
}
