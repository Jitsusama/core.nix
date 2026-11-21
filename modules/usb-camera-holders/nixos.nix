# Which programs are streaming from a USB camera, in the same folder the
# loopback cameras' holders are written to (../v4l2loopback/nixos.nix), for
# the shell's camera piece and the desk's light to read. A USB camera comes
# and goes with its cable and takes whatever node number is free, so udev
# starts a watch for each one as it comes, bound to its node, and the watch
# writes /run/camera-holders/<the camera's name> while the camera is there
# and removes it when it goes.
#
# A camera a relay shares through a loopback, as ../mx-brio does, is marked
# CAMERA_RELAYED=1 by its own udev rule, and its loopback's holders take its
# name in the folder, since the loopback is what programs open. Its own
# watch then writes in the hidden folder /run/camera-holders/.relayed
# instead, which the shell and the light pass over as they do every dotted
# name, so the two never write one file and the relay is never shown as
# the program on camera.
#
# A program is named only while the camera streams, which uvcvideo starts by
# picking a streaming interface's alternate setting other than zero: the
# programs that only look at a camera, PipeWire's among them, open it for a
# moment as it comes, and naming them would flash the light and the piece.
# The camera's capture node alone is watched, the first a UVC camera makes,
# not its metadata node.
#
# While it streams, the watch also says in its journal every ten seconds
# how: the format and frame rate asked for, the frames the camera gave by
# uvcvideo's count in debugfs and the packets it sent in error, and every
# control's value, exposure among them. A call that looked bad or choppy
# is read back afterwards with `journalctl -u 'usb-camera-holders*'`,
# rather than pieced together from what else was running. It reads no
# frames.
{ lib, pkgs, ... }:
let
  holders = pkgs.callPackage ../v4l2loopback/holders/package.nix { };
  holdersDir = "/run/camera-holders";
  relayedDir = "${holdersDir}/.relayed";

  # The watch of the camera at /dev/%I, writing its file in folder.
  watch = description: folder: {
    inherit description;
    # It goes when the camera does, and a camera plugged in again starts it
    # afresh, whatever node it's given.
    bindsTo = [ "dev-%i.device" ];
    after = [
      "dev-%i.device"
      "systemd-tmpfiles-setup.service"
    ];
    serviceConfig = {
      ExecStart = "${lib.getExe holders} --usb /dev/%I ${folder}";
      # A run killed before it could remove its files would leave the
      # camera said there, and its readers said, until another camera came
      # at this node; as the unit stops, what says the node goes too,
      # unless a loopback has the node by then, whose files say it rightly.
      ExecStopPost = "${lib.getExe holders} --usb --forget /dev/%I ${folder}";
      # A run that crashes with its camera still plugged in starts again
      # once its files are forgotten, rather than leaving the camera
      # unwatched until it's plugged in again; the start limit ends a
      # run that can never start.
      Restart = "on-failure";
      RestartSec = "1s";
      # The same reach as a loopback's watch, every process's open files,
      # and the camera's own node, opened to read its format and controls.
      # ProtectClock allows only the clock's device unless others are named.
      DeviceAllow = [ "char-video4linux r" ];
      # Running as root, it reads uvcvideo's counts in
      # debugfs too, which ProtectKernelTunables leaves readable: root owns
      # debugfs and uvcvideo makes its stats readable by all, so no
      # capability is needed, and the line says why if they can't be read.
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
      ReadWritePaths = [ folder ];
    };
  };
in
{
  # A watch on the folder lasts only while the folder does, so it's made at
  # boot and kept, whether or not a camera is ever plugged in. It's the
  # loopbacks' folder too, so it's set by name, which the two modules'
  # settings merge on.
  systemd.tmpfiles.settings.camera-holders = {
    ${holdersDir}.d = {
      mode = "0755";
      user = "root";
      group = "root";
    };
    ${relayedDir}.d = {
      mode = "0755";
      user = "root";
      group = "root";
    };
  };

  # Read after the rules that mark a relayed camera, which come before
  # systemd's 73-seat-late.rules, as this file comes at 99.
  services.udev.extraRules = ''
    SUBSYSTEM=="video4linux", ENV{ID_USB_DRIVER}=="uvcvideo", ENV{ID_V4L_CAPABILITIES}==":capture:", ATTR{index}=="0", ENV{CAMERA_RELAYED}!="1", TAG+="systemd", ENV{SYSTEMD_WANTS}+="usb-camera-holders@%k.service"
    SUBSYSTEM=="video4linux", ENV{ID_USB_DRIVER}=="uvcvideo", ENV{ID_V4L_CAPABILITIES}==":capture:", ATTR{index}=="0", ENV{CAMERA_RELAYED}=="1", TAG+="systemd", ENV{SYSTEMD_WANTS}+="usb-camera-holders-relayed@%k.service"
  '';

  systemd.services."usb-camera-holders@" =
    watch "Says which programs stream from the USB camera at /dev/%I" holdersDir;
  systemd.services."usb-camera-holders-relayed@" =
    watch "Records how the USB camera at /dev/%I streams to the relay sharing it" relayedDir;
}
