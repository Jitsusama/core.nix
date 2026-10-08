# The Dell XPS 14 (DA14260, 2026): an Intel Core Ultra Series 3 (Panther Lake
# H) with Arc B390 graphics and an NPU, a 2880x1800 120 Hz OLED touchscreen,
# four CS35L57 SoundWire amplifiers, an IPU7 camera behind Intel's CVS bridge,
# and Intel's BE211 Wi-Fi 7. docs/hardware.md says what works, where each
# workaround comes from, and what's left.
{ nixosModules }:
{ pkgs, ... }:
{
  imports = [ nixosModules.kernel ];

  nixpkgs.hostPlatform = "x86_64-linux";
  jitsusama.kernel.cpu = "pantherlake";

  # Microcode, and the firmware for Wi-Fi, Bluetooth, the GPU and the audio
  # DSP, which Sound Open Firmware runs.
  hardware.cpu.intel.updateMicrocode = true;
  hardware.enableRedistributableFirmware = true;

  boot.kernelPatches = [
    {
      # Without it the camera's CVS bridge holds the pin all four amplifiers
      # read their speaker ID from, so none of them bind and there's no sound
      # card. From Intel, and accepted into the media tree.
      name = "cvs-wake-irq";
      patch = pkgs.fetchpatch {
        name = "cvs-wake-irq.patch";
        url = "https://patchwork.linuxtv.org/project/linux-media/patch/20260913133017.624919-1-junjie.cao@intel.com/mbox/";
        hash = "sha256-javJIhGW6mUlgQjgUTgKCEqpD7704+t8zZLtVTe72Dw=";
      };
    }
  ];

  # The disk, and the USB and Thunderbolt ports a dock's keyboard arrives
  # through, so the disk can be unlocked from either keyboard.
  boot.initrd.availableKernelModules = [
    "nvme"
    "xhci_pci"
    "thunderbolt"
    "usb_storage"
    "uas"
    "sd_mod"
  ];
  boot.kernelModules = [ "kvm-intel" ];

  # Video decoding and encoding through VA-API and oneVPL, and OpenCL and
  # Level Zero for compute, all on the Arc GPU.
  hardware.graphics = {
    enable = true;
    extraPackages = [
      pkgs.intel-media-driver
      pkgs.vpl-gpu-rt
      pkgs.intel-compute-runtime
    ];
  };
  environment.sessionVariables.LIBVA_DRIVER_NAME = "iHD";

  # The NPU, through Level Zero.
  hardware.cpu.intel.npu.enable = true;

  # The ambient light and presence sensors, behind the Integrated Sensor Hub.
  hardware.sensor.iio.enable = true;

  # Intel's thermal daemon, which sets the chip's power limits from the
  # laptop's own thermal tables as it heats.
  services.thermald.enable = true;

  # Authorizes Thunderbolt and USB4 docks.
  services.hardware.bolt.enable = true;

  # Dell publishes this laptop's BIOS and firmware updates through LVFS.
  services.fwupd.enable = true;

  # The BE211's Wi-Fi 7 receive path drops links to the slowest rate, so it
  # runs as Wi-Fi 6, as Omarchy runs it. Remove this once a Wi-Fi 7 link holds
  # its rate without it.
  boot.extraModprobeConfig = "options iwlwifi disable_11be=Y";
}
