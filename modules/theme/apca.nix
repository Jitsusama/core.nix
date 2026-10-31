# APCA's lightness contrast (Lc) of text on a background, both #RRGGBB, in
# plain Nix, so a theme's legibility is known while the machine is
# evaluated. It's the 0.0.98G-4g formula WCAG 3's drafts use:
# https://github.com/Myndex/apca-w3. Unlike WCAG 2's ratio it weighs how
# light the background is, so a dark theme's dim text measures as dim as it
# reads: Osaka Jade's parchment passes 4.5:1 and is Lc 67.6, under the 75 body
# text needs.
{ lib }:
let
  abs = x: if x < 0 then -x else x;

  # Nix has no powers of fractions, and APCA raises to 2.4, 0.56, 0.57, 0.62,
  # 0.65 and 1.414. So x^p is e^(p ln x), each from its series: ln after
  # halving or doubling x into [0.5, 2), and e^y as (e^(y/64))^64.
  ln2 = 0.6931471805599453;
  reduced =
    x: k:
    if x < 0.5 then
      reduced (x * 2) (k - 1)
    else if x >= 2.0 then
      reduced (x / 2) (k + 1)
    else
      { inherit x k; };
  ln =
    x:
    let
      r = reduced x 0;
      z = (r.x - 1) / (r.x + 1);
      series =
        lib.foldl'
          (sum: n: {
            total = sum.total + sum.power / (2 * n + 1);
            power = sum.power * z * z;
          })
          {
            total = 0.0;
            power = z;
          }
          (lib.range 0 24);
    in
    2 * series.total + r.k * ln2;
  exp =
    y:
    let
      step = y / 64.0;
      series =
        lib.foldl'
          (sum: n: rec {
            term = sum.term * step / n;
            total = sum.total + term;
          })
          {
            total = 1.0;
            term = 1.0;
          }
          (lib.range 1 20);
    in
    lib.foldl' (v: _: v * v) series.total (lib.range 1 6);
  power = x: p: if x <= 0 then 0.0 else exp (p * ln x);

  luminance =
    colour:
    let
      rgb = builtins.match "#(..)(..)(..).*" colour;
      channel = n: power (lib.fromHexString (builtins.elemAt rgb n) / 255.0) 2.4;
    in
    if rgb == null then
      throw "${colour} isn't a #RRGGBB colour"
    else
      0.2126729 * channel 0 + 0.7151522 * channel 1 + 7.2175e-2 * channel 2;

  # Near black, the screen's own flare lifts what the eye sees, so APCA
  # softens the darkest luminances rather than letting them reach zero.
  softened = y: if y >= 2.2e-2 then y else y + power (2.2e-2 - y) 1.414;

  # Signed as APCA gives it: positive for dark text on light, negative for
  # light on dark.
  lc =
    text: background:
    let
      t = softened (luminance text);
      b = softened (luminance background);
      dark = (power b 0.56 - power t 0.57) * 1.14;
      light = (power b 0.65 - power t 0.62) * 1.14;
    in
    if abs (b - t) < 5.0e-4 then
      0.0
    else if b > t then
      (if dark < 0.1 then 0.0 else (dark - 2.7e-2) * 100)
    else if light > -0.1 then
      0.0
    else
      (light + 2.7e-2) * 100;
in
{
  inherit lc;

  # How far apart text and background are, either way round.
  lightness = text: background: abs (lc text background);

  # An Lc as people write it, to the nearest tenth: 67.6.
  show =
    lc:
    let
      tenths = builtins.floor (abs lc * 10 + 0.5);
    in
    "${toString (tenths / 10)}.${toString (lib.mod tenths 10)}";
}
