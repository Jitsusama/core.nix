# Two of the theme's colours mixed, for a program that can't see through one
# to the other: Claude Code's diffs, whose backgrounds are a tint of the
# success or alert colour over the background.
#
#   mix 0.25 roles.success roles.background  →  25% success, 75% background
{ lib }:
weight: over: under:
let
  channel =
    colour: index:
    let
      hex = lib.removePrefix "#" colour;
    in
    lib.fromHexString (builtins.substring (index * 2) 2 hex);
  blend =
    index:
    let
      value = weight * channel over index + (1 - weight) * channel under index;
      rounded = builtins.floor (value + 0.5);
    in
    lib.toLower (lib.fixedWidthString 2 "0" (lib.toHexString rounded));
in
"#${blend 0}${blend 1}${blend 2}"
