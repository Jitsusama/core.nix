# What systemd-backlight's units save with at shutdown (saves-while-open.sh):
# the level as it reads, except the screen's while a lid is shut. Given
# `save`, the systemd-backlight it saves with, so a check can give a
# stand-in.
{
  gnugrep,
  lib,
  save,
  writeShellApplication,
}:
writeShellApplication {
  name = "saves-while-open";
  runtimeInputs = [ gnugrep ];
  text = lib.concatLines [
    "save=${lib.escapeShellArg save}"
    (builtins.readFile ./saves-while-open.sh)
  ];
}
