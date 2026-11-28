# Reads the few shapes of a Rust source's items that the checks pin against
# the locked mycelium: an enum's variants and a struct's fields, each with
# the attributes over it. It fails closed: an item, or any line of one, it
# can't read gives null, never a shorter answer, so what it misses fails a
# check rather than passing it unread.
{ lib }:
let
  # LINES with each attribute rustfmt spreads over several lines joined into
  # one, at the indent it opened at, and null where one never closes.
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
            tight = lib.hasSuffix "(" acc.open || lib.hasPrefix ")" trimmed;
            open = acc.open + (if tight then "" else " ") + trimmed;
          in
          if lib.hasSuffix ")]" trimmed then
            acc
            // {
              lines = acc.lines ++ [ open ];
              open = null;
            }
          else
            acc // { inherit open; }
        else if lib.hasPrefix "#[" trimmed && !lib.hasSuffix "]" trimmed then
          acc // { open = line; }
        else
          acc // { lines = acc.lines ++ [ line ]; };
      read = lib.foldl' step {
        lines = [ ];
        open = null;
      } lines;
    in
    if read.open == null then read.lines else null;

  # Item START's attributes, nearest first, its doc lines left out, and the
  # lines of its body, doc lines left out, in LINES; null where it isn't
  # there, isn't closed, or its attributes don't start after a blank line,
  # as a comment or anything else between them would.
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
      over = lib.filter (lib.hasPrefix "#[") (lib.take marks before);
      # Brackets as bracket expressions, since macOS's engine refuses `\]`.
      known = builtins.match "#[[](derive|serde|cfg_attr)[(].*[)][]]";
    in
    if
      all == null || from == null || to == null || !edged || lib.any (line: known line == null) over
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

  # Words that name a field otherwise than its own name, or take it out of
  # its place, so its name alone isn't what serde reads.
  renames = words: builtins.match ".*(rename|alias|flatten|skip_deserializing).*" words != null;
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
  # or a serde attribute over one, or a field is renamed, aliased, flattened
  # or never read.
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
        else if one != null && !lib.any renames acc.pending then
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
