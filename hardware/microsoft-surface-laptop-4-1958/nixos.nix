# The Microsoft Surface Laptop 4, 13.5" with AMD (model 1958, 2021): an AMD
# Ryzen 5 Microsoft Surface Edition, six Zen 2 cores with Radeon (Renoir)
# graphics, Intel's AX200 Wi-Fi 6, a discrete Nuvoton TPM, and a keyboard,
# battery and charger behind Surface's own embedded controller, the Surface
# Aggregator Module. docs/hardware.md says what works and what's left.
{ config, pkgs, ... }:
{
  imports = [
    ../../modules/device-check/nixos.nix
    ../../modules/firmware-settings/nixos.nix
    ../../modules/gpu/nixos.nix
    ../../modules/kernel/nixos.nix
    ../../modules/keyboard/nixos.nix
    ./clock/clock.nix
    ./firmware/firmware.nix
    ./sound/microphones.nix
    ./touch/touchscreen.nix
  ];

  nixpkgs.hostPlatform = "x86_64-linux";
  jitsusama.kernel.cpu = "znver2";

  # The devices this laptop always has, each where its driver binds it where
  # it has one, since a device the kernel found but no driver took works no
  # better than one that's gone. The battery, the charger, the keyboard and
  # the touchpad come through the Surface Aggregator, so those gone together
  # mean the controller's drivers didn't bind. The microphones aren't here
  # yet: their driver doesn't bind at boot.
  jitsusama.devices.expected = {
    "Wi-Fi" = "/sys/bus/pci/drivers/iwlwifi/0000:02:00.0";
    "Wi-Fi's radio" = "/sys/class/ieee80211/phy*";
    # The AX200's Bluetooth half is a USB device inside the laptop.
    "Bluetooth" = "/sys/bus/usb/drivers/btusb/3-4:1.0";
    "Bluetooth's adapter" = "/sys/class/bluetooth/hci0";
    "The disk" = "/sys/bus/pci/drivers/nvme/0000:01:00.0";
    "The graphics" = "/sys/bus/pci/drivers/amdgpu/0000:03:00.0";
    "The screen" = "/sys/class/drm/card*-eDP-1";
    "The screen's brightness" = "/sys/class/backlight/amdgpu_bl*";
    # The analog codec's controller; 03:00.1 is the GPU's, for HDMI and DP.
    "The speakers" = "/sys/bus/pci/drivers/snd_hda_intel/0000:03:00.6";
    "The keyboard" = "/sys/bus/surface_aggregator/drivers/surface_hid/01:15:02:01:00";
    "The touchpad" = "/sys/bus/surface_aggregator/drivers/surface_hid/01:15:02:03:00";
    # The panel, bound once the touchscreen's driver loads after boot.
    "The touchscreen" = "/sys/bus/spi/drivers/sl4a_spi_hid/spi-MSHW0231:00";
    "The light sensor" = "/sys/bus/i2c/drivers/apds9960/i2c-MSHW0184:00";
    "The lid's switch" = "/sys/bus/platform/drivers/acpi-button/PNP0C0D:00";
    "The camera" = "/sys/class/video4linux/video*";
    "The battery" = "/sys/class/power_supply/BAT1";
    "The charger" = "/sys/class/power_supply/ADP1";
    "The power profiles" =
      "/sys/bus/surface_aggregator/drivers/surface_platform_profile/01:03:01:00:01";
    # The controller's sensors and fan, which the kernel patch below names.
    "The temperature sensors" = "/sys/bus/surface_aggregator/drivers/surface_temp/01:03:01:00:02";
    "The fan" = "/sys/bus/surface_aggregator/drivers/surface_fan/01:05:01:01:01";
    "The processor's temperature" = "/sys/bus/pci/drivers/k10temp/0000:00:18.3";
    # The device surface-clock sets the controller's clock through.
    "The controller's clock" = "/sys/class/misc/surface_aggregator";
    # What tells the kernel how deep each sleep went, which device-check reads.
    "The sleep controller" = "/sys/bus/platform/drivers/amd_pmc/AMD0005:00";
    # What the disk's seal and the accounts' SSH keys are kept in.
    "The TPM" = "/sys/bus/platform/drivers/tpm_tis/NTC0702:00";
    "The watchdog" = "/sys/class/watchdog/watchdog0";
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

  # The firmware sets the CMOS clock from the controller's own clock at every
  # restart, warm or cold, whatever the kernel wrote to it, and the controller's
  # clock, which nothing set, read a day in 2025; clock/clock.nix keeps it at
  # the true time. The kernel sets the system's time from rtc0 when it
  # registers. Loaded late by udev,
  # it set each boot back to 2025 after systemd had moved the time forward
  # to its last saved one. Loaded in the initrd, it sets the time before systemd's
  # own advance, so the time only moves forward from there.
  boot.initrd.kernelModules = [ "rtc_cmos" ];

  # The clock runs on HPET. Early in each boot the TSC and the platform's
  # timers disagree by up to 0.35%, so the kernel's watchdog throws the TSC
  # out, and with the watchdog off its calibration can come out 0.16% fast,
  # more than NTP can steer. docs/hardware.md has the measurements.
  #
  # amd-pstate runs in guided mode: the firmware is always autonomous, and
  # its _OSC doesn't grant what active mode's energy preference writes need.
  # The patch below says how both were found.
  boot.kernelParams = [
    "tsc=unstable"
    "amd_pstate=guided"
  ];

  # A panic drew its text but never restarted the laptop, which sat frozen
  # until the power button was held. The chipset's watchdog, which systemd
  # feeds while the kernel runs, resets it 30 s after the kernel stops.
  systemd.settings.Manager.RuntimeWatchdogSec = "30s";

  # A sleep stops the watchdog's count, and its driver never starts it
  # again, so the hook stops it before each sleep and starts it after; the
  # script says why. systemd-sleep runs it with a PATH that has no
  # coreutils, so it brings its own.
  environment.etc."systemd/system-sleep/watchdog-wakes-again".source =
    let
      hook = pkgs.writeShellApplication {
        name = "watchdog-wakes-again";
        runtimeInputs = [
          config.systemd.package
          pkgs.coreutils
        ];
        text = builtins.readFile ./watchdog-wakes-again.sh;
      };
    in
    "${hook}/bin/watchdog-wakes-again";

  boot.kernelPatches = [
    # The Surface Aggregator answers for eight thermal sensors and the fan's
    # speed, but the kernel's registry gives this model the Intel models'
    # devices, which have neither. The patch says what the controller
    # answered.
    {
      name = "surface-laptop-amd-sensors-and-fan";
      patch = ./kernel/surface-laptop-amd-sensors-and-fan.patch;
    }
    # The firmware fixes _CPC's Autonomous Selection Enable at 1, which
    # amd-pstate refused to accept, so the CPU fell back to acpi-cpufreq.
    {
      name = "amd-pstate-fixed-autonomous-selection";
      patch = ./kernel/amd-pstate-fixed-autonomous-selection.patch;
    }
    # The AX200 joined Wi-Fi at boot before it had heard a country, and the
    # driver ignored the one it heard while connected, so it stayed on the
    # world domain.
    {
      name = "iwlmvm-country-while-associated";
      patch = ./kernel/iwlmvm-country-while-associated.patch;
    }
  ];
}
