# The Apple MacBook Air 13-inch (M2, 2022; model A2681, Mac14,2, board
# J413): Apple's M2 (T8112), four performance and four efficiency cores and
# its own GPU, a 2560x1664 Liquid Retina panel, Broadcom's BCM4387 Wi-Fi 6
# and Bluetooth, two USB-C ports and MagSafe. Asahi Linux drives it, all
# built by core: its kernel (kernel/), with a screen on the front-left port;
# its boot chain (boot/); the firmware for Wi-Fi, Bluetooth and the camera,
# which Asahi's installer takes from macOS (modules/asahi-firmware); and
# its speakers (modules/asahi-audio); and the tools that pick macOS as the
# next start (modules/asahi-nvram). It has no TPM and no Secure Boot a
# Linux system can use. docs/hardware.md says what works and what's left.
# What kernel/ and boot/ take from nixos-apple-silicon comes under its MIT
# notice, LICENSE.nixos-apple-silicon.
{ bluetooth-disk-prompt, hibernate }:
{ pkgs, ... }:
{
  imports = [
    ../../modules/asahi-audio/nixos.nix
    ../../modules/asahi-firmware/nixos.nix
    ../../modules/asahi-nvram/nixos.nix
    ../../modules/firmware-settings/nixos.nix
    ./boot/nixos.nix
    ./kernel/nixos.nix
  ];

  disabledModules = [
    # The laptop role hibernates, which here can't come back: systemd keeps
    # where the image went in an EFI variable, and U-Boot keeps none, so
    # the next start would begin afresh and the session would be lost. No
    # one has yet resumed from disk on Apple silicon. The kernel is built
    # without hibernation too, so nothing offers it.
    hibernate

    # The laptop role lets a paired Bluetooth keyboard type at the disk
    # prompt, with its pairings sealed to the TPM, which this Mac lacks, so
    # its sealing would fail at every run. Its initrd Bluetooth is for USB
    # controllers too, and this one is on PCIe, with firmware the initrd
    # doesn't carry. The built-in keyboard types at the prompt instead.
    bluetooth-disk-prompt
  ];

  nixpkgs.hostPlatform = "aarch64-linux";

  # The video decoder's firmware, which Asahi writes itself, so unlike the
  # rest it's built from source here.
  hardware.firmware = [ pkgs.avd-fw ];

  # iio-sensor-proxy, for the sensors Asahi's kernel drives, as
  # nixos-apple-silicon sets for every Mac.
  hardware.sensor.iio.enable = true;

  # m1n1, which Asahi's installer puts in the macOS stub's place, starts
  # U-Boot, which gives a plain UEFI with no Secure Boot and no variables
  # for systemd-boot to keep its entries in, so it's installed where
  # firmware looks without any. boot/ installs m1n1's second stage beside
  # it on each switch.
  boot.loader.systemd-boot.enable = true;

  # The keyboard reaches the system through the M2's DockChannel, which
  # Asahi's kernel gives Apple's ID for this model: 05ac:0351, the 2022 Air's
  # in its hid-ids.h, seen on an Air as "Apple MTP keyboard". Without it,
  # keyd would leave the built-in keyboard US at the stick, the disk prompt
  # and the session alike.
  jitsusama.keyboard.builtIn = [ "05ac:0351" ];

  # With neither Secure Boot nor a TPM, the guide makes no Secure Boot keys
  # and seals nothing, and the disk opens with a YubiKey or the recovery
  # words at every start.
  jitsusama.firmware.secureBoot = false;
  jitsusama.firmware.tpm = false;

  # U-Boot starts a stick before the disk while nothing is installed, and
  # after once something is; its own menu picks one.
  jitsusama.firmware.startFromStick = "Plug the stick in and press the power button. If the installed system starts instead, restart, press a key when U-Boot says it will start on its own, type bootmenu and pick usb 0.";
}
