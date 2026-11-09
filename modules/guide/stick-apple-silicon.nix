# The stick's own system on an Apple silicon Mac, beside stick.nix: the
# same install and repairs, on core's own build of Asahi's kernel, booted by
# the U-Boot that Asahi's installer leaves on the Mac's own boot partition.
# U-Boot has no Secure Boot, so nothing signs this stick. Nothing on it
# plays sound, so it leaves out the speakers' support.
{ ... }:
{
  imports = [
    # The kernel the MacBook Air M2 runs, core's only Apple model, so the
    # stick starts the very kernel an install will, and one build serves
    # both. Asahi's kernel holds every Apple model's device trees; this
    # moves to where every model takes it when a second one comes.
    ../../hardware/apple-macbook-air-m2-a2681/kernel/nixos.nix
    # Its boot chain too, carried on the stick, so an install has m1n1
    # and U-Boot without fetching them.
    ../../hardware/apple-macbook-air-m2-a2681/boot/nixos.nix
    # The firmware for Wi-Fi, Bluetooth and the camera, which Asahi's
    # installer copied from macOS to the Mac's own boot partition, unpacked
    # from there as the stick starts, as on an installed Mac. Read while
    # evaluating instead, it would be whichever Mac built the stick's, or
    # none.
    ../asahi-firmware/nixos.nix
  ];

  nixpkgs.hostPlatform = "aarch64-linux";
}
