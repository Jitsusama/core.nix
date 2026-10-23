# Which programs are streaming from a USB camera, in the same folder the
# loopback cameras' holders are written to (../v4l2loopback/nixos.nix), for
# the shell's camera piece and the desk's light to read. A USB camera comes
# and goes with its cable and takes whatever node number is free, so udev
# starts a watch for each one as it comes, bound to its node, and the watch
# writes /run/camera-holders/<the camera's name> while the camera is there
# and removes it when it goes.
#
# A program is named only while the camera streams, which uvcvideo starts by
# picking a streaming interface's alternate setting other than zero: the
# programs that only look at a camera, PipeWire's among them, open it for a
# moment as it comes, and naming them would flash the light and the piece.
# The camera's capture node alone is watched, the first a UVC camera makes,
# not its metadata node.
{ lib, pkgs, ... }:
let
  holders = pkgs.callPackage ../v4l2loopback/holders/package.nix { };
  holdersDir = "/run/camera-holders";
in
{
  # A watch on the folder lasts only while the folder does, so it's made at
  # boot and kept, whether or not a camera is ever plugged in. It's the
  # loopbacks' folder too, so it's set by name, which the two modules'
  # settings merge on.
  systemd.tmpfiles.settings.camera-holders.${holdersDir}.d = {
    mode = "0755";
    user = "root";
    group = "root";
  };

  services.udev.extraRules = ''
    SUBSYSTEM=="video4linux", ENV{ID_USB_DRIVER}=="uvcvideo", ENV{ID_V4L_CAPABILITIES}==":capture:", ATTR{index}=="0", TAG+="systemd", ENV{SYSTEMD_WANTS}+="usb-camera-holders@%k.service"
  '';

  systemd.services."usb-camera-holders@" = {
    description = "Says which programs stream from the USB camera at /dev/%I";
    # It goes when the camera does, and a camera plugged in again starts it
    # afresh, whatever node it's given.
    bindsTo = [ "dev-%i.device" ];
    after = [
      "dev-%i.device"
      "systemd-tmpfiles-setup.service"
    ];
    serviceConfig = {
      ExecStart = "${lib.getExe holders} --usb /dev/%I ${holdersDir}";
      # The same reach as a loopback's watch: every process's open files,
      # and nothing else.
      CapabilityBoundingSet = [
        "CAP_DAC_READ_SEARCH"
        "CAP_SYS_PTRACE"
      ];
      NoNewPrivileges = true;
      PrivateNetwork = true;
      PrivateTmp = true;
      ProtectSystem = "strict";
      ProtectHome = "read-only";
      ProtectKernelTunables = true;
      ProtectKernelModules = true;
      ProtectKernelLogs = true;
      ProtectControlGroups = true;
      ProtectClock = true;
      ProtectHostname = true;
      RestrictAddressFamilies = "none";
      RestrictNamespaces = true;
      RestrictRealtime = true;
      RestrictSUIDSGID = true;
      LockPersonality = true;
      MemoryDenyWriteExecute = true;
      SystemCallArchitectures = "native";
      SystemCallFilter = [
        "@system-service"
        "~@privileged @resources"
      ];
      ReadWritePaths = [ holdersDir ];
    };
  };
}
