# Reads the few shapes of a Rust source's items that the checks pin against
# the locked mycelium: an enum's variants and a struct's fields, each with
# the attributes over it. It fails closed: an item, or any line of one, it
# can't read gives null, never a shorter answer, so what it misses fails a
# check rather than passing it unread.
{ lib }:
let
  # LINES with each attribute rustfmt spreads over several lines joined into
  # one, at the indent it opened at, the comma rustfmt leaves before its
  # close dropped, and null where one never closes. An attribute that
  # neither closes on its line nor opens a list there, as one with a
  # comment after it, reads as UNREAD, which no item takes.
  joined =
    lines:
    let
      step =
        acc: line:
        let
          trimmed = lib.trim line;
        in
        if acc.open != null then
          let
            closing = lib.hasPrefix ")" trimmed;
            tight = lib.hasSuffix "(" acc.open || closing;
            before = if closing then lib.removeSuffix "," acc.open else acc.open;
            open = before + (if tight then "" else " ") + trimmed;
          in
          if lib.hasSuffix ")]" trimmed then
            acc
            // {
              lines = acc.lines ++ [ open ];
              open = null;
            }
          else
            acc // { inherit open; }
        else if lib.hasPrefix "#[" trimmed && lib.hasSuffix "(" trimmed then
          acc // { open = line; }
        else if lib.hasPrefix "#[" trimmed && !lib.hasSuffix "]" trimmed then
          acc // { lines = acc.lines ++ [ unread ]; }
        else
          acc // { lines = acc.lines ++ [ line ]; };
      read = lib.foldl' step {
        lines = [ ];
        open = null;
      } lines;
    in
    if read.open == null then read.lines else null;

  # What an attribute line joined can't read is put as, which no item's
  # attributes or body take.
  unread = "#[?]";

  # Item START's attributes, nearest first, its doc lines left out, and the
  # lines of its body, doc lines left out, in LINES; null where it isn't
  # there, isn't closed, or its attributes don't start after a blank line,
  # as a comment or anything else between them would, or an attribute
  # stands above that blank line, since Rust puts it on the item too. Over
  # it, a cfg_attr may hold anything but serde's words, which it would
  # give the item where its condition holds.
  item =
    lines: start:
    let
      all = joined lines;
      from = lib.lists.findFirstIndex (line: line == start) null all;
      rest = lib.drop (from + 1) all;
      to = lib.lists.findFirstIndex (line: line == "}") null rest;
      before = lib.reverseList (lib.take from all);
      marked = line: lib.hasPrefix "#[" line || lib.hasPrefix "///" line;
      marks = lib.lists.findFirstIndex (line: !marked line) (lib.length before) before;
      edged = marks == lib.length before || lib.elemAt before marks == "";
      # The first line above that blank line that isn't one or a comment.
      spoken =
        line:
        let
          trimmed = lib.trim line;
        in
        trimmed != ""
        && !lib.hasPrefix "//" trimmed
        && !lib.hasPrefix "/*" trimmed
        && !lib.hasPrefix "*" trimmed
        && !lib.hasSuffix "*/" trimmed;
      above = lib.findFirst spoken "" (lib.drop marks before);
      gapped = lib.hasPrefix "#[" (lib.trim above);
      over = lib.filter (lib.hasPrefix "#[") (lib.take marks before);
      # Brackets as bracket expressions, since macOS's engine refuses `\]`.
      known =
        line:
        builtins.match "#[[](derive|serde|cfg_attr)[(].*[)][]]" line != null
        && !(lib.hasPrefix "#[cfg_attr" line && lib.hasInfix "serde" line);
    in
    if
      all == null || from == null || to == null || !edged || gapped || lib.any (line: !known line) over
    then
      null
    else
      {
        inherit over;
        body = lib.filter (line: !lib.hasPrefix "    ///" line) (lib.take to rest);
      };

  # Whether OVER says the item is read from serde's words.
  deserialized = lib.any (
    line: builtins.match "#[[]derive[(](.*, )?Deserialize(, .*)?[)][]]" line != null
  );

  # The serde attributes over FOUND, as written.
  serdeOf = found: lib.filter (lib.hasPrefix "#[serde") found.over;

  # A variant as `{ name, holds }`: its one word lowercased, as serde's
  # rename_all to snake_case or lowercase names it, and the one type it
  # holds, or null.
  variant =
    line:
    let
      read = builtins.match "    ([A-Z][a-z]*)([(]([A-Za-z]+)[)])?," line;
    in
    if read == null then
      null
    else
      {
        name = lib.toLower (lib.elemAt read 0);
        holds = lib.elemAt read 2;
      };

  # A field line as `{ name, type }`, or null.
  field =
    line:
    let
      read = builtins.match "    pub ([a-z_][a-z0-9_]*): (.*)," line;
    in
    if read == null then
      null
    else
      {
        name = lib.elemAt read 0;
        type = lib.elemAt read 1;
      };

  # A field's serde attribute's words, or null where the line is none.
  fieldSerde =
    line:
    let
      read = builtins.match "    #[[]serde[(](.*)[)][]]" line;
    in
    if read == null then null else lib.head read;

  # Whether a field's serde WORDS are only ones that leave it read by its
  # own name, in its place: a default, when it's written, and a reader of
  # its value. Any other, as a rename, an alias, a flatten, a skip or a
  # `with`, isn't taken, so a word serde comes to know fails the check
  # rather than passing unread.
  plain =
    words:
    lib.all (
      word:
      builtins.match "default|(default|skip_serializing_if|deserialize_with) = \"[^\"]*\"" word != null
    ) (lib.splitString ", " words);
in
{
  # Enum ENUM in LINES as `{ over, serde, variants }`: the attributes over
  # it, its serde ones alone, and each variant as `{ name, holds }`; null where it isn't there,
  # isn't read from serde's words, or any line of it isn't a plain variant
  # or one holding one type, as a renamed, skipped or struct-like one isn't.
  enumIn =
    lines: enum:
    let
      found = item lines "pub enum ${enum} {";
      variants = map variant found.body;
    in
    if found == null || !deserialized found.over || lib.any (one: one == null) variants then
      null
    else
      {
        inherit (found) over;
        serde = serdeOf found;
        inherit variants;
      };

  # Struct STRUCT in LINES as `{ over, serde, fields }`: the attributes over
  # it, its serde ones alone, and each field as `{ name, type, serde }`, `serde` the words of the
  # serde attributes over the field; null where it isn't there, isn't read
  # from serde's words, or any line of it isn't a public field on one line
  # or a serde attribute over one, or a field's serde says more than how
  # it defaults, when it's written and what reads its value (plain).
  structIn =
    lines: struct:
    let
      found = item lines "pub struct ${struct} {";
      step =
        acc: line:
        let
          words = fieldSerde line;
          one = field line;
        in
        if acc == null then
          null
        else if words != null then
          acc // { pending = acc.pending ++ [ words ]; }
        else if one != null && lib.all plain acc.pending then
          {
            fields = acc.fields ++ [ (one // { serde = acc.pending; }) ];
            pending = [ ];
          }
        else
          null;
      read = lib.foldl' step {
        fields = [ ];
        pending = [ ];
      } found.body;
    in
    if found == null || !deserialized found.over || read == null || read.pending != [ ] then
      null
    else
      {
        inherit (found) over;
        serde = serdeOf found;
        inherit (read) fields;
      };
}
