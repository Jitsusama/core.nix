# One of the theme's colours as a terminal escape's own numbers, for a
# program that takes those rather than #RRGGBB: 38 for the text's colour, 48
# for what's behind it.
#
#   sgr "48" roles.selection  →  48;2;50;71;59
{ lib }:
layer: colour:
let
  hex = lib.removePrefix "#" colour;
  channel = index: toString (lib.fromHexString (builtins.substring (index * 2) 2 hex));
in
"${layer};2;${channel 0};${channel 1};${channel 2}"
