# The front cameras: an OV08X40 behind Intel's CVS bridge, imaged by the IPU7's
# hardware ISP, and beside it the HM1092 IR camera, whose greyscale frames are
# captured raw. The kernel has the IPU's core and capture side and the bridge,
# nixpkgs's hardware.ipu7 adds the ISP's module and firmware, and Intel's camera
# HAL turns the sensor's raw frames into a picture with its tuning for this
# sensor. Programs only speak V4L2, so a relay plays the HAL's picture into a
# loopback camera they can open. nixos-hardware's profile for this laptop,
# https://github.com/NixOS/nixos-hardware/pull/1912, worked out every piece;
# docs/hardware.md says which.
{
  config,
  lib,
  pkgs,
  ...
}:
let
  # Intel's main branch, which knows the bridge, patched so this sensor's graph
  # goes through it.
  hal = pkgs.ipu75xa-camera-hal.overrideAttrs (old: {
    version = "0-unstable-2026-08-12";
    src = pkgs.fetchFromGitHub {
      owner = "intel";
      repo = "ipu7-camera-hal";
      rev = "11d8aff0d1ddc16aef56c8e6518e08e2f936a95b";
      hash = "sha256-NSZVVOZKa3xhwitdKw4EZpukf5B/ObQC4GEDwHMmZ6s=";
    };
    patches = old.patches or [ ] ++ [ ./camera-cvs.patch ];
  });
  icamerasrc = pkgs.gst_all_1.icamerasrc-ipu75xa.override { ipu7x-camera-hal = hal; };

  # The HAL's configuration with Dell's tuning for the laptop's camera module
  # in place of Intel's, from Dell's driver pack, which can't be shared, so
  # it's fetched as the system builds. The HAL reads the tuning the sensor's
  # file names, and the file is the same format on Linux as on Windows.
  module = config.jitsusama.dellXps14.cameraModule;
  driverPack = pkgs.fetchurl {
    url = "https://downloads.dell.com/FOLDER14363593M/1/XPS-14-DA14260-10KF7_Win11_1.0_A01.exe";
    hash = "sha256-xt2uMrziirUj2G5M/1XFhz8HvUFtgPTKvqIrX7K+xCA=";
  };
  tuning = "OV08X40_${module}_PTL";
  halConfig =
    pkgs.runCommand "camera-config-${module}"
      {
        nativeBuildInputs = [ pkgs._7zz ];
        meta.license = lib.licenses.unfree;
      }
      ''
        cp -r ${hal}/etc/camera/ipu75xa $out
        chmod -R u+w $out
        7zz e -o$out ${driverPack} 'XPS-14-DA14260/Win11/x64/camera/YMY0G_A00-00/3.IPU/${tuning}.aiqb' > /dev/null
        if [ ! -e $out/${tuning}.aiqb ]; then
          echo "Dell's driver pack has no tuning for camera module ${module}."
          exit 1
        fi
        substituteInPlace $out/sensors/ov08x40-uf.json \
          --replace-fail '"OV08X40_KAFE799_PTL"' '"${tuning}"'
      '';

  # Intel's IPU drivers, which hardware.ipu7 builds the ISP's module from. That
  # module shares structs with the kernel's own IPU driver and is written for a
  # kernel patched to match it, so the kernel patch and the module come from
  # this one revision.
  drivers = pkgs.fetchFromGitHub {
    owner = "intel";
    repo = "ipu7-drivers";
    rev = "495acc90feb09d8008c0a6228fb8bb4c6415ca62";
    hash = "sha256-a2hIJ4wMCHQeDb4gp+5pjLizJ/CCfA0JivVDWeqB4vY=";
  };

  # The kernel's IPU driver, patched as the ISP's module needs.
  ipuPatches = [
    {
      # The ISP's module locks a mutex and sets a callback at the end of the
      # kernel's device struct, which the kernel's driver doesn't have: it
      # writes past the end of what the kernel allocated, into whatever lies
      # next. From Intel's drivers, whose IoT release adds it to the kernel
      # for the module; it also has the MMU wait for the firmware's running
      # tasks before dropping their mappings.
      name = "ipu7-fw-task-lock";
      patch = "${drivers}/patch/v7.0.0_iot/0001-ipu7-Wait-ipu-fw-requests-to-clear.patch";
    }
    {
      # The wait above lasts until the firmware hands back every frame it was
      # given, and a stopped stream never hands back the frames still queued,
      # so every mapping dropped as the camera closed waited out 110 ms: 32 of
      # them, 3.5 s, each with a "wait running fw tasks clear timeout".
      # This takes a stream's frames back once the firmware says it's closed.
      # The next patch in Intel's same release.
      name = "ipu7-reclaim-stream-msgs";
      patch = "${drivers}/patch/v7.0.0_iot/0002-staging-ipu7-reclaim-pending-fw-msg-buffers-by-strea.patch";
    }
    {
      # Without it one failed wake leaves the ISP failed until the next boot,
      # and unbinding the driver oopses holding the device's lock, so suspend
      # and reboot hang. Written here; the patch says why.
      name = "ipu7-survives-resume";
      patch = ./ipu7-survives-resume.patch;
    }
  ];

  # The ISP's module, built against the kernel's own headers rather than
  # Intel's copies of them, so a struct can't be laid out differently on each
  # side again: a field the kernel lacks fails the build instead. Every header
  # the kernel also has is replaced, patched as the kernel is, and the build
  # stops if the bus header, the one that drifted, wasn't among them.
  psys = final: prev: {
    ipu7-drivers = prev.ipu7-drivers.overrideAttrs (old: {
      src = drivers;
      patches = old.patches or [ ] ++ [
        # Without it a resume that couldn't wake the ISP leaves its firmware
        # unstarted on every open, and the camera dark, until the next boot.
        # Written here; the patch says why.
        ./ipu7-psys-authenticates.patch
        # The same 4 MiB for a log only Intel's offline logger writes, which
        # warns whenever a rebind can't find it. Written here; the patch says
        # why.
        ./ipu7-psys-skips-unread-log.patch
      ];
      postPatch = old.postPatch or "" + ''
        mkdir kernel
        tar -xf ${final.kernel.src} -C kernel --strip-components=1 \
          --wildcards '*/drivers/staging/media/ipu7/*'
        ${lib.concatMapStrings (p: "patch -d kernel -p1 < ${p.patch}\n") ipuPatches}
        for header in drivers/media/pci/intel/ipu7/*.h drivers/media/pci/intel/ipu7/abi/*.h; do
          ours=kernel/drivers/staging/media/ipu7/''${header#drivers/media/pci/intel/ipu7/}
          if [ -e "$ours" ]; then cp "$ours" "$header"; fi
        done
        cmp kernel/drivers/staging/media/ipu7/ipu7-bus.h drivers/media/pci/intel/ipu7/ipu7-bus.h
      '';
    });
  };

  plugins = lib.makeSearchPathOutput "lib" "lib/gstreamer-1.0" [
    pkgs.gst_all_1.gstreamer.out
    pkgs.gst_all_1.gst-plugins-base
    pkgs.gst_all_1.gst-plugins-good
    icamerasrc
  ];
  loopback = lib.getExe' config.jitsusama.v4l2loopback.package.bin "v4l2loopback-ctl";
  device = "/run/camera/device";

  # Dell's tuning has the image processor send full-range video, Y from 0 to
  # 255, but Chrome and every Electron app ignore a camera's colour label and
  # read its frames as BT.601 limited range, 16 to 235, so they clip about a
  # tenth of the picture to black or white. Squeezing the picture into limited
  # range makes it what they read. This is exactly Y 16 + 219y/255 and chroma
  # 128 + 224(c - 128)/255, for about 3% of a core. The label says so from the
  # start, so neither videoconvert has anything to convert.
  limited = "colorimetry=(string)2:4:7:1";
  squeeze = "videobalance contrast=0.858824 brightness=0.053887 saturation=0.878431";

  # The sensor is mounted upside down. Turning the picture around, rather than
  # mirroring it, keeps it the way others should see it; calls mirror Joel's
  # own preview themselves.
  input = lib.concatStringsSep " ! " [
    "icamerasrc"
    "capssetter caps=video/x-raw,${limited}"
    "videoconvert"
    "videoscale"
    "videoflip method=rotate-180"
    squeeze
  ];
  # The ISP scales for free, and 1080p moves a quarter of 4K's bytes through
  # the loopback. A leaky queue drops a stale frame rather than delaying the
  # next one.
  output = lib.concatStringsSep " ! " [
    "appsrc name=appsrc caps=video/x-raw,format=NV12,width=1920,height=1080,framerate=30/1,${limited}"
    "queue leaky=downstream max-size-buffers=3"
    "videoconvert"
    "v4l2sink name=v4l2sink device=$(cat ${device}) sync=false"
  ];

  # Wires the IR sensor's pipeline before each stream, and links the relay's
  # source to the node it ends at. The firmware fills in the sensor's CSI-2
  # port at boot, and models differ, so the port is read from the sensor's own
  # link rather than assumed. The links and formats last until the next boot,
  # and the HAL touches only the colour camera's, so wiring them again is
  # only ever setting them to what they are.
  irPipeline = pkgs.writeShellApplication {
    name = "ir-camera-pipeline";
    runtimeInputs = [
      pkgs.gawk
      pkgs.v4l-utils
    ];
    text = ''
      format=Y10_1X10/648x368

      media=
      for device in /sys/bus/media/devices/media*; do
        if [ "$(cat "$device/model")" = ipu7 ]; then
          media=/dev/''${device##*/}
        fi
      done
      sensor=
      for node in /sys/class/video4linux/v4l-subdev*; do
        name=$(cat "$node/name")
        case $name in "hm1092 "*) sensor=$name ;; esac
      done
      if [ -z "$media" ] || [ -z "$sensor" ]; then
        echo "The IR sensor hasn't bound to the IPU7." >&2
        exit 1
      fi

      topology=$(media-ctl -d "$media" -p)
      # The entity a pad of an entity's links out to, its first if several.
      target() {
        awk -v entity="$1" -v pad="$2:" '
          /^- entity / { inside = index($0, ": " entity " (") > 0; at = "" }
          inside && /^\tpad[0-9]+:/ { at = $1 }
          inside && at == pad && /^\t\t-> "/ { split($0, part, "\""); print part[2]; exit }
        ' <<<"$topology"
      }
      port=$(target "$sensor" pad0)
      capture=$(target "$port" pad1)
      if [ -z "$port" ] || [ -z "$capture" ]; then
        echo "No capture node follows $sensor." >&2
        exit 1
      fi

      media-ctl -d "$media" -l "\"$port\":1 -> \"$capture\":0 [1]"
      media-ctl -d "$media" -V "\"$sensor\":0 [fmt:$format]"
      media-ctl -d "$media" -V "\"$port\":0 [fmt:$format]"
      ln -sfn "$(media-ctl -d "$media" -e "$capture")" /run/ir-camera/source
    '';
  };
in
{
  imports = [
    ./camera-module.nix
    ../../modules/camera/nixos.nix
    ../../modules/ir-camera/nixos.nix
    ../../modules/kernel/nixos.nix
    ../../modules/v4l2loopback/nixos.nix
  ];

  # Pictures and videos are taken from the loopback, which the file names.
  jitsusama.camera.device = device;

  # The IR camera, for any program that reads one. Its sensor's single mode.
  jitsusama.irCamera = {
    enable = true;
    source = "/run/ir-camera/source";
    format = "Y10";
    width = 648;
    height = 368;
    prepare = lib.getExe irPipeline;
  };
  # Howdy's own threshold rejects every frame here: in the IR light, about
  # three in four of a frame's pixels read near black, face and all.
  services.howdy.settings.video.dark_threshold = lib.mkOverride 900 90;

  hardware.ipu7 = {
    enable = true;
    platform = "ipu75xa";
  };
  # Its relay makes the loopback with the default buffers, which nixos-hardware
  # measured at a few frames a second, so the camera service below runs the
  # relay instead.
  services.v4l2-relayd.instances.ipu7.enable = false;

  boot.kernelPatches = ipuPatches ++ [
    {
      # The camera's privacy light stays off while it streams, since the
      # bridge's firmware lights it only when it owns it. From the media list,
      # and tested by Intel on this bridge.
      name = "cvs-privacy-led";
      patch = pkgs.fetchpatch {
        name = "cvs-privacy-led.patch";
        url = "https://patchwork.linuxtv.org/project/linux-media/patch/20261008-cvs-privacy-led-v1-1-b78d5c512cba@gmail.com/mbox/";
        hash = "sha256-wr8GO6xBzuoLJe3zJP00f9AkC6YiNh4G0B+4bewhFbM=";
      };
    }
    {
      # The bridge's firmware sometimes answers a handover of the camera's
      # link late, and the kernel took one late answer as a failure it never
      # clears, so the camera gave black frames until the next boot. Written
      # here; the patch says why.
      name = "cvs-link-owner-wait";
      patch = ./cvs-link-owner-wait.patch;
    }
    # The IR camera Windows signs in with: an HM1092 on one of the IPU7's
    # CSI-2 ports, powered by its own INT3472. The four patches below, three
    # from the media list and one written here, take it from unknown to
    # streaming greyscale frames.
    {
      # Gives the sensor its link frequencies, without which it never binds.
      # From mainline.
      name = "ipu-bridge-hm1092";
      patch = ./ipu-bridge-hm1092.patch;
    }
    {
      # The colour sensor is mounted upside down, but the firmware says 0,
      # so its rotation control said so too. Written here, after the quirk
      # linux-next has for the Pro 14 Premium, the same sensor.
      name = "ipu-bridge-da14260-upside-down";
      patch = ./ipu-bridge-da14260-upside-down.patch;
    }
    {
      # The sensor's driver, which no release has yet.
      name = "hm1092-ir-sensor";
      patch = ./hm1092-ir-sensor.patch;
      structuredExtraConfig.VIDEO_HM1092 = lib.kernel.module;
    }
    {
      # Lights the IR flood LED while the sensor streams, and only then, so
      # its frames aren't dark. Written here; the patch says why.
      name = "hm1092-ir-flood";
      patch = ./hm1092-ir-flood.patch;
    }
    {
      # The capture side's greyscale formats, without which a mono sensor
      # can't stream.
      name = "ipu7-mono-formats";
      patch = ./ipu7-mono-formats.patch;
    }
    {
      # Unbinding the IPU frees the capture side while WirePlumber still
      # holds its nodes open, and the kernel oopses as they close. Written
      # here; the patch says why.
      name = "ipu7-outlives-open-files";
      patch = ./ipu7-outlives-open-files.patch;
    }
    {
      # The capture side's probe fails without 4 MiB of contiguous memory for
      # a firmware log nothing reads, so a rebind on fragmented memory loses
      # the camera. Written here; the patch says why.
      name = "ipu7-isys-skips-unread-log";
      patch = ./ipu7-isys-skips-unread-log.patch;
    }
    {
      # Each unbind, the recovery rebind included, left the capture side's
      # firmware boot memory allocated. From linux-next.
      name = "ipu7-isys-releases-firmware";
      patch = ./ipu7-isys-releases-firmware.patch;
    }
  ];
  jitsusama.kernel.packageOverlays = [ psys ];

  jitsusama.v4l2loopback.enable = true;
  # Which programs have the camera open, for the shell to show.
  jitsusama.v4l2loopback.holders.camera = {
    inherit device;
    unit = "camera.service";
    card = "Built-in Camera";
    ignoreUnits = [ "camera.service" ];
  };
  # The bridge carries the sensor's I2C over USB, so its drivers have to be up
  # before the IPU looks for the sensor; otherwise the bridge resets under it,
  # over and over.
  boot.extraModprobeConfig = ''
    softdep intel_ipu7 pre: usbio gpio_usbio i2c_usbio intel_cvs intel_skl_int3472_discrete
  '';

  # nixos-hardware found the bridge wedges when USB suspends it. The match is
  # on the device's own attributes, since its interfaces inherit them and have
  # no autosuspend to set. The camera service starts when the ISP appears, so
  # it never runs where there's no camera.
  services.udev.extraRules = ''
    SUBSYSTEM=="usb", ATTR{idVendor}=="06cb", ATTR{idProduct}=="0701", ATTR{power/autosuspend}="-1"
    SUBSYSTEM=="intel-ipu7-psys", TAG+="systemd", ENV{SYSTEMD_WANTS}+="camera.service"
  '';

  # The capture side gives every CSI-2 stream a raw V4L2 node no program can
  # show, and libcamera offers the bare sensor, which the HAL already holds
  # and which PipeWire would otherwise rank above the loopback as the default
  # camera. PipeWire hides both, so the loopback is the only camera listed.
  # The only libcamera camera here is that sensor; a USB camera still shows
  # through V4L2.
  services.pipewire.wireplumber.extraConfig."50-hide-raw-camera-nodes" = {
    "monitor.v4l2.rules" = [
      {
        matches = [ { "device.product.name" = "ipu7"; } ];
        actions.update-props."device.disabled" = true;
      }
    ];
    "monitor.libcamera.rules" = [
      {
        matches = [ { "device.api" = "libcamera"; } ];
        actions.update-props."device.disabled" = true;
      }
    ];
  };

  systemd.services.camera = {
    description = "The front camera, as a V4L2 camera programs can open";
    wants = [ "modprobe@v4l2loopback.service" ];
    after = [ "modprobe@v4l2loopback.service" ];
    environment = {
      GST_PLUGIN_PATH = plugins;
    }
    // lib.optionalAttrs (module != null) { CAMERA_CFG_PATH = "${halConfig}/"; };
    serviceConfig = {
      RuntimeDirectory = "camera";
      # It runs as root to make the loopback camera, as nixpkgs's relay does,
      # and like that relay it needs no network and no shared /tmp.
      PrivateNetwork = true;
      PrivateTmp = true;
      # The sensor may bind a moment after the ISP appears.
      Restart = "always";
      RestartSec = 2;
    };
    # Four buffers keep the frame rate up, and exclusive capture lets Chrome
    # list it as a camera.
    preStart = "${loopback} add -b 4 -x 1 -n 'Built-in Camera' > ${device}";
    script = ''exec ${lib.getExe pkgs.v4l2-relayd} -i "${input}" -o "${output}"'';
    postStop = ''${loopback} delete "$(cat ${device})" || true'';
  };
}
