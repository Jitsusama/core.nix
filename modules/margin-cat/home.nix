# The margin's cat: the empty places' cat, sitting in the gap above the
# column Joel is in (MarginCat.qml), a surface of the shell's added from here
# so a machine has it by importing this module, and leaves it out by not.
{ niri, quickshell }:
{ lib, ... }:
{
  imports = [
    niri
    quickshell
  ];

  jitsusama.quickshell.surfaces.MarginCat = ./MarginCat.qml;
  xdg.configFile."quickshell/surfaces/MarginCat.js".source = ./MarginCat.js;

  # A shared screen shows what Joel shares, not the company he keeps. The
  # cat leaves while a share is on (MarginCat.qml); this keeps it from the
  # frames before the shell hears the share began.
  xdg.configFile."niri/config.kdl".text = lib.mkAfter ''
    layer-rule {
        match namespace="^cat$"
        block-out-from "screencast"
    }
  '';
}
