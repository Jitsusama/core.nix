# The walk the margin cat's code takes, read from the JavaScript that walks
# it, for modules/theme/checks.nix to hold its strides to; its own file, as
# it changes only with the cat's code or how that code is read. The check
# proves the walk that's drawn rather than a copy: the words WALKS lists,
# when it's every list STRIDING strides by. Only the code is read, comments
# aside, and each name only as a word of its own, not another's property.
# Each is read whole, to its semicolon, and must be only what's meant,
# spaces allowed only between its parts; elsewhere, WALKS is named only in
# STRIDING, and STRIDING is declared once and otherwise only read by a key
# with nothing assigned there. So anything else, another list, a stride
# written inline, an entry not a quoted word, something after its bracket,
# an assignment to either or either not found, reads as no walk, which
# fails saying where it looked and what it wanted. It guards the walk from
# an honest change, not a hostile one: comments are found by their marks,
# so one inside a string isn't seen, nor a delete, a write through what a
# read hands back or a destructuring pattern, or one from another file;
# none of the cat's files has any.
{ lib }:
file:
let
  # The code with each comment, a line's or a block's, made a space.
  comment = "//[^\n]*|/\\*([^*]|\\*+[^*/])*\\*+/";
  ways = lib.concatMapStrings (piece: if lib.isString piece then piece else " ") (
    builtins.split comment file
  );
  # The constant's declaration, up to the semicolon that ends it.
  declared =
    name:
    let
      after = builtins.split "const ${name}[[:space:]]*=[[:space:]]*" ways ++ [
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
  # in a row, each holding brackets of its own too, assigning nothing
  # there, by itself or with an operator, on its line or the next, and
  # neither counted up nor down.
  onlyRead =
    let
      by = "(\\*\\*|<<|>>>?|&&|\\|\\||\\?\\?|[-+*/%&|^])?";
      key = "\\[([^][]|\\[[^][]*])*]";
      writes =
        one:
        builtins.match "(${key})+[[:space:]]*(${by}=([^=].*)?|(\\+\\+|--).*)" one.after != null
        || bumped one.before;
      bumped =
        before:
        let
          tail = builtins.substring (lib.max 0 (lib.stringLength before - 8)) 8 before;
        in
        builtins.match ".*(\\+\\+|--)[[:space:]]*" tail != null;
      declaring = one: lib.hasSuffix "const " one.before;
      all = uses "STRIDING" ways;
      reads = lib.filter (one: !declaring one) all;
    in
    lib.length (lib.filter declaring all) == 1
    && lib.all (one: lib.hasPrefix "[" one.after && !writes one) reads;
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
