# Which colour camera module the laptop has. Dell fits the XPS 14 with one of
# several OV08X40 modules, and its Windows driver tunes the picture for the
# one fitted, its lens shading and colour response; Linux has only Intel's
# tuning for its own reference module. The firmware names the module in the
# sensor's ACPI _DDN, which Linux doesn't show; docs/hardware.md says how to
# read it.
{ lib, ... }:
{
  options.jitsusama.dellXps14.cameraModule = lib.mkOption {
    type = lib.types.nullOr (lib.types.strMatching "[A-Z0-9_]+");
    default = null;
    example = "CJFOE90";
    description = ''
      The colour camera's module, as the firmware names it. Set, the picture
      is tuned with Dell's tuning for that module, taken from Dell's Windows
      driver pack as the system builds. Null keeps Intel's tuning for its
      reference module.
    '';
  };
}
