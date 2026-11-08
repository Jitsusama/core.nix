# The stick's own system on an Apple silicon Mac, beside stick.nix: the
# same install and repairs, on Asahi's kernel through nixos-apple-silicon,
# booted by the U-Boot that Asahi's installer leaves on the Mac's own boot
# partition. U-Boot has no Secure Boot, so nothing signs this stick.
{ nixos-apple-silicon }:
{ ... }:
{
  imports = [ nixos-apple-silicon.nixosModules.apple-silicon-support ];

  nixpkgs.hostPlatform = "aarch64-linux";

  # Set rather than left to the module's default, which it says will change.
  hardware.asahi.enable = true;

  # The firmware for Wi-Fi, Bluetooth and the camera, which Asahi's
  # installer copied from macOS to the Mac's own boot partition, is
  # unpacked from there as the stick starts, as on an installed Mac
  # (hardware/apple-macbook-air-m2-a2681/nixos.nix). Read while evaluating
  # instead, it would be whichever Mac built the stick's, or none. The stick
  # already starts with systemd in its initrd, which unpacking needs.
  hardware.asahi.peripheralFirmwareDirectory = null;
}
