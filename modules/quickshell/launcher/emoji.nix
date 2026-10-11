# Every emoji, for the launcher, from Unicode's list and CLDR's English
# keywords: a line of JSON each, [emoji, name, keywords, group], in
# Unicode's order, with skin tones folded into the emoji they tone, which
# takes `tone` when it has one: none, light, medium-light, medium,
# medium-dark or dark. emoji.py says how.
{ lib, pkgs }:
tone:
let
  cldr = "${pkgs.cldr-annotations}/share/unicode/cldr/common";
in
pkgs.runCommandLocal "emoji" { } ''
  ${lib.getExe pkgs.python3} ${./emoji.py} \
    ${pkgs.unicode-emoji}/share/unicode/emoji/emoji-test.txt \
    ${cldr}/annotations/en.xml ${cldr}/annotationsDerived/en.xml ${tone} > $out
''
