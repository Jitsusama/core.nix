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
    ../../modules/hibernate/nixos.nix
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
  # better than one that's gone: every device a driver binds, read from the
  # laptop, but those only another entry's device can bring, such as a PCIe
  # bridge, the Aggregator's UART and hub, or a camera's streams. The I2C
  # controllers but the light sensor's carry nothing a driver takes, so the
  # light sensor stands for them. The battery, the charger, the keyboard and
  # the touchpad come through the Surface Aggregator, so those gone together
  # mean the controller's drivers didn't bind. The kernel numbers HID
  # devices, sound cards and I2C buses in the order they come, so those are
  # globs. devices-seen.txt holds what the laptop showed, for
  # the-surface-names-each-device-it-misses.
  jitsusama.devices.expected = {
    "Wi-Fi" = "/sys/bus/pci/drivers/iwlwifi/0000:02:00.0";
    "Wi-Fi's radio" = "/sys/class/ieee80211/phy*";
    # The AX200's Bluetooth half is a USB device inside the laptop.
    "Bluetooth" = "/sys/bus/usb/drivers/btusb/3-4:1.0";
    "Bluetooth's adapter" = "/sys/class/bluetooth/hci0";
    # The interface a headset's microphone and speaker talk over.
    "Bluetooth's audio" = "/sys/bus/usb/drivers/btusb/3-4:1.1";
    # The controller with two of the ports the firmware says reach the
    # outside; the other has a third, the cameras and Bluetooth.
    "The USB ports" = "/sys/bus/pci/drivers/xhci_hcd/0000:03:00.3";
    "The inner USB controller" = "/sys/bus/pci/drivers/xhci_hcd/0000:03:00.4";
    "The disk" = "/sys/bus/pci/drivers/nvme/0000:01:00.0";
    "The graphics" = "/sys/bus/pci/drivers/amdgpu/0000:03:00.0";
    "The screen" = "/sys/class/drm/card*-eDP-1";
    "The screen's brightness" = "/sys/class/backlight/amdgpu_bl*";
    # The analog codec's controller; 03:00.1 is the GPU's, for HDMI and DP.
    "The speakers" = "/sys/bus/pci/drivers/snd_hda_intel/0000:03:00.6";
    # The analog codec, the ALC274, which both the speakers and the
    # microphones go through. Its card's number depends on which of the two
    # controllers is found first.
    "The microphones" = "/sys/bus/hdaudio/drivers/snd_hda_codec_alc269/hdaudioC*D0";
    # The GPU's codec, for sound over HDMI and DP.
    "Sound to a display" = "/sys/bus/hdaudio/drivers/snd_hda_codec_atihdmi/hdaudioC*D0";
    # What the Aggregator's devices hang from, on its UART.
    "The Surface controller" = "/sys/bus/serial/drivers/surface_serial_hub/serial0-0";
    "The keyboard" = "/sys/bus/surface_aggregator/drivers/surface_hid/01:15:02:01:00";
    "The keyboard's keys" = "/sys/bus/hid/drivers/hid-generic/0019:045E:09AE.*";
    "The touchpad" = "/sys/bus/surface_aggregator/drivers/surface_hid/01:15:02:03:00";
    # hid-multitouch makes it a touchpad of many fingers; taken by
    # hid-generic, it would be a mouse.
    "The touchpad's gestures" = "/sys/bus/hid/drivers/hid-multitouch/0019:045E:09AF.*";
    # The Aggregator's fifth HID device, 01:15:02:05:00, carries only
    # Microsoft's own usage page, 0xFF05, in reports of 32 bytes each way
    # and a feature report of 57: a channel of Microsoft's, not an input.
    "The controller's own channel" = "/sys/bus/hid/drivers/hid-generic/0019:045E:0922.*";
    # The panel, bound once the touchscreen's driver loads after boot.
    "The touchscreen" = "/sys/bus/spi/drivers/sl4a_spi_hid/spi-MSHW0231:00";
    # The pen, which the touchscreen's driver presents as a HID device of
    # its own, numbered in the order HID devices come.
    "The pen" = "/sys/bus/hid/drivers/hid-generic/001C:045E:0C19.*";
    "The light sensor" = "/sys/bus/i2c/drivers/apds9960/i2c-MSHW0184:00";
    "The lid's switch" = "/sys/bus/platform/drivers/acpi-button/PNP0C0D:00";
    # The firmware's own power button, beside the one MSHW0040 gives.
    "The firmware's power button" = "/sys/bus/platform/drivers/acpi-button/PNP0C0C:00";
    # The pins the power button's key, among others, comes through.
    "The GPIO controller" = "/sys/bus/platform/drivers/amd_gpio/AMDI0031:00";
    # The power button, on Surface's button device MSHW0040, which gives the
    # power key alone; the volume keys are on the keyboard. The kernel numbers
    # the instance.
    "The power button" = "/sys/bus/platform/drivers/gpio-keys/gpio-keys.*.auto";
    # The front cameras are one USB device inside the laptop: the colour
    # camera's first interface and the IR camera's.
    "The camera" = "/sys/bus/usb/drivers/uvcvideo/3-3:1.0";
    "The IR camera" = "/sys/bus/usb/drivers/uvcvideo/3-3:1.2";
    "The battery" = "/sys/class/power_supply/BAT1";
    "The charger" = "/sys/class/power_supply/ADP1";
    "The power profiles" =
      "/sys/bus/surface_aggregator/drivers/surface_platform_profile/01:03:01:00:01";
    # The controller's sensors and fan, which the kernel patch below names.
    "The temperature sensors" = "/sys/bus/surface_aggregator/drivers/surface_temp/01:03:01:00:02";
    "The fan" = "/sys/bus/surface_aggregator/drivers/surface_fan/01:05:01:01:01";
    "The processor's temperature" = "/sys/bus/pci/drivers/k10temp/0000:00:18.3";
    "The power telemetry" = "/sys/bus/platform/drivers/intel_rapl_msr/intel_rapl_msr.0";
    # The memory's two SPD chips, on the SMBus, which say what memory it is.
    "The memory's first description" = "/sys/bus/i2c/drivers/ee1004/*-0050";
    "The memory's second description" = "/sys/bus/i2c/drivers/ee1004/*-0051";
    # The CMOS clock, rtc0, which sets the time at boot.
    "The CMOS clock" = "/sys/bus/platform/drivers/rtc_cmos/rtc_cmos";
    # The device surface-clock sets the controller's clock through.
    "The controller's clock" = "/sys/class/misc/surface_aggregator";
    # What tells the kernel how deep each sleep went, which device-check reads.
    "The sleep controller" = "/sys/bus/platform/drivers/amd_pmc/AMD0005:00";
    # What the disk's seal and the accounts' SSH keys are kept in.
    "The TPM" = "/sys/bus/platform/drivers/tpm_tis/NTC0702:00";
    # AMD's platform security processor.
    "The security processor" = "/sys/bus/pci/drivers/ccp/0000:03:00.2";
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

  # The first hibernation rolled back for want of memory: amdgpu copies the
  # GPU's memory out a second time as it stops, after the image was sized,
  # and 343,920 pages more needed saving than had been set aside, where
  # only 117,000 were free. A reserve of 1 GiB holds the image to about
  # 400,000 pages and leaves about a million free for the copy, which
  # needs twice what the driver adds. docs/hardware.md has the arithmetic.
  jitsusama.hibernate.reserve = 1024 * 1024 * 1024;

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
    # Mainline's fix, marked for stable, until a 7.3 release carries it.
    {
      name = "amd-pstate-skip-matching-auto-sel";
      patch = ./kernel/amd-pstate-skip-matching-auto-sel.patch;
    }
    # A wake could ask the charger for its state before the controller had
    # resumed, which warned and tainted the kernel.
    {
      name = "surface-power-asks-only-awake";
      patch = ./kernel/surface-power-asks-only-awake.patch;
    }
    # A restore from hibernation asked the controller to turn the keyboard's
    # and touchpad's events back on while it couldn't answer, and waited 27 s
    # on its timeouts with the screen dark, leaving them off. They're asked
    # for once the restore is over.
    {
      name = "surface-events-after-restore";
      patch = ./kernel/surface-events-after-restore.patch;
    }
    # The PSP refuses the secure display TA at every boot, and only a
    # debugfs test of the display uses it.
    {
      name = "amdgpu-no-secure-display";
      patch = ./kernel/amdgpu-no-secure-display.patch;
    }
    # A wake warned when a screen was off as the laptop slept: the restored
    # state kept VRR on for the dark screen, and the driver asked for its
    # vblank while it was off. VRR now counts only while a screen is on.
    {
      name = "amdgpu-vrr-reference-only-while-on";
      patch = ./kernel/amdgpu-vrr-reference-only-while-on.patch;
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
