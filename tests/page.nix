# The smallest PDF mupdf opens without repairing it, of empty pages, one
# unless asked for more, for opening documents in checks and tests: its
# table of where each object starts has to be right to the byte, so it's
# counted rather than written. A page given a word in `words`, by its
# place, shows it, for a search to find; the rest stay empty.
{
  lib,
  pkgs,
  pages ? 1,
  words ? [ ],
}:
let
  # Each page is the object after the catalog's and the pages' own, then
  # the font the words are in, then each worded page's words.
  numbers = lib.range 3 (pages + 2);
  word = at: if at < lib.length words then lib.elemAt words at else "";
  worded = lib.filter (at: word at != "") (lib.range 0 (pages - 1));
  font = pages + 3;
  contents = at: font + 1 + lib.lists.findFirstIndex (each: each == at) 0 worded;
  shown = at: "BT /F1 24 Tf 20 100 Td (${word at}) Tj ET";
  page =
    at:
    if word at == "" then
      "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] >>"
    else
      "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] "
      + "/Resources << /Font << /F1 ${toString font} 0 R >> >> /Contents ${toString (contents at)} 0 R >>";
  objects = [
    "<< /Type /Catalog /Pages 2 0 R >>"
    "<< /Type /Pages /Kids [${
      lib.concatMapStringsSep " " (n: "${toString n} 0 R") numbers
    }] /Count ${toString pages} >>"
  ]
  ++ map page (lib.range 0 (pages - 1))
  ++ lib.optional (worded != [ ]) "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>"
  ++ map (
    at: "<< /Length ${toString (lib.stringLength (shown at))} >>\nstream\n${shown at}\nendstream"
  ) worded;
  head = "%PDF-1.4\n";
  written = lib.imap1 (n: object: "${toString n} 0 obj\n${object}\nendobj\n") objects;
  starts = lib.foldl' (at: object: at ++ [ (lib.last at + builtins.stringLength object) ]) [
    (builtins.stringLength head)
  ] written;
  entry = at: "${lib.fixedWidthString 10 "0" (toString at)} 00000 n \n";
  count = toString (lib.length objects + 1);
in
pkgs.writeText "page.pdf" (
  head
  + lib.concatStrings written
  + "xref\n0 ${count}\n0000000000 65535 f \n"
  + lib.concatMapStrings entry (lib.init starts)
  + "trailer\n<< /Size ${count} /Root 1 0 R >>\nstartxref\n${toString (lib.last starts)}\n%%EOF\n"
)
