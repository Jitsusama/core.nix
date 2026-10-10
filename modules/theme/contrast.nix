# WCAG 2's contrast ratio between two #RRGGBB colours, in plain Nix, so a
# theme's legibility is known while the machine is evaluated:
# https://www.w3.org/TR/WCAG22/#dfn-contrast-ratio
{ lib }:
let
  # Nix has no powers of fractions, and sRGB's curve needs x^2.4. That's
  # x^2 times x^0.4, and x^0.4 is the fifth root of x^2, which Newton's
  # method finds in a few steps for anything between 0 and 1.
  fifthRoot = a: lib.foldl' (y: _: (4 * y + a / (y * y * y * y)) / 5) 1.0 (lib.range 1 24);
  power2_4 = x: x * x * fifthRoot (x * x);

  channel =
    hex:
    let
      s = lib.fromHexString hex / 255.0;
    in
    if s <= 4.045e-2 then s / 12.92 else power2_4 ((s + 5.5e-2) / 1.055);

  luminance =
    color:
    let
      rgb = builtins.match "#(..)(..)(..).*" color;
      at = builtins.elemAt rgb;
    in
    if rgb == null then
      throw "${color} isn't a #RRGGBB colour"
    else
      0.2126 * channel (at 0) + 0.7152 * channel (at 1) + 7.22e-2 * channel (at 2);
in
{
  inherit luminance;

  ratio =
    one: other:
    let
      a = luminance one;
      b = luminance other;
    in
    (lib.max a b + 5.0e-2) / (lib.min a b + 5.0e-2);

  # A ratio as people write it: 4.52, not 4.523810.
  show =
    ratio:
    let
      hundredths = builtins.floor (ratio * 100);
      cents = lib.mod hundredths 100;
    in
    "${toString (hundredths / 100)}.${lib.fixedWidthNumber 2 cents}";
}
