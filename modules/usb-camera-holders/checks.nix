# The checks of usb-camera-holders, which checks.nix gathers with every other
# folder's; its opening comment says what each kind of check proves. Its watch's
# own tests run outside systemd, so what's checked here is the sandbox the unit
# runs it in.
{
  graphical,
  holds,
  lib,
  onLinux,
  ...
}:
onLinux {
  # The watch opens the camera it records, to read its format and controls,
  # which the unit's sandbox allows, opened read-only: ProtectClock alone
  # allows the clock's device and no other, and PrivateDevices would hide
  # the camera, every line then saying it can't ask it. Every policy lets
  # through what DeviceAllow names, so none is refused. An entry's access
  # is its last word, spaces around it aside, and one without it grants all
  # three of r, w and m.
  usb-camera-holders-may-open-the-camera =
    let
      unit = graphical.config.systemd.services."usb-camera-holders@".serviceConfig;
      allowed = lib.toList (unit.DeviceAllow or [ ]);
      words = entry: lib.filter (word: word != "") (lib.splitString " " entry);
      access = entry: lib.last ([ "rwm" ] ++ lib.tail (words entry));
      writes = entry: lib.hasInfix "w" (access entry) || lib.hasInfix "m" (access entry);
      hidden = unit.PrivateDevices or false;
    in
    holds "usb-camera-holders-may-open-the-camera"
      "The USB camera watch's sandbox keeps it from opening the camera to read, or lets it write."
      (lib.elem "char-video4linux r" allowed && !lib.any writes allowed && !hidden);

  # A run killed before it removes its files leaves its camera said there;
  # each watch forgets its node as its unit stops, in the folder it writes,
  # so the relayed camera's watch forgets in its own and not the shared one,
  # and one that crashed starts again.
  usb-camera-holders-forget-a-killed-run =
    let
      services = graphical.config.systemd.services;
      forgets =
        unit: folder:
        let
          config = services.${unit}.serviceConfig;
        in
        lib.hasSuffix " --usb /dev/%I ${folder}" config.ExecStart
        && (config.Restart or null) == "on-failure"
        && (services.${unit}.unitConfig.StartLimitIntervalSec or null) == "0"
        && (config.RestartMaxDelaySec or null) == "1min"
        && lib.any (line: lib.hasSuffix " --usb --forget /dev/%I ${folder}" line) (
          lib.toList (config.ExecStopPost or [ ])
        );
    in
    holds "usb-camera-holders-forget-a-killed-run"
      "A USB camera's watch doesn't forget its node in its own folder as it stops, or start again."
      (
        forgets "usb-camera-holders@" "/run/camera-holders"
        && forgets "usb-camera-holders-relayed@" "/run/camera-holders/.relayed"
      );
}
