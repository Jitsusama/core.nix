# The Apple MacBook Air 13-inch (M2, 2022; model A2681, Mac14,2, board
# J413): Apple's M2 (T8112), four performance and four efficiency cores and
# its own GPU, a 2560x1664 Liquid Retina panel, Broadcom's BCM4387 Wi-Fi 6
# and Bluetooth, two USB-C ports and MagSafe. Asahi Linux drives it: core's
# own build of its kernel (kernel/), with a screen on the front-left port,
# and core's own boot chain (boot/), with the firmware for
# Wi-Fi, Bluetooth and the camera taken from macOS by Asahi's installer.
# It has no TPM and no Secure Boot a Linux system can use. docs/hardware.md
# says what works and what's left.
{ nixos-apple-silicon, hibernate }:
{ ... }:
{
  imports = [
    nixos-apple-silicon.nixosModules.apple-silicon-support
    ../../modules/firmware-settings/nixos.nix
    ./boot/nixos.nix
    ./kernel/nixos.nix
  ];

  disabledModules = [
    # core builds the kernel itself, so nixos-apple-silicon's kernel module,
    # which would set its own, is left out.
    "${nixos-apple-silicon}/apple-silicon-support/modules/kernel"
    # core builds the boot chain too (boot/), with the Air's own device
    # tree.
    "${nixos-apple-silicon}/apple-silicon-support/modules/boot-m1n1"
    # The laptop role hibernates, which here can't come back: systemd keeps
    # where the image went in an EFI variable, and U-Boot keeps none, so
    # the next start would begin afresh and the session would be lost. No
    # one has yet resumed from disk on Apple silicon. The kernel is built
    # without hibernation too, so nothing offers it.
    hibernate
  ];

  nixpkgs.hostPlatform = "aarch64-linux";

  # Set rather than left to the module's default, which it says will change.
  hardware.asahi.enable = true;

  # The firmware Asahi's installer copied from macOS stays on the EFI system
  # partition, and the initrd unpacks it at each boot. The module's default
  # reads it from /boot while evaluating, which a machine built anywhere but
  # on itself, or by a pure evaluation, can't do. Unpacking it needs systemd
  # in the initrd.
  hardware.asahi.peripheralFirmwareDirectory = null;
  boot.initrd.systemd.enable = true;

  # m1n1, which Asahi's installer puts in the macOS stub's place, starts
  # U-Boot, which gives a plain UEFI with no Secure Boot and no variables
  # for systemd-boot to keep its entries in, so it's installed where
  # firmware looks without any. boot/ installs m1n1's second stage beside
  # it on each switch.
  boot.loader.systemd-boot.enable = true;

  # With neither Secure Boot nor a TPM, the guide makes no Secure Boot keys
  # and seals nothing, and the disk opens with a YubiKey or the recovery
  # words at every start.
  jitsusama.firmware.secureBoot = false;
  jitsusama.firmware.tpm = false;

  # U-Boot starts a stick before the disk while nothing is installed, and
  # after once something is; its own menu picks one.
  jitsusama.firmware.startFromStick = "Plug the stick in and press the power button. If the installed system starts instead, restart, press a key when U-Boot says it will start on its own, type bootmenu and pick usb 0.";
}
