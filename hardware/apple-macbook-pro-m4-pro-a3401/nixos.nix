# The Apple MacBook Pro 14-inch (M4 Pro, 2024; model A3401, Mac16,8, board
# J614s): Apple's M4 Pro (T6040), eight or ten performance and four
# efficiency cores and its own GPU, three Thunderbolt 5 ports, HDMI, an SD
# card slot and MagSafe. Aurora
# Silicon's kernel drives it (kernel/). This module holds only the kernel so
# far: the boot chain, the firmware taken from macOS, and the rest of the
# machine come as each is proven on the hardware. docs/hardware.md says what
# works and what's left.
{ bluetooth-disk-prompt, hibernate }:
{ ... }:
{
  imports = [ ./kernel/nixos.nix ];

  disabledModules = [
    # The laptop role hibernates, which no Apple silicon machine can come
    # back from yet, and the kernel is built without it (kernel/package.nix).
    hibernate

    # The laptop role lets a paired Bluetooth keyboard type at the disk
    # prompt, with its pairings sealed to the TPM, which this Mac lacks, so
    # its sealing would fail at every run. Its initrd Bluetooth is for USB
    # controllers too, and this one is on PCIe. The built-in keyboard types
    # at the prompt instead.
    bluetooth-disk-prompt
  ];

  nixpkgs.hostPlatform = "aarch64-linux";
}
