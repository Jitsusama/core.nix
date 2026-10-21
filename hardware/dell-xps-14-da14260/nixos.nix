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
  # and woke and burst from idle as quickly; docs/hardware.md has the
  # numbers. Its test of Intel's driver expects the value the profile writes,
  # so it changes too. Built here rather than taken from the cache, the
  # daemon runs its own tests, two of which read a value before it settles
  # (power-profiles-tests-wait.patch).
  powerProfiles = pkgs.power-profiles-daemon.overrideAttrs (old: {
    patches = (old.patches or [ ]) ++ [ ./power-profiles-tests-wait.patch ];
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
    ../../modules/device-check/nixos.nix
    ../../modules/firmware-settings/nixos.nix
    ../../modules/gpu/nixos.nix
    ../../modules/kernel/nixos.nix
    ../../modules/keyboard/nixos.nix
    ../../modules/panel/nixos.nix
    ../../modules/power-profiles-daemon/nixos.nix
    ../../modules/touchpad/nixos.nix
    ./camera.nix
    ./speakers.nix
  ];

  nixpkgs.hostPlatform = "x86_64-linux";
  jitsusama.kernel.cpu = "pantherlake";

  # The devices this laptop always has. The USB-C controller, the speakers'
  # amplifiers and the camera's ISP have each failed to come up here before.
  # Wi-Fi's number moves if its driver starts again.
  jitsusama.devices.expected = {
    "The USB-C ports" = "/sys/class/typec/port0";
    "Bluetooth" = "/sys/class/bluetooth/hci0";
    "Wi-Fi" = "/sys/class/ieee80211/phy*";
    "The speakers" = "/proc/asound/sofsoundwire";
    "The camera's ISP" = "/sys/bus/auxiliary/devices/intel_ipu7.psys.*";
  };

  # The keyboard is wired to the embedded controller, which presents it as a
  # PS/2 keyboard.
  jitsusama.keyboard.builtIn = [ "0001:0001" ];

  # Its firmware's boot menu and setup, for the guide to say when a stick is
  # written for it.
  jitsusama.firmware.startFromStick = "Press F12 as it starts, and pick the stick from the boot menu.";
  jitsusama.firmware.clearSecureBootKeys = "Press F2 as it starts. Under Secure Boot, Expert Key Management, choose custom mode and delete all keys, then turn Secure Boot on.";

  # The Arc GPU. Its driver takes the firmware framebuffer's card away as it
  # starts, and has its own ready about 1.6 s later, so for that long there's
  # no card at all.
  jitsusama.gpu.path = "pci-0000:00:02.0";

  # The panel, LG Display's 0x07C6, shows a gamut wider than Display P3: its
  # EDID gives red 0.689/0.310, green 0.240/0.721 and blue 0.136/0.045. Dell's
  # Windows profile for it has the same primaries and no calibration curves,
  # so Windows only lets the programs that convert colours use it. P3 is the
  # nearest space Chromium names.
  jitsusama.panel.colourSpace = "display-p3-d65";

  # Microcode, and the firmware for Wi-Fi, Bluetooth, the GPU and the audio
  # DSP, which Sound Open Firmware runs.
  hardware.cpu.intel.updateMicrocode = true;
  hardware.enableRedistributableFirmware = true;

  # The Bluetooth chip's firmware and settings from linux-firmware's own
  # tree, which is ahead of its 20260916 release in two ways:
  # - Intel's firmware release 24.80.0.2 for this chip, from 1 October, where
  #   the release carries the build from July.
  # - The settings file, whose release copy ends in a stray newline. The
  #   kernel reads it as one record too many and rejects it with "Malformed
  #   DDC record" at every boot, after sending the records before it, so
  #   Bluetooth works either way.
  # The files keep their names and go first, so they're the ones found,
  # until nixpkgs takes a release that has them.
  hardware.firmware =
    let
      commit = "8a2941fae0d02364d8ae6542ba890fd496681b55";
      file =
        name: hash:
        pkgs.fetchurl {
          inherit name hash;
          url = "https://git.kernel.org/pub/scm/linux/kernel/git/firmware/linux-firmware.git/plain/intel/${name}?id=${commit}";
        };
      files = {
        "ibt-00a0-01a1-iml.sfi" = "sha256-ab63c4enmnXudB/2+I94Kg/ETIsu/Me8tBxBMSKSgCY=";
        "ibt-00a0-01a1-pci.sfi" = "sha256-VFrc2+gWzKWRI6CRK0WC9GDwmUK58MXeKmy9il6haA0=";
        "ibt-00a0-01a1-pci.ddc" = "sha256-ot+YgNGJUiDpv7uGsuI1hBnFaFa13MLWMAcTlp39B0Q=";
      };
    in
    lib.mkIf (lib.versionOlder pkgs.linux-firmware.version "20261002") (
      lib.mkBefore [
        (pkgs.runCommandLocal "ibt-00a0-01a1" { } (
          lib.concatStrings (
            lib.mapAttrsToList (name: hash: ''
              install -D -m 444 ${file name hash} $out/lib/firmware/intel/${name}
            '') files
          )
        ))
      ]
    );

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
      # The CVS bridge's wake interrupt leaves an active-low flag on that
      # same pin, so the first amplifier to read its speaker ID reads it
      # inverted and loads the other speakers' tuning: the right woofer,
      # at every boot. Written here; the patch says why.
      name = "gpio-acpi-irq-leaves-polarity";
      patch = ./gpio-acpi-irq-leaves-polarity.patch;
    }
    {
      # The four amplifiers probe at once and each takes that pin for itself,
      # so now and then one finds it taken and the sound card never comes.
      # Written here; the patch says why.
      name = "cs35l56-spkid-lock";
      patch = ./cs35l56-spkid-lock.patch;
    }
    {
      # The USB-C controller lists a charger's offers with an empty one
      # first, so the kernel reads each one a place late and reports the
      # wrong voltage. Written here; the patch says why.
      name = "ucsi-leading-empty-pdo";
      patch = ./ucsi-leading-empty-pdo.patch;
    }
    {
      # Now and then the USB-C controller stops answering while the kernel
      # starts it at boot, and the kernel gives up, leaving no USB-C ports
      # or charger reports until the next boot. Written here; the patch says
      # why.
      name = "ucsi-retry-timed-out-init";
      patch = ./ucsi-retry-timed-out-init.patch;
    }
    {
      # The USB-C controller answers "not supported" when asked for a
      # charger's sink offers, which it has none of, so the kernel logs an
      # error and never links the charger's offers to it. Written here; the
      # patch says why.
      name = "ucsi-partner-without-pdos";
      patch = ./ucsi-partner-without-pdos.patch;
    }
    {
      # The memory is soldered and has no SPD chip, but the kernel probes the
      # SMBus for one anyway: the first probe hangs the bus, the next seven
      # fail, and the SMBus driver stalls for 450 ms at every boot. Written
      # here; the patch says why.
      name = "spd-skips-soldered-memory";
      patch = ./spd-skips-soldered-memory.patch;
    }
    {
      # The encrypted disk could report a failed read or write as a success
      # when two completions raced. Mainline's fix, which 7.2's stable
      # releases have yet to take.
      name = "dm-crypt-status-after-last-reference";
      patch = ./dm-crypt-status-after-last-reference.patch;
    }
    {
      # This chip's Bluetooth power limits come as a revision 2 table, which
      # 7.2 rejects. Mainline's support for it, from 7.3.
      name = "bluetooth-sar-rev2";
      patch = ./bluetooth-sar-rev2.patch;
    }
    {
      # The firmware gives Bluetooth's power limits and antenna gain setting
      # only where Windows looks, not in ACPI where the kernel does, so
      # Bluetooth ran without them. Written here; the patch says why. It
      # needs the revision 2 patch before it.
      name = "bluetooth-uefi-power-tables";
      patch = ./bluetooth-uefi-power-tables.patch;
    }
    {
      # A suspend before an amplifier first attaches skipped a step its
      # resume always undoes, unbalancing its power management. Mainline's
      # fix, backported to 7.2's names.
      name = "cs35l56-suspend-before-attach";
      patch = ./cs35l56-suspend-before-attach.patch;
    }
    {
      # Dell's driver offers quiet and the SoC slider low-power, so the legacy
      # profile file offered neither, and power-saver ran as balanced.
      name = "soc-slider-takes-quiet";
      patch = ./soc-slider-takes-quiet.patch;
    }
    {
      # The panel gives its 20 to 120 Hz variable refresh range only where
      # the kernel doesn't look, so the driver never offers it. Written here;
      # the patch says why.
      name = "panel-variable-refresh";
      patch = ./panel-variable-refresh.patch;
    }
    {
      # The panel gives its 400 nit ceiling only where the kernel doesn't
      # look, so the driver guessed 512 and the brightest fifth of the range
      # asked for more than the panel is rated for. Written here; the patch
      # says why. It needs the variable refresh patch before it.
      name = "panel-luminance-range";
      patch = ./panel-luminance-range.patch;
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
  # The GPU's driver, with the firmware it asks for, so it takes the screen
  # from the firmware's framebuffer before the disk's PIN is asked for rather
  # than just after, which blanked the screen, and the greeter, which waits
  # for the GPU, starts as soon as the disk is open.
  boot.initrd.kernelModules = [ "xe" ];
  # The boot menu in the firmware's text mode 1, picked by eye on the panel
  # as the most readable of the modes its r key steps through.
  boot.loader.systemd-boot.consoleMode = "1";
  boot.kernelModules = [
    "kvm-intel"
    # thermald looks once, as it starts, for the power limits the CPU's own
    # registers hold, and udev loads the driver for them a second later, so
    # it warns they're missing. Loading it with the early modules, which
    # finish before thermald may start, has them there first.
    "intel_rapl_msr"
    # The same for the cores' temperatures. When udev loaded coretemp after
    # thermald started, thermald found no sensor, made no zones, and marked
    # itself to restart without its adaptive mode, which this CPU needs, so
    # it stayed down for the rest of the boot.
    "coretemp"
    # Its adaptive mode also needs the drivers for the laptop's thermal
    # tables, its sensors, and the CPU's own zone and power limits, which
    # the tables steer. At the boot it failed they came a second after
    # thermald, and a boot that lacked only coretemp still ran. Each probes
    # as it loads.
    "int3400_thermal"
    "int3403_thermal"
    "processor_thermal_device_pci"
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
  # Level Zero's loader checks a driver's file exists before it loads it,
  # and looks everywhere but where NixOS links the drivers, so without this
  # no program finds the GPU's or the NPU's driver unless LD_LIBRARY_PATH
  # names that directory. As an overlay, so whatever is built against the
  # loader gets the fixed one. Take it out once nixpkgs's package does it.
  nixpkgs.overlays = [
    (_final: prev: {
      level-zero = prev.level-zero.overrideAttrs (old: {
        patches = (old.patches or [ ]) ++ [ ./level-zero-driver-link.patch ];
        postPatch = (old.postPatch or "") + ''
          substituteInPlace source/loader/linux/driver_discovery_lin.cpp \
            --replace-fail @driverLink@ ${prev.addDriverRunpath.driverLink}
        '';
      });
    })
  ];

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

  # The presence sensor, behind the Integrated Sensor Hub. Its light sensors
  # are the backlight module's, which hears them itself.
  hardware.sensor.iio.enable = true;

  # Intel's thermal daemon, which sets the chip's power limits from the
  # laptop's own thermal tables as it heats. Stopping it slept two seconds,
  # at every shutdown and restart, where it now waits only for its own
  # thread to end (thermald-joins-its-thread.patch).
  services.thermald.enable = true;
  services.thermald.package = pkgs.thermald.overrideAttrs (old: {
    patches = (old.patches or [ ]) ++ [ ./thermald-joins-its-thread.patch ];
  });
  # When its adaptive mode can't run, thermald leaves itself a note and
  # stops, so that its next start runs without it. This CPU runs only in
  # adaptive mode, and thermald refuses it otherwise, so the note would keep
  # it down until the next boot. Each start clears the note, and a stop on
  # failure starts it again, adaptive, once whatever it lacked is there.
  # Three starts a minute at most, so one that never can stops trying.
  systemd.services.thermald = {
    serviceConfig = {
      ExecStartPre = "${lib.getExe' pkgs.coreutils "rm"} -f /run/thermald/ignore_adaptive";
      Restart = "on-failure";
      RestartSec = 2;
    };
    startLimitBurst = 3;
    startLimitIntervalSec = 60;
  };

  # The profiles' energy preferences, with performance's eased; see above.
  jitsusama.power-profiles-daemon.package = powerProfiles;

  # Each core's frequency and idle states and the package's power, which is
  # how the power limits and profiles here were measured.
  environment.systemPackages = [ turbostat ];

  # Authorizes Thunderbolt and USB4 docks. bolt 0.9.8 doesn't know Panther
  # Lake's USB4 controllers and warns at every boot before treating them as
  # they should be (bolt-panther-lake.patch).
  services.hardware.bolt.enable = true;
  services.hardware.bolt.package = pkgs.bolt.overrideAttrs (old: {
    patches = (old.patches or [ ]) ++ [ ./bolt-panther-lake.patch ];
  });

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

    # Settings chosen here that a BIOS update or reset would put back
    # without a word, as 1.11.0 turned the adapter warnings back off. The
    # thermal mode isn't one of them: it's the platform profile the power
    # profile follows (modules/power-profiles-daemon), so it's balanced's
    # Optimized after a boot on battery, and the charger puts back the
    # Performance the power limits were measured in. Nor is the keyboard
    # light: KeyboardIllumination is the level the light key sets, which
    # dell::kbd_backlight writes straight through (1 reads back Dim, 2
    # Bright), and systemd-backlight puts the last one back at each boot.
    # The memory's frequency drops when it's idle, which the battery
    # needs more than the memory's latency needs it pinned.
    IntelSagv = "Enabled";
    # No pause at power-on over a charger it thinks too weak.
    PowerWarn = "Disabled";
    # A bad BIOS can be rolled back.
    AllowBiosDowngrade = "Enabled";
    # Microsoft's presence detection, as the laptop ships and as the camera
    # was proven on. Intel's gave no presence reading either; see
    # docs/hardware.md.
    HPDSensor = "MSHPD";

    # None of Dell's ways in from outside: no network stack before the
    # loader, so no HTTP(S) boot and no BIOSConnect recovery from Dell's
    # servers, no SupportAssist taking the boot over after failed ones, no
    # software handed to the operating system and no telemetry. A machine
    # that won't boot starts from the USB stick instead, which needs only
    # USB boot. The firmware measures these settings into PCR 1 alone, which
    # no disk policy here binds, and BIOS recovery from the disk stays.
    UefiNwStack = "Disabled";
    HttpsBoot = "Disabled";
    BIOSConnect = "Disabled";
    SupportAssistOSRecovery = "Disabled";
    AutoOSRecoveryThreshold = "OFF";
    DcsdSwDvc = "Disabled";
    TelemetryAccessLvl = "Disabled";
  };

  # The BE211's Wi-Fi 7 receive path drops links to the slowest rate, so it
  # runs as Wi-Fi 6, as Omarchy runs it. Remove this once a Wi-Fi 7 link holds
  # its rate without it.
  boot.extraModprobeConfig = "options iwlwifi disable_11be=Y";
}
