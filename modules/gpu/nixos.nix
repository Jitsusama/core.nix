# Names the GPU the desktop draws with, so what needs it can wait for its
# driver. The hardware module sets the path; this gives that card a systemd
# device unit, which systemd doesn't make for graphics cards by itself.
{
  config,
  lib,
  utils,
  ...
}:
let
  inherit (config.jitsusama.gpu) path;
in
{
  options.jitsusama.gpu = {
    path = lib.mkOption {
      type = lib.types.nullOr lib.types.str;
      default = null;
      example = "pci-0000:00:02.0";
      description = ''
        The GPU the desktop draws with, as udev's ID_PATH names it. Set by the
        hardware module, so a firmware framebuffer, which has a path of its
        own, can't stand in for it.
      '';
    };

    unit = lib.mkOption {
      type = lib.types.nullOr lib.types.str;
      readOnly = true;
      default =
        if path == null then null else "${utils.escapeSystemdPath "/dev/dri/by-path/${path}-card"}.device";
      defaultText = lib.literalMD "the device unit of `/dev/dri/by-path/<path>-card`, or null";
      description = "The device unit that's active once the GPU's driver has its card ready.";
    };
  };

  config = lib.mkIf (path != null) {
    services.udev.extraRules = ''
      SUBSYSTEM=="drm", KERNEL=="card[0-9]*", ENV{ID_PATH}=="${path}", TAG+="systemd"
    '';

    # A card that never comes up, because its driver failed or the system
    # booted elsewhere, gives up on the unit after 15 s rather than systemd's
    # 90, so what waits for it starts on whatever else draws. A working card
    # was ready 4 to 6 s after the system started where this was measured.
    systemd.units.${config.jitsusama.gpu.unit} = {
      overrideStrategy = "asDropin";
      text = ''
        [Unit]
        JobRunningTimeoutSec=15s
      '';
    };
  };
}
