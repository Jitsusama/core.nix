# The Dell XPS 14 (DA14260, 2026): an Intel Core Ultra Series 3 (Panther Lake
# H) with Arc B390 graphics and an NPU, a 2880x1800 120 Hz OLED touchscreen,
# four CS35L57 SoundWire amplifiers, an IPU7 camera behind Intel's CVS bridge,
# and Intel's BE211 Wi-Fi 7. docs/hardware.md says what works, where each
# workaround comes from, and what's left.
{
  config,
  lib,
  pkgs,
  ...
}:
let
  # nixpkgs gives only the compute runtime's OpenCL driver the graphics
  # compiler's libraries on its RUNPATH, so Level Zero's GPU driver can't load
  # libigc.so.2 and aborts in zeInit. This gives that driver the same RUNPATH,
  # as the line docs/hardware.md has for nixpkgs would. Take it out once
  # nixpkgs's package does it.
  computeRuntime = pkgs.intel-compute-runtime.overrideAttrs (old: {
    postFixup = old.postFixup + ''
      patchelf --set-rpath ${
        lib.makeLibraryPath [
          pkgs.intel-gmmlib
          pkgs.intel-graphics-compiler
          pkgs.libva
          pkgs.stdenv.cc.cc
        ]
      } \
        $out/lib/libze_intel_gpu.so.1
    '';
  });

  # turbostat's Makefile calls the compiler gcc, and the kernel's packages
  # build with clang, which has no gcc, so nixpkgs's turbostat fails to
  # build. cc is whichever compiler the set uses. Take the flag out once
  # nixpkgs's package passes it.
  turbostat = config.boot.kernelPackages.turbostat.overrideAttrs (old: {
    makeFlags = old.makeFlags ++ [ "CC=cc" ];
  });

  # power-profiles-daemon's performance profile asks the cores for the energy
  # preference performance. On this chip balance_performance idled at 3.0 to
  # 4.4 W against 7.0 to 9.5 W, ran a sixteen-thread load 9 to 11% faster,
  # and woke and burst from idle as quickly; docs/hardware.md has the numbers. Its test of
  # Intel's driver expects the value the profile writes, so it changes too.
  powerProfiles = pkgs.power-profiles-daemon.overrideAttrs (old: {
    postPatch = (old.postPatch or "") + ''
      substituteInPlace src/ppd-driver-intel-pstate.c \
        --replace-fail 'return "performance";' 'return "balance_performance";'
      sed -i '/def test_intel_pstate(self):/,/def test_/ s/(energy_prefs, "performance")/(energy_prefs, "balance_performance")/' \
        tests/integration_test.py
    '';
  });
in
{
  imports = [
    ../../modules/firmware-settings/nixos.nix
    ../../modules/gpu/nixos.nix
    ../../modules/kernel/nixos.nix
    ../../modules/keyboard/nixos.nix
    ./camera.nix
    ./speakers.nix
  ];

  nixpkgs.hostPlatform = "x86_64-linux";
  jitsusama.kernel.cpu = "pantherlake";

  # The keyboard is wired to the embedded controller, which presents it as a
  # PS/2 keyboard.
  jitsusama.keyboard.builtIn = [ "0001:0001" ];

  # The Arc GPU. Its driver takes the firmware framebuffer's card away as it
  # starts, and has its own ready about 1.6 s later, so for that long there's
  # no card at all.
  jitsusama.gpu.path = "pci-0000:00:02.0";

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
    {
      # The four amplifiers probe at once and each takes that pin for itself,
      # so now and then one finds it taken and the sound card never comes.
      # Written here; the patch says why.
      name = "cs35l56-spkid-lock";
      patch = ./cs35l56-spkid-lock.patch;
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
  boot.kernelModules = [
    "kvm-intel"
    # thermald looks once, as it starts, for the power limits the CPU's own
    # registers hold, and udev loads the driver for them a second later, so
    # it warns they're missing. Loading it with the early modules, which
    # finish before thermald may start, has them there first.
    "intel_rapl_msr"
  ];

  # Video decoding and encoding through VA-API and oneVPL, and OpenCL and
  # Level Zero for compute, all on the Arc GPU.
  hardware.graphics = {
    enable = true;
    extraPackages = [
      pkgs.intel-media-driver
      pkgs.vpl-gpu-rt
      computeRuntime
    ];
  };
  environment.sessionVariables.LIBVA_DRIVER_NAME = "iHD";

  # The NPU, through Level Zero.
  hardware.cpu.intel.npu.enable = true;

  # The mute key's light follows the switch PipeWire's mute flips, as
  # alsa-ucm-conf's cs42l45 profile would attach it if it ran as root. udev
  # writes the attachment itself and runs nothing to do it. Sound Open Firmware
  # loads the LED layer before the card registers, and the kernel makes the
  # card's attach file in the same call that announces its controls, so the
  # file is there by the time udev handles that event. The card must be the
  # first one, which it is: the codec's is the only card the laptop has.
  services.udev.extraRules = ''
    SUBSYSTEM=="sound", KERNEL=="controlC0", ACTION=="add", ATTRS{id}=="sofsoundwire", ATTR{[sound/ctl-led]mic/card0/attach}="cs42l45 FU 113 Channel Switch"
  '';

  # The ambient light and presence sensors, behind the Integrated Sensor Hub.
  hardware.sensor.iio.enable = true;

  # Intel's thermal daemon, which sets the chip's power limits from the
  # laptop's own thermal tables as it heats.
  services.thermald.enable = true;

  # The profiles' energy preferences, with performance's eased; see above.
  services.power-profiles-daemon.package = powerProfiles;

  # Each core's frequency and idle states and the package's power, which is
  # how the power limits and profiles here were measured.
  environment.systemPackages = [ turbostat ];

  # Authorizes Thunderbolt and USB4 docks.
  services.hardware.bolt.enable = true;

  # Dell publishes this laptop's BIOS and firmware updates through LVFS.
  services.fwupd.enable = true;

  # The battery stops charging at 80% and starts again below 75%: a lithium
  # cell ages fastest held near full, and with the starting point that close
  # it leaves the charger near 80% however briefly it was plugged in. The
  # limits hold only in the custom mode, and the firmware won't change them
  # until it's in it. With a BIOS password set, the firmware refuses them
  # through the kernel's charge_types and thresholds, which can't send the
  # password, so they're set here with it.
  jitsusama.firmware.settings = {
    PrimaryBattChargeCfg = "Custom";
    CustomChargeStart = "75";
    CustomChargeStop = "80";
  };

  # The BE211's Wi-Fi 7 receive path drops links to the slowest rate, so it
  # runs as Wi-Fi 6, as Omarchy runs it. Remove this once a Wi-Fi 7 link holds
  # its rate without it.
  boot.extraModprobeConfig = "options iwlwifi disable_11be=Y";
}
