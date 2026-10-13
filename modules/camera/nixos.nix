# Which camera the machine has, for the account's camera program
# (home.nix): its hardware's module says, since only that knows where its
# camera appears. With none, nothing offers to take a picture with it.
{ lib, ... }:
{
  options.jitsusama.camera.device = lib.mkOption {
    type = lib.types.nullOr lib.types.str;
    default = null;
    example = "/dev/video0";
    description = ''
      The camera pictures and videos are taken with, as a V4L2 device, or a
      file naming one, for a camera made at boot under a number it's given
      then. Null for none.
    '';
  };
}
