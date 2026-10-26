# The checks of dell-xps-14-da14260, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  account,
  bareNixos,
  graphical,
  holds,
  lib,
  onLinux,
  pkgs,
  self,
  ...
}:
let
  # The XPS 14's kernel, and the camera's drivers built beside it.
  inherit ((bareNixos [ self.nixosModules.dell-xps-14-da14260 ]).config.boot.kernelPackages)
    kernel
    ipu7-drivers
    ;
  # A few files of a source, with each patch given applied to them, as the
  # machine builds them (tests/patched-kernel-source.sh).
  patched =
    source: patches: files:
    pkgs.runCommandLocal "patched-source" { nativeBuildInputs = [ pkgs.patchutils ]; } ''
      ${lib.getExe pkgs.bash} ${../../tests/patched-kernel-source.sh} ${source} ${
        pkgs.writeText "patches" (lib.concatMapStrings (patch: "${patch}\n") patches)
      } $out ${lib.escapeShellArgs files}
    '';
  # A few files of the kernel's source, as the XPS 14 builds them.
  kernelSource = patched kernel.src (
    lib.filter (patch: patch != null) (map (patch: patch.patch) kernel.kernelPatches)
  );
in
{
  # A model runs on the NPU only once its compiler has turned it into the
  # NPU's program, and the driver looks for the compiler in its own
  # directory, so both have to be in the graphics driver directory that
  # /run/opengl-driver links to. Anywhere else, the driver never finds it.
  the-npu-finds-its-compiler =
    let
      inherit ((bareNixos [ self.nixosModules.dell-xps-14-da14260 ]).config.systemd.tmpfiles) settings;
      drivers = settings.graphics-driver."/run/opengl-driver"."L+".argument;
    in
    pkgs.runCommandLocal "the-npu-finds-its-compiler" { } ''
      for f in libze_intel_npu.so.1 libopenvino_intel_npu_compiler_loader.so \
        libopenvino_intel_npu_compiler.so; do
        [ -e ${drivers}/lib/$f ] ||
          { echo "The graphics drivers lack $f, so the NPU can't compile a model." >&2; exit 1; }
      done
      touch $out
    '';

  # The GPU's security controller waits 20 s from xe's start for its link
  # to the CSME and never tries again, so the link's drivers load in the
  # initrd with xe rather than after the disk's PIN.
  the-gpu-security-link-loads-with-the-gpu =
    let
      inherit ((bareNixos [ self.nixosModules.dell-xps-14-da14260 ]).config.boot.initrd) kernelModules;
    in
    holds "the-gpu-security-link-loads-with-the-gpu"
      "The GPU's link to the CSME doesn't load with xe, so a slow PIN leaves the GPU's security controller without it."
      (
        lib.all (module: lib.elem module kernelModules) [
          "xe"
          "mei_me"
          "mei_gsc_proxy"
        ]
      );

  # When xe's boot-time read of the panel's eDP revision comes back wrong,
  # it drives brightness by PWM or Intel's interface, which the XPS 14's
  # OLED both ignore, unless it's held to VESA's controls.
  the-panel-dims-after-a-bad-read =
    let
      inherit ((bareNixos [ self.nixosModules.dell-xps-14-da14260 ]).config.boot) kernelParams;
    in
    holds "the-panel-dims-after-a-bad-read"
      "xe isn't held to VESA's brightness controls, so a bad read of the panel leaves its brightness keys doing nothing."
      (lib.elem "xe.enable_dpcd_backlight=2" kernelParams);

  # Under a build, reclaim backed up the XPS 14's window buffers at xe's
  # default seeks, and the compositor waited on their locks; at 64 it
  # backed up none.
  the-window-buffers-stay-under-load =
    let
      inherit ((bareNixos [ self.nixosModules.dell-xps-14-da14260 ]).config.services.udev) extraRules;
    in
    holds "the-window-buffers-stay-under-load"
      "xe's non-purgeable seeks aren't set to 64, so reclaim under a build backs up window buffers and the compositor waits on them."
      (lib.hasInfix ''DRIVER=="xe", ATTR{shrinker_non_purgeable_seeks}="64"'' extraRules);

  greetd-waits-for-the-gpu =
    let
      xps = bareNixos [
        self.nixosModules.graphical
        self.nixosModules.dell-xps-14-da14260
      ];
      inherit (xps.config) jitsusama services systemd;
      greetd = systemd.services.greetd;
    in
    holds "greetd-waits-for-the-gpu"
      "greetd doesn't wait for ${jitsusama.gpu.unit}, nothing tags the card for systemd, or nothing caps the wait."
      (
        greetd.serviceConfig.Type == "simple"
        && lib.hasInfix "JobRunningTimeoutSec=15s" systemd.units.${jitsusama.gpu.unit}.text
        && lib.elem jitsusama.gpu.unit greetd.wants
        && lib.elem jitsusama.gpu.unit greetd.after
        && lib.hasInfix ''ENV{ID_PATH}=="${jitsusama.gpu.path}", TAG+="systemd"'' services.udev.extraRules
        && graphical.config.systemd.services.greetd.serviceConfig.Type == "idle"
      );

  # The XPS 14's firmware holds its chosen settings at each boot, but not the
  # keyboard light's: KeyboardIllumination is the level the light key sets,
  # so holding it failed the boot's check whenever the key had been used.
  firmware-leaves-the-keyboard-light =
    let
      xps = bareNixos [ self.nixosModules.dell-xps-14-da14260 ];
      inherit (xps.config.jitsusama.firmware) settings;
    in
    holds "firmware-leaves-the-keyboard-light"
      "The XPS 14 holds KeyboardIllumination, which its light key sets, or lost IntelSagv."
      (settings ? IntelSagv && !(settings ? KeyboardIllumination));

  # thermald reads the XPS 14's tables, sensors and the CPU's power limits
  # once, as it starts, and stops if its adaptive mode finds no zone, which
  # it did when udev loaded their drivers after it started.
  thermald-starts-after-its-drivers =
    let
      xps = bareNixos [ self.nixosModules.dell-xps-14-da14260 ];
      inherit (xps.config) boot systemd;
      early = [
        "intel_rapl_msr"
        "coretemp"
        "int3400_thermal"
        "int3403_thermal"
        "processor_thermal_device_pci"
      ];
    in
    holds "thermald-starts-after-its-drivers"
      "thermald can start before a driver it reads, or a restart can't run it adaptive again."
      (
        lib.all (module: lib.elem module boot.kernelModules) early
        && (systemd.services.thermald.serviceConfig.Restart or null) == "on-failure"
        && lib.hasInfix "ignore_adaptive" (systemd.services.thermald.serviceConfig.ExecStartPre or "")
      );

  # The XPS 14's camera takes Dell's tuning only for a board a machine names,
  # and otherwise keeps Intel's, fetching nothing from Dell.
  the-camera-takes-its-boards-tuning =
    let
      camera =
        board:
        (bareNixos [
          self.nixosModules.dell-xps-14-da14260
          { jitsusama.dellXps14.cameraBoard = board; }
        ]).config.systemd.services.camera.environment;
      named = camera "CJFOE90";
      unnamed = camera null;
    in
    holds "the-camera-takes-its-boards-tuning"
      "The camera doesn't read Dell's tuning for the board named, or reads one unasked."
      (
        lib.hasSuffix "-camera-config-CJFOE90/" (named.CAMERA_CFG_PATH or "")
        && !(unnamed ? CAMERA_CFG_PATH)
      );

  # The image processor sends full-range video, and Chrome reads a camera's
  # frames as limited range whatever they're labelled, so the relay has to
  # squeeze them exactly, and neither of its converters may touch them after.
  # The relay's own two pipelines run here on a full-range ramp, the camera
  # swapped for a file with no colour label, as the camera gives none. Each
  # file's caps are pinned as given, as the camera's and the relay's appsrc
  # are, since a file source would otherwise take whatever label is asked of
  # it.
  the-camera-gives-limited-range =
    let
      xps = bareNixos [
        self.nixosModules.dell-xps-14-da14260
        { nixpkgs.config.allowUnfree = true; }
      ];
      halves = builtins.match ''.* -i "([^"]*)" -o "([^"]*)".*'' xps.config.systemd.services.camera.script;
      input = builtins.elemAt halves 0;
      output = builtins.elemAt halves 1;
      caps = builtins.head (builtins.match "appsrc name=appsrc caps=([^ ]*) .*" output);
      size = "format=NV12,width=1920,height=1080,framerate=30/1";
      pinned =
        file: caps:
        "multifilesrc location=${file} num-buffers=1 caps=${caps} ! capssetter replace=true caps=${caps}";
      ramp = pinned "ramp.nv12" "video/x-raw,${size}";
      relayed = builtins.replaceStrings [ "icamerasrc" ] [ ramp ] input;
      written = builtins.head (
        builtins.match "appsrc name=appsrc caps=[^ ]* ! (.*) ! v4l2sink .*" output
      );
      # What the loopback asks a writer for: BT.601 limited, V4L2's default.
      loopback = "video/x-raw,colorimetry=(string)2:4:7:1";
      plugins = lib.makeSearchPathOutput "lib" "lib/gstreamer-1.0" [
        pkgs.gst_all_1.gstreamer.out
        pkgs.gst_all_1.gst-plugins-base
        pkgs.gst_all_1.gst-plugins-good
      ];
      launch = lib.getExe' pkgs.gst_all_1.gstreamer.bin "gst-launch-1.0";
    in
    assert halves != null;
    pkgs.runCommandLocal "the-camera-gives-limited-range"
      {
        nativeBuildInputs = [ pkgs.python3 ];
        GST_PLUGIN_SYSTEM_PATH_1_0 = plugins;
        GST_REGISTRY = "registry.bin";
      }
      ''
        python3 ${../../tests/camera-range.py} ramp ramp.nv12
        # gst-launch takes each argument as one word, so the pipelines are
        # split at their spaces, which parses them as the relay's own
        # gst_parse_launch does.
        relayed=${lib.escapeShellArg "${relayed} ! ${caps} ! filesink location=relayed.nv12"}
        written=${lib.escapeShellArg "${pinned "relayed.nv12" caps} ! ${written} ! ${loopback} ! filesink location=written.nv12"}
        ${launch} -q $relayed 2> relayed.log || { cat relayed.log; exit 1; }
        ${launch} -q $written 2> written.log || { cat written.log; exit 1; }
        python3 ${../../tests/camera-range.py} compare ramp.nv12 relayed.nv12 written.nv12
        touch $out
      '';

  # niri maps sRGB onto the panel's own primaries for every program
  # (srgb-gamut), so a program told the panel's colours as well converts them
  # twice: it draws in P3, and niri maps that as if it were sRGB, washing
  # every colour out.
  chromium-leaves-the-panel-to-niri =
    let
      xps = bareNixos [
        self.nixosModules.dell-xps-14-da14260
        { nixpkgs.config.allowUnfree = true; }
      ];
      told = package: lib.hasInfix "force-color-profile" (package.postFixup or "");
      mapped = lib.elem "srgb-gamut" ((account graphical).jitsusama.niri.outputs."eDP-1" or [ ]);
    in
    holds "chromium-leaves-the-panel-to-niri"
      "Chrome, Slack or 1Password would convert the XPS 14's colours on top of niri's mapping, or niri would no longer map them."
      (
        mapped
        && !lib.any told [
          xps.pkgs.google-chrome
          xps.pkgs.slack
          xps.pkgs._1password-gui
        ]
      );
}
// onLinux {
  # The SoundWire machine driver binds each amplifier by the names of the
  # DAIs it lists for it, and one the amplifiers' driver doesn't register
  # leaves no sound card at all. 7.2.11 named the OT25 DAI before it had
  # one. Read from the source the XPS 14 builds, with its patches
  # (tests/amps-have-every-dai-named.sh).
  speaker-amps-have-every-dai-named =
    let
      codec = "sound/soc/codecs/cs35l56.c";
      machine = "sound/soc/sdw_utils/soc_sdw_utils.c";
    in
    pkgs.runCommandLocal "speaker-amps-have-every-dai-named" { } ''
      cd ${
        kernelSource [
          codec
          machine
        ]
      }
      ${lib.getExe pkgs.bash} ${../../tests/amps-have-every-dai-named.sh} ${machine} ${codec}
      touch $out
    '';

  # The XPS 14's EC sends type 0x0012 events with code 0x0000 about hourly,
  # and dell-wmi logs each as an unknown key unless its keymap for that
  # type ignores the code. Read from the XPS 14's patched source
  # (tests/dell-wmi-ignores-the-hourly-event.py).
  dell-events-are-known =
    let
      driver = "drivers/platform/x86/dell/dell-wmi-base.c";
    in
    pkgs.runCommandLocal "dell-events-are-known" { nativeBuildInputs = [ pkgs.python3 ]; } ''
      python3 ${../../tests/dell-wmi-ignores-the-hourly-event.py} ${kernelSource [ driver ]}/${driver}
      touch $out
    '';

  # The XPS 14's colour sensor, OVTI08F4, is mounted upside down, and its
  # firmware says it isn't. The bridge reports 180 degrees for a sensor in
  # its upside-down table that matches the machine, whatever the firmware
  # says, and nothing else says so. Read from the XPS 14's patched source
  # (tests/ipu-bridge-knows-the-upside-down-sensor.py).
  the-colour-camera-says-it-is-upside-down =
    let
      bridge = "drivers/media/pci/intel/ipu-bridge.c";
    in
    pkgs.runCommandLocal "the-colour-camera-says-it-is-upside-down"
      { nativeBuildInputs = [ pkgs.python3 ]; }
      ''
        python3 ${../../tests/ipu-bridge-knows-the-upside-down-sensor.py} \
          ${kernelSource [ bridge ]}/${bridge}
        touch $out
      '';

  # The legacy profile file, the one power-profiles-daemon writes, offers
  # only what every profile driver takes, so a profile Dell's driver offers
  # that the SoC slider doesn't never reaches either. Without quiet there,
  # power-saver runs as balanced. Read from the XPS 14's patched source
  # (tests/platform-profiles-reach-the-legacy-file.sh).
  platform-profiles-reach-the-legacy-file =
    let
      dell = "drivers/platform/x86/dell/dell-pc.c";
      slider = "drivers/thermal/intel/int340x_thermal/processor_thermal_soc_slider.c";
    in
    pkgs.runCommandLocal "platform-profiles-reach-the-legacy-file" { } ''
      cd ${
        kernelSource [
          dell
          slider
        ]
      }
      ${lib.getExe pkgs.bash} ${../../tests/platform-profiles-reach-the-legacy-file.sh} ${dell} ${slider}
      touch $out
    '';

  # The capture side of the XPS 14's image processor, and the ports its video nodes sit
  # in, are freed at the V4L2 device's last reference rather than with the
  # device, since a file open on one of its nodes outlives an unbind and
  # reaches them as it closes.
  ipu7-outlives-its-open-files =
    let
      isys = "drivers/staging/media/ipu7/ipu7-isys.c";
    in
    pkgs.runCommandLocal "ipu7-outlives-its-open-files" { } ''
      cd ${kernelSource [ isys ]}
      fail() { echo "The IPU7's capture side $1, so a node still open after an unbind reads freed memory as it closes."; exit 1; }
      grep -q 'sizeof(\*isys)' ${isys} || fail "isn't allocated where this check looks"
      ! grep -q 'devm_kzalloc(&auxdev->dev, sizeof(\*isys)' ${isys} || fail "is freed with its device"
      ! grep -q 'devm_kcalloc' ${isys} || fail "has its CSI-2 ports freed with its device"
      grep -q 'v4l2_dev.release = ' ${isys} || fail "sets no release for its V4L2 device"
      touch $out
    '';

  # Both halves of the XPS 14's camera asked for 4 MiB of contiguous memory
  # at probe, for a firmware log only Intel's offline logger writes, and the
  # capture side failed its probe without it, and the capture side kept its
  # firmware's boot memory after every unbind. Read from the patched
  # sources, the logger's code taken out as it is when built.
  the-camera-holds-no-memory-it-never-uses =
    let
      isys = "drivers/staging/media/ipu7/ipu7-isys.c";
      psys = "drivers/media/pci/intel/ipu7/psys/ipu-psys.c";
      sources = pkgs.symlinkJoin {
        name = "camera-sources";
        paths = [
          (kernelSource [ isys ])
          (patched ipu7-drivers.src ipu7-drivers.patches [ psys ])
        ];
      };
    in
    pkgs.runCommandLocal "the-camera-holds-no-memory-it-never-uses"
      { nativeBuildInputs = [ pkgs.unifdef ]; }
      ''
        for file in ${isys} ${psys}; do
          install -D -m 644 ${sources}/$file $file
          unifdef -m -UENABLE_FW_OFFLINE_LOGGER $file || [ $? = 1 ]
        done
        moved() { echo "The camera's $1 side isn't probed where this check looks."; exit 1; }
        asks() { echo "The camera's $1 side asks for 4 MiB for a log nothing reads."; exit 1; }
        grep -q 'isys_register_devices(isys)' ${isys} || moved capture
        grep -q 'ipu7_psys_init_debugfs(psys)' ${psys} || moved processing
        ! grep -q 'FW_LOG_BUF_SIZE' ${isys} || asks capture
        ! grep -q 'psys_fw_log_init(psys)' ${psys} || asks processing
        keeps() { echo "The camera keeps its firmware's memory past an unbind."; exit 1; }
        awk '/^static void isys_remove\(/,/^}/' ${isys} > remove.c
        grep -q 'ipu7_fw_isys_release(isys);' remove.c || keeps
        touch $out
      '';

  # The XPS 14's firmware gives Bluetooth's power limits and antenna gain
  # setting only as UEFI variables, in a revision 2 table, so a kernel that
  # reads only ACPI's, or rejects revision 2, runs Bluetooth without them.
  # Read from the XPS 14's patched source.
  bluetooth-keeps-its-power-limits =
    let
      btintel = "drivers/bluetooth/btintel.c";
      without = what: "The kernel ${what}, so Bluetooth runs without its power limits.";
    in
    pkgs.runCommandLocal "bluetooth-keeps-its-power-limits" { } ''
      cd ${kernelSource [ btintel ]}
      if ! grep -q 'btintel_set_sar_rev2(hdev, sar)' ${btintel}; then
        echo "${without "rejects revision 2 Bluetooth SAR tables"}"
        exit 1
      fi
      if ! grep -Eq '^[[:space:]]+ret = btintel_uefi_set_sar\(hdev, ver\);' ${btintel}; then
        echo "${without "never reads Bluetooth's SAR table from UEFI"}"
        exit 1
      fi
      if ! grep -Eq '^[[:space:]]+if \(!btintel_uefi_get_ppag\(&mode\)\)' ${btintel}; then
        echo "${without "never reads Bluetooth's antenna gain setting from UEFI"}"
        exit 1
      fi
      touch $out
    '';

  # The XPS 14's panel gives its variable refresh range only in DisplayID's
  # Adaptive Sync block, so a kernel that doesn't read it never offers
  # variable refresh. Read from the XPS 14's patched source.
  the-panel-offers-variable-refresh =
    let
      edid = "drivers/gpu/drm/drm_edid.c";
      displayid = "drivers/gpu/drm/drm_displayid_internal.h";
    in
    pkgs.runCommandLocal "the-panel-offers-variable-refresh" { } ''
      cd ${
        kernelSource [
          edid
          displayid
        ]
      }
      if ! grep -Eq '^#define DATA_BLOCK_2_ADAPTIVE_SYNC[[:space:]]+0x2b$' ${displayid}; then
        echo "The kernel doesn't know DisplayID's Adaptive Sync block, so the panel's variable refresh is never offered."
        exit 1
      fi
      if ! grep -q 'block->tag != DATA_BLOCK_2_ADAPTIVE_SYNC' ${edid}; then
        echo "The kernel reads no refresh range from DisplayID's Adaptive Sync block, so the panel's variable refresh is never offered."
        exit 1
      fi
      touch $out
    '';

  # The XPS 14's panel gives its luminance only in DisplayID's Display
  # Parameters block, so a kernel that doesn't read it guesses the panel's
  # brightness range. Read from the XPS 14's patched source.
  the-panel-knows-its-luminance =
    let
      edid = "drivers/gpu/drm/drm_edid.c";
      displayid = "drivers/gpu/drm/drm_displayid_internal.h";
    in
    pkgs.runCommandLocal "the-panel-knows-its-luminance" { } ''
      cd ${
        kernelSource [
          edid
          displayid
        ]
      }
      guess() {
        echo "The kernel $1, so the panel's brightness range is a guess."
        exit 1
      }
      grep -q 'block->tag != DATA_BLOCK_2_DISPLAY_PARAMETERS' ${edid} ||
        guess "reads no luminance from DisplayID's Display Parameters block"
      grep -Eq '^[[:space:]]+drm_get_luminance_range_displayid\(connector,' ${edid} ||
        guess "never asks DisplayID for the panel's luminance"
      touch $out
    '';
}
