# The walk the margin cat's code takes, read from the JavaScript that walks
# it, for modules/theme/checks.nix to hold its strides to; its own file, as
# it changes only with the cat's code or how that code is read. The check
# proves the walk that's drawn rather than a copy: the words WALKS lists,
# when it's every list STRIDING strides by. Only the code is read, comments
# aside, and each name only as a word of its own, not another's property.
# Each is read whole, to its semicolon, and must be only what's meant,
# spaces allowed only between its parts; elsewhere, WALKS is named only in
# STRIDING, and STRIDING is declared once and otherwise only read by a key,
# then perhaps by a method that changes nothing, with nothing assigned
# there. So anything else, another list, a stride written inline, an entry
# not a quoted word, something after its bracket, a key nested past three
# deep, an assignment to either or either not found, reads as no walk,
# which fails saying where it looked and what it wanted. It guards the walk
# from an honest change, not a hostile one: a comment's marks are found
# wherever they stand, so `//` or `/*` inside a string opens a comment
# there; nor is a delete seen, a write through what a read hands back or a
# destructuring pattern, or one from another file. None of the cat's files
# has any, and honest reads written otherwise, `kind in STRIDING` say, are
# refused, failing loudly rather than reading a walk it doesn't take.
{ lib }:
file:
let
  # The code with each comment, a line's or a block's, made a space, read
  # mark by mark, so a long comment costs no deeper a regex than a short
  # one: a pattern repeated per character overflows Nix's stack on one of
  # some thousands.
  ways =
    let
      step =
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
  # Where `name` stands in `text` as a word of its own, not a part of a
  # longer one or another's property, by the character before it, the
  # text since the name before, and the statement after it, to its
  # semicolon, as one line.
  wordy = c: builtins.match "[A-Za-z0-9_$]" c != null;
  uses =
    name: text:
    let
      pieces = lib.splitString name text;
      ending = s: builtins.substring (lib.stringLength s - 1) 1 s;
      use = i: {
        before = lib.elemAt pieces i;
        # Right after the name before it, the character is that name's last.
        last =
          if lib.elemAt pieces i != "" then
            ending (lib.elemAt pieces i)
          else if i > 0 then
            ending name
          else
            "";
        after = builtins.replaceStrings [ "\n" ] [ " " ] (
          lib.head (lib.splitString ";" (lib.concatStringsSep name (lib.drop (i + 1) pieces)))
        );
      };
      next = one: builtins.substring 0 1 one.after;
      # A spread's dots open a value, not a property.
      property = one: one.last == "." && !lib.hasSuffix "..." one.before;
      alone = one: !wordy one.last && !property one && !wordy (next one);
    in
    lib.filter alone (lib.genList use (lib.length pieces - 1));
  named = name: text: lib.length (uses name text);
  # STRIDING is declared once, and everywhere else read by a key, or keys
  # in a row, each holding keys of its own up to three deep, then perhaps
  # by a method that changes nothing, assigning nothing there, by itself
  # or with an operator, anywhere up to its semicolon, and neither counted
  # up nor down. A key that can't be read whole, or any other method,
  # fails it.
  onlyRead =
    let
      by = "(\\*\\*|<<|>>>?|&&|\\|\\||\\?\\?|[-+*/%&|^])?";
      inner = "\\[([^][]|\\[[^][]*])*]";
      key = "\\[([^][]|${inner})*]";
      unchanging = "length|includes|indexOf|lastIndexOf|at|slice|concat|join|map|filter|some|every";
      method = "[[:space:]]*\\??\\.[[:space:]]*(${unchanging})([^A-Za-z0-9_$].*)?";
      # What follows its keys, past a method that changes nothing.
      rest =
        one:
        let
          keyed = builtins.match "(${key})+(.*)" one.after;
          after = if keyed == null then null else lib.last keyed;
          called = builtins.match method (if after == null then "" else after);
        in
        if after == null then
          null
        else if called != null then
          toString (lib.last called)
        else if builtins.match "[[:space:]]*(\\??\\.|\\[).*" after != null then
          null
        else
          after;
      writes = after: builtins.match "[[:space:]]*(${by}=([^=].*)?|(\\+\\+|--).*)" after != null;
      read = one: rest one != null && !writes (rest one) && !bumped one.before;
      ending = n: s: builtins.substring (lib.max 0 (lib.stringLength s - n)) n s;
      bumped = before: builtins.match ".*(\\+\\+|--)[[:space:]]*" (ending 8 before) != null;
      declaring =
        one:
        builtins.match "(.*[^A-Za-z0-9_$])?const[[:space:]]+" (
          builtins.replaceStrings [ "\n" ] [ " " ] (ending 16 one.before)
        ) != null;
      all = uses "STRIDING" ways;
      reads = lib.filter (one: !declaring one) all;
    in
    lib.length (lib.filter declaring all) == 1 && lib.all read reads;
  s = "[[:space:]]*";
  word = "(\"[A-Za-z]+\"|'[A-Za-z]+')";
  stride = "[A-Za-z_]+${s}:${s}WALKS";
  listed = one: "${s}(${one}${s},${s})*${one}${s},?${s}";
  words = map lib.head (lib.filter lib.isList (builtins.split "[\"']([A-Za-z]+)[\"']" walks));
  # What the code isn't, of what it must be, so a walk read as none
  # says why.
  faults = map (rule: rule.wanted) (
    lib.filter (rule: !rule.met) [
      {
        wanted = "WALKS as quoted words";
        met = builtins.match "\\[${listed word}]${s}" walks != null;
      }
      {
        wanted = "STRIDING as strides by WALKS";
        met = builtins.match "\\{${listed stride}}${s}" striding != null;
      }
      {
        wanted = "WALKS named nowhere else";
        met = named "WALKS" ways == 1 + named "WALKS" striding;
      }
      {
        wanted = "STRIDING declared once and only read by a key";
        met = onlyRead;
      }
    ]
  );
in
{
  inherit faults;
  walk = lib.optionals (faults == [ ]) words;
}
