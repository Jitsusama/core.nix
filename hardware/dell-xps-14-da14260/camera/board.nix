# Which colour camera board the laptop has. Dell fits the XPS 14 with one of
# several OV08X40 boards, and its own driver tunes the picture for the one
# fitted, its lens shading and colour response; Linux has only Intel's
# tuning for its own reference board. The firmware names the board in the
# sensor's ACPI _DDN, which Linux doesn't show; docs/hardware.md says how to
# read it.
{ lib, ... }:
{
  options.jitsusama.dellXps14.cameraBoard = lib.mkOption {
    type = lib.types.nullOr (lib.types.strMatching "[A-Z0-9_]+");
    default = null;
    example = "CJFOE90";
    description = ''
      The colour camera's board, as the firmware names it. Set, the picture
      is tuned with Dell's tuning for that board, taken from Dell's driver
      pack as the system builds. Null keeps Intel's tuning for its
      reference board.
    '';
  };
}
