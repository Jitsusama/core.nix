# A program's desktop entry, masked: an entry of the same name that says
# Hidden, which the desktop entry specification reads as deleted, so the
# launcher and every other list of programs leave it out while the program
# still runs from a terminal. Put among a profile's packages, it takes the
# place of the program's own entry there.
{ lib, writeTextDir }:
name:
lib.hiPrio (
  writeTextDir "share/applications/${name}.desktop" ''
    [Desktop Entry]
    Hidden=true
  ''
)
