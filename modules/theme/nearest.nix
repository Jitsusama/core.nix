# Which of a terminal's colours stands for one of the theme's, for a
# surface that can draw only those sixteen: the console, and the guide on
# it. The one holding it, or else one within 48 of it across red, green
# and blue, as a colour the theme nudged from one of them is, so it keeps
# its hue there: Osaka Jade's alert, ember, is vermilion lightened to read
# on a selected row (VKCF3Z, 2026-11-07), and is drawn in the console's
# red. A colour further from every one, as Flexoki's graphite text is,
# has none, and the surface draws it in its own foreground. Both are
# #RRGGBB, and the index is into the colours given, or null.
#
#   nearest "#FF6F5F" [ "#111C18" "#FF5345" "#db9f9c" ]  →  1
{ lib }:
colour: colours:
let
  # Far enough to tell two hues apart, near enough to hold one nudged.
  near = 48;
  channels =
    hex:
    map (index: lib.fromHexString (builtins.substring (index * 2) 2 (lib.removePrefix "#" hex))) [
      0
      1
      2
    ];
  apart =
    one: other:
    lib.foldl' (sum: pair: sum + (pair.fst - pair.snd) * (pair.fst - pair.snd)) 0 (
      lib.zipLists (channels one) (channels other)
    );
  distances = lib.imap0 (index: candidate: {
    inherit index;
    distance = apart colour candidate;
  }) colours;
  closest = lib.foldl' (
    best: next: if next.distance < best.distance then next else best
  ) (builtins.head distances) distances;
in
if colour == null || colours == [ ] || closest.distance > near * near then null else closest.index
