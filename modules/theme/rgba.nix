# One of the theme's colours seen through, as CSS and GTK 3 write it, for a
# program that reads rgba() and not #RRGGBBAA: zathura's search highlight,
# which has to let the words under it show.
#
#   rgba colours.selection "0.5"  →  rgba(50, 71, 59, 0.5)
{ lib }:
colour: alpha:
let
  hex = lib.removePrefix "#" colour;
  channel = index: toString (lib.fromHexString (builtins.substring (index * 2) 2 hex));
in
"rgba(${channel 0}, ${channel 1}, ${channel 2}, ${alpha})"
