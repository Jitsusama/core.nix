# The stick's own system on an Apple silicon Mac, beside stick.nix: the
# same install and repairs, on core's own build of Asahi's kernel, with the
# rest of Asahi's support through nixos-apple-silicon, booted by the U-Boot
# that Asahi's installer leaves on the Mac's own boot partition. U-Boot has
# no Secure Boot, so nothing signs this stick.
{ nixos-apple-silicon }:
{ ... }:
{
  imports = [
    nixos-apple-silicon.nixosModules.apple-silicon-support
    # The kernel the MacBook Air M2 runs, core's only Apple model, so the
    # stick starts the very kernel an install will, and one build serves
    # both. Asahi's kernel holds every Apple model's device trees; this
    # moves to where every model takes it when a second one comes.
    ../../hardware/apple-macbook-air-m2-a2681/kernel/nixos.nix
    # Its boot chain too, carried on the stick, so an install has m1n1
    # and U-Boot without fetching them.
    ../../hardware/apple-macbook-air-m2-a2681/boot/nixos.nix
  ];

  # core builds the kernel and the boot chain itself, so
  # nixos-apple-silicon's modules for them, which would set their own, are
  # left out.
  disabledModules = [
    "${nixos-apple-silicon}/apple-silicon-support/modules/kernel"
    "${nixos-apple-silicon}/apple-silicon-support/modules/boot-m1n1"
  ];

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
