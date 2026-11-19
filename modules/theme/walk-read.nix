# The walk the margin cat's code takes, read from the JavaScript that walks
# it, for modules/theme/checks.nix to hold its strides to; its own file, as
# it changes only with the cat's code or how that code is read. The check
# proves the walk that's drawn rather than a copy: the words WALKS lists,
# when it's every list STRIDING strides by. Both are frozen where they're
# declared, so no code after can change the walk, and only the two
# declarations are read, comments aside, each whole, to its semicolon. Each
# must be only what's meant, spaces allowed between its parts: WALKS frozen
# quoted words, STRIDING frozen strides by WALKS, each declared once, and
# WALKS named nowhere else, though a word of its own only, not another's
# property. So anything else, another list, a stride written inline, an
# entry not a quoted word, a walk left unfrozen, declared twice or not
# found, reads as no walk, which fails saying what it wanted, strictly: a
# string naming WALKS counts, and a key quoted or a pose with a digit isn't
# read. It guards the walk from an honest change, not a hostile one: a
# comment's marks are found wherever they stand, so `//` or `/*` inside a
# string opens a comment there; nor is a name spelt with a Unicode escape
# seen, one declared again after a comma, as a parameter or destructured,
# which kinds STRIDING names, or code that strides by a list of its own or
# reorders STRIDING's as it reads it, which no check here sees. None of the
# cat's files has any.
{ lib }:
file:
let
  # The code with each comment, a line's or a block's, made a space, read
  # mark by mark, so a long comment costs no deeper a regex than a short
  # one: a pattern repeated per character overflows Nix's stack on one of
  # some thousands. Each step's code is made at once, so a long file
  # leaves no chain of joins as deep as its marks are many.
  ways =
    let
      step =
        was: piece:
        let
          next = stepped was piece;
        in
        builtins.seq next.code next;
      stepped =
        was: piece:
        let
          mark = lib.head piece;
        in
        if lib.isString piece then
          was // lib.optionalAttrs (was.inside == "code") { code = was.code ++ [ piece ]; }
        else if was.inside == "code" && (mark == "//" || mark == "/*") then
          {
            inside = if mark == "//" then "line" else "block";
            code = was.code ++ [ " " ];
          }
        else if was.inside == "code" then
          was // { code = was.code ++ [ mark ]; }
        else if was.inside == "line" && mark == "\n" || was.inside == "block" && mark == "*/" then
          {
            inside = "code";
            code = was.code ++ lib.optional (mark == "\n") mark;
          }
        else
          was;
    in
    lib.concatStrings
      (lib.foldl' step {
        inside = "code";
        code = [ ];
      } (builtins.split "(//|/\\*|\\*/|\n)" file)).code;
  # The constant's declaration, up to the semicolon that ends it.
  declared =
    name:
    let
      after = builtins.split "const[[:space:]]+${name}[[:space:]]*=[[:space:]]*" ways ++ [
        [ ]
        ""
      ];
    in
    lib.head (lib.splitString ";" (lib.elemAt after 2));
  walks = declared "WALKS";
  striding = declared "STRIDING";
  # How often `name` stands in `text` as a word of its own, not a part of
  # a longer one or another's property, by the characters either side.
  wordy = c: builtins.match "[A-Za-z0-9_$]" c != null;
  named =
    name: text:
    let
      pieces = lib.splitString name text;
      piece = lib.elemAt pieces;
      ending = s: builtins.substring (lib.stringLength s - 1) 1 s;
      # Right after the name before it, the character is that name's last.
      last =
        i:
        if piece i != "" then
          ending (piece i)
        else if i > 0 then
          ending name
        else
          "";
      # A spread's dots open a value, not a property.
      property = i: last i == "." && !lib.hasSuffix "..." (piece i);
      alone = i: !wordy (last i) && !property i && !wordy (builtins.substring 0 1 (piece (i + 1)));
    in
    lib.count alone (lib.range 0 (lib.length pieces - 2));
  # How often `name` is declared, as a constant, a variable, a function or
  # a class, so one declared again in a block is seen.
  declaring = "(^|[^A-Za-z0-9_$.])(const|let|var|function|class)[[:space:]]+";
  declarations =
    name: lib.count lib.isList (builtins.split "${declaring}${name}([^A-Za-z0-9_$]|$)" ways);
  s = "[[:space:]]*";
  word = "(\"[A-Za-z]+\"|'[A-Za-z]+')";
  stride = "[A-Za-z_]+${s}:${s}WALKS";
  listed = one: "${s}(${one}${s},${s})*${one}${s},?${s}";
  frozen = value: "Object${s}\\.${s}freeze${s}\\(${s}${value}${s}\\)${s}";
  words = map lib.head (lib.filter lib.isList (builtins.split "[\"']([A-Za-z]+)[\"']" walks));
  # What the code isn't, of what it must be, so a walk read as none
  # says why.
  faults = map (rule: rule.wanted) (
    lib.filter (rule: !rule.met) [
      {
        wanted = "WALKS frozen as quoted words";
        met = builtins.match (frozen "\\[${listed word}]") walks != null;
      }
      {
        wanted = "STRIDING frozen as strides by WALKS";
        met = builtins.match (frozen "\\{${listed stride}}") striding != null;
      }
      {
        wanted = "WALKS named nowhere else";
        met = named "WALKS" ways == 1 + named "WALKS" striding;
      }
      {
        wanted = "WALKS and STRIDING each declared once";
        met = declarations "WALKS" == 1 && declarations "STRIDING" == 1;
      }
    ]
  );
in
{
  inherit faults;
  walk = lib.optionals (faults == [ ]) words;
}
