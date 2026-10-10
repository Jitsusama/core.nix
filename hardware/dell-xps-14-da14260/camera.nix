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
  hardware.ipu7 = {
    enable = true;
    platform = "ipu75xa";
  };
  # Its relay makes the loopback with the default buffers, which nixos-hardware
  # measured at a few frames a second, so the camera service below runs the
  # relay instead.
  services.v4l2-relayd.instances.ipu7.enable = false;

  boot.extraModulePackages = [ config.boot.kernelPackages.v4l2loopback ];
  # The bridge carries the sensor's I2C over USB, so its drivers have to be up
  # before the IPU looks for the sensor; otherwise the bridge resets under it,
  # over and over. The relay makes its own loopback camera, so the module makes
  # none.
  boot.extraModprobeConfig = ''
    softdep intel_ipu7 pre: usbio gpio_usbio i2c_usbio intel_cvs intel_skl_int3472_discrete
    options v4l2loopback devices=0
  '';

  # nixos-hardware found the bridge wedges when USB suspends it. The camera
  # service starts when the ISP appears, so it never runs where there's no
  # camera.
  services.udev.extraRules = ''
    SUBSYSTEM=="usb", ATTRS{idVendor}=="06cb", ATTRS{idProduct}=="0701", ATTR{power/autosuspend}="-1"
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
