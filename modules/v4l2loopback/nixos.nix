# The loopback camera module, for a relay that reads a camera and hands its
# frames on through a camera other programs can open. Whichever module makes
# a loopback turns it on, and takes v4l2loopback-ctl from its package, so the
# module loaded and the program making its cameras are the same build.
{ config, lib, ... }:
let
  cfg = config.jitsusama.v4l2loopback;
in
{
  options.jitsusama.v4l2loopback = {
    enable = lib.mkEnableOption "the loopback camera module";
    package = lib.mkOption {
      type = lib.types.package;
      readOnly = true;
      # A relay stops writing while it waits for its next reader, and the
      # module would hand that reader the last frame written, the end of the
      # session before. Written here; the patch says why.
      default = config.boot.kernelPackages.v4l2loopback.overrideAttrs (old: {
        patches = old.patches or [ ] ++ [ ./reader-starts-after-newest.patch ];
      });
      defaultText = lib.literalMD "the kernel's `v4l2loopback`, patched";
      description = "The module, built for the kernel, with `v4l2loopback-ctl` in its `bin`.";
    };
  };

  config = lib.mkIf cfg.enable {
    boot.extraModulePackages = [ cfg.package ];
    # Each relay makes the loopback it needs, so the module makes none.
    boot.extraModprobeConfig = ''
      options v4l2loopback devices=0
    '';
  };
}
