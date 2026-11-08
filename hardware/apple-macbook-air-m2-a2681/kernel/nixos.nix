# The M2 Air runs core's own Asahi kernel (package.nix), in place of the one
# nixos-apple-silicon's kernel module gives, which the hardware module leaves
# out. What that module set beside its kernel is kept here, from its
# modules/kernel at the commit core pins (MIT), each with its reason.
{ config, lib, ... }:
{
  # Built from hardware.asahi.pkgs, so a machine that cross-compiles its
  # Apple packages cross-compiles this too.
  boot.kernelPackages = config.hardware.asahi.pkgs.linuxPackagesFor (
    config.hardware.asahi.pkgs.callPackage ./package.nix { }
  );

  # schedutil, for the kernel's energy model, which knows the efficiency
  # cores from the performance ones.
  powerManagement.cpuFreqGovernor = lib.mkOverride 800 "schedutil";

  # What the disk, the keyboard and the stick need before the root file
  # system is open, after Asahi's own initramfs (asahi-scripts' list).
  boot.initrd.availableKernelModules = [
    "apple-mailbox"
    "apple_nvmem_spmi"
    "nvme_apple"
    "pinctrl-apple-gpio"
    "macsmc"
    "macsmc-power"
    "macsmc-input"
    "macsmc-hwmon"
    "macsmc-reboot"
    "i2c-pasemi-platform"
    "tps6598x"
    "sn201202x"
    "apple-dart"
    "dwc3"
    "dwc3-of-simple"
    "xhci-pci"
    "pcie-apple"
    "gpio_macsmc"
    "phy-apple-atc"
    "nvmem_apple_efuses"
    "spi-apple"
    "spi-hid-apple"
    "spi-hid-apple-of"
    "rtc-macsmc"
    "spmi-apple-controller"
    "apple-dockchannel"
    "dockchannel-hid"
    "apple-rtkit-helper"
    # A stick, and a keyboard on USB, for when the installed system won't
    # start.
    "usb-storage"
    "xhci-plat-hcd"
    "usbhid"
    "hid_generic"
    "thunderbolt_apple"
    "reset_apple_cio"
  ];

  boot.kernelParams = [
    "earlycon"
    "console=tty0"
    "boot.shell_on_fail"
    # Apple's disks take tens of milliseconds over each flush, so programs
    # that sync often crawl; this sends flushes at once rather than
    # gathering them over a delay, which would risk data at a power loss.
    "nvme_apple.flush_interval=0"
  ];

  # U-Boot keeps no EFI variables.
  boot.loader.efi.canTouchEfiVariables = lib.mkForce false;
}
