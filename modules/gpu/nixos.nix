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
  };
}
