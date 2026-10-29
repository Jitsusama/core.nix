# The Microsoft Surface Laptop 4, 13.5" with AMD (model 1958, 2021): an AMD
# Ryzen 5 Microsoft Surface Edition, six Zen 2 cores with Radeon (Renoir)
# graphics, Intel's AX200 Wi-Fi 6, a discrete Nuvoton TPM, and a keyboard,
# battery and charger behind Surface's own embedded controller, the Surface
# Aggregator Module. docs/hardware.md says what works and what's left.
{
  imports = [
    ../../modules/device-check/nixos.nix
    ../../modules/firmware-settings/nixos.nix
    ../../modules/gpu/nixos.nix
    ../../modules/kernel/nixos.nix
    ../../modules/keyboard/nixos.nix
    ./sound/microphones.nix
  ];

  nixpkgs.hostPlatform = "x86_64-linux";
  jitsusama.kernel.cpu = "znver2";

  # The devices this laptop always has. The battery and the charger come
  # through the Surface Aggregator, as the keyboard does, so either one gone
  # means the controller's drivers didn't bind.
  jitsusama.devices.expected = {
    "Wi-Fi" = "/sys/class/ieee80211/phy*";
    "Bluetooth" = "/sys/class/bluetooth/hci0";
    "The battery" = "/sys/class/power_supply/BAT1";
    "The charger" = "/sys/class/power_supply/ADP1";
    "The camera" = "/sys/class/video4linux/video*";
  };

  # The keyboard reaches the system through the Surface Aggregator, as
  # Microsoft's HID device 045e:09ae.
  jitsusama.keyboard.builtIn = [ "045e:09ae" ];

  # Surface UEFI is reached by a held volume button rather than a key, for
  # the guide to say when a stick is written for it. Other Surfaces enter
  # setup mode when Secure Boot is set to None; this one hasn't been seen to.
  jitsusama.firmware.startFromStick = "Hold volume-down, press and release power, and keep holding volume-down until the Surface logo has gone.";
  jitsusama.firmware.clearSecureBootKeys = "Hold volume-up, press and release power, and keep holding until Surface UEFI opens. Under Security, set Secure Boot to None.";

  # The Radeon GPU, behind the PCIe bridge at 00:08.1.
  jitsusama.gpu.path = "pci-0000:03:00.0";

  # Microcode, and the firmware for Wi-Fi, Bluetooth and the GPU.
  hardware.cpu.amd.updateMicrocode = true;
  hardware.enableRedistributableFirmware = true;

  # The disk and USB, then the keyboard's whole path, so the disk can be
  # unlocked from it: the UART the Surface Aggregator talks over (8250_dw),
  # the Aggregator's own driver, the registry and hub that name its devices,
  # and the HID drivers for the keyboard itself. udev loads each as its
  # device appears. The GPIO driver the UART needs, pinctrl_amd, is built in.
  # The linux-surface wiki lists the same set for disk encryption; a stick
  # carrying it typed on this keyboard before any disk was open.
  boot.initrd.availableKernelModules = [
    "nvme"
    "xhci_pci"
    "usb_storage"
    "uas"
    "sd_mod"
    "8250_dw"
    "surface_aggregator"
    "surface_aggregator_registry"
    "surface_aggregator_hub"
    "surface_hid_core"
    "surface_hid"
  ];
  boot.kernelModules = [ "kvm-amd" ];

  # The AX200's Bluetooth firmware and settings in the initrd, where a paired
  # keyboard types at the disk prompt (bluetooth-disk-prompt). btintel picks
  # the files by what the chip answers, so the module tree can't name them.
  boot.initrd.extraFirmwarePaths = [
    "intel/ibt-20-1-3.sfi"
    "intel/ibt-20-1-3.ddc"
  ];

  # The GPU's driver in the initrd, so it takes the screen from the
  # firmware's framebuffer before the disk's PIN is asked for. Loaded after
  # the disk opened, it took the console over just as udev started the
  # console's setup again for it, and loadkeys failed with an I/O error.
  hardware.amdgpu.initrd.enable = true;

  # The firmware's Time and Alarm Device registers as rtc0 ahead of the
  # CMOS clock, but every read of it fails with an I/O error, so the kernel
  # set no time at boot and each boot started at systemd's fallback date
  # until NTP answered. Without it the CMOS clock is rtc0, which the kernel
  # keeps in step while NTP answers. docs/hardware.md has what was seen.
  boot.blacklistedKernelModules = [ "acpi_tad" ];

  # The firmware sets the CMOS clock back to a day in 2025 at every
  # restart, warm or cold, whatever the kernel wrote to it, and the kernel
  # sets the system's time from rtc0 when it registers. Loaded late by udev,
  # it set each boot back to 2025 after systemd had moved the time forward
  # to its last saved one. Loaded in the initrd, it sets the time before systemd's
  # own advance, so the time only moves forward from there.
  boot.initrd.kernelModules = [ "rtc_cmos" ];

  # The clock runs on HPET. Early in each boot the TSC and the platform's
  # timers disagree by up to 0.35%, so the kernel's watchdog throws the TSC
  # out, and with the watchdog off its calibration can come out 0.16% fast,
  # more than NTP can steer. docs/hardware.md has the measurements.
  boot.kernelParams = [ "tsc=unstable" ];
}
