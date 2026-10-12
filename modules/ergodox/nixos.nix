# The ErgoDox EZ's USB devices, open to whoever is at the machine: the board
# itself, which the shell talks to over raw HID, and the bootloader it
# restarts into to be flashed. Without these only root can reach either, and
# home.nix's `ergodox` can't flash it.
{
  hardware.keyboard.zsa.enable = true;
}
