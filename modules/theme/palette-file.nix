# The theme's pigments as a GIMP palette, the .gpl file GIMP and Inkscape both
# read, so a drawing can take its colours from the theme the desktop is drawn
# in. The palette is named after the theme and each colour after its pigment,
# the shades first, from the background to the brightest text, then the rest
# by name: https://developer.gimp.org/core/standards/gpl/
{ lib }:
theme:
let
  channels =
    colour:
    let
      rgb = builtins.match "#(..)(..)(..).*" colour;
    in
    map lib.fromHexString rgb;

  # The pigment's own name, as the theme file spells it, in words.
  named = name: lib.replaceStrings [ "_" ] [ " " ] name;

  nameOf =
    colour: lib.findFirst (name: theme.palette.${name} == colour) null (lib.attrNames theme.palette);
  shaded = lib.filter (name: name != null) (map nameOf theme.shades);
  rest = lib.filter (name: !(lib.elem name shaded)) (lib.attrNames theme.palette);

  row =
    name:
    let
      padded = map (value: lib.fixedWidthString 3 " " (toString value)) (channels theme.palette.${name});
    in
    "${lib.concatStringsSep " " padded}\t${named name}";
in
''
  GIMP Palette
  Name: ${theme.title}
  Columns: 8
  #
  ${lib.concatMapStringsSep "\n" row (lib.unique (shaded ++ rest))}
''
