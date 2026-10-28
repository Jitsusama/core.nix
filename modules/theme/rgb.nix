# One of the theme's colours as its three channels, for a program that reads
# rgb() and not #RRGGBB: broot's skin.
#
#   rgb colours.selection  →  rgb(50, 71, 59)
{ lib }:
colour:
let
  hex = lib.removePrefix "#" colour;
  channel = index: toString (lib.fromHexString (builtins.substring (index * 2) 2 hex));
in
"rgb(${channel 0}, ${channel 1}, ${channel 2})"
