# The front camera: an OV08X40 behind Intel's CVS bridge, imaged by the IPU7's
# hardware ISP. The kernel has the IPU's core and capture side and the bridge,
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
  loopback = lib.getExe' config.boot.kernelPackages.v4l2loopback.bin "v4l2loopback-ctl";
  device = "/run/camera/device";

  # The sensor is mounted upside down. Turning the picture around, rather than
  # mirroring it, keeps it the way others should see it; calls mirror Joel's
  # own preview themselves.
  input = "icamerasrc ! videoconvert ! videoscale ! videoflip method=rotate-180";
  # The ISP scales for free, and 1080p moves a quarter of 4K's bytes through
  # the loopback. A leaky queue drops a stale frame rather than delaying the
  # next one.
  output = lib.concatStringsSep " ! " [
    "appsrc name=appsrc caps=video/x-raw,format=NV12,width=1920,height=1080,framerate=30/1"
    "queue leaky=downstream max-size-buffers=3"
    "videoconvert"
    "v4l2sink name=v4l2sink device=$(cat ${device}) sync=false"
  ];
in
{
  imports = [
    ../../modules/camera/nixos.nix
    ../../modules/kernel/nixos.nix
  ];

  # Pictures and videos are taken from the loopback, which the file names.
  jitsusama.camera.device = device;

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
  ];
  jitsusama.kernel.packageOverlays = [ psys ];

  boot.extraModulePackages = [ config.boot.kernelPackages.v4l2loopback ];
  # The bridge carries the sensor's I2C over USB, so its drivers have to be up
  # before the IPU looks for the sensor; otherwise the bridge resets under it,
  # over and over. The relay makes its own loopback camera, so the module makes
  # none.
  boot.extraModprobeConfig = ''
    softdep intel_ipu7 pre: usbio gpio_usbio i2c_usbio intel_cvs intel_skl_int3472_discrete
    options v4l2loopback devices=0
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
    environment.GST_PLUGIN_PATH = plugins;
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
