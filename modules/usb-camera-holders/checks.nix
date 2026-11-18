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
  # which the unit's sandbox allows, and only to read: ProtectClock alone
  # allows the clock's device and no other, and PrivateDevices or a closed
  # policy would hide the camera, every line then saying it can't ask it.
  usb-camera-holders-may-open-the-camera =
    let
      unit = graphical.config.systemd.services."usb-camera-holders@".serviceConfig;
      allowed = lib.toList (unit.DeviceAllow or [ ]);
    in
    holds "usb-camera-holders-may-open-the-camera"
      "The USB camera watch's sandbox keeps it from opening the camera to read, or lets it write."
      (
        lib.elem "char-video4linux r" allowed
        && !lib.any (lib.hasInfix "video4linux rw") allowed
        && !(unit.PrivateDevices or false)
        && (unit.DevicePolicy or "auto") == "auto"
      );
}
