# The smallest PDF mupdf opens without repairing it, of empty pages, one
# unless asked for more, for opening documents in checks and tests: its
# table of where each object starts has to be right to the byte, so it's
# counted rather than written.
{
  lib,
  pkgs,
  pages ? 1,
}:
let
  # Each page is the object after the catalog's and the pages' own.
  numbers = lib.range 3 (pages + 2);
  objects = [
    "<< /Type /Catalog /Pages 2 0 R >>"
    "<< /Type /Pages /Kids [${
      lib.concatMapStringsSep " " (n: "${toString n} 0 R") numbers
    }] /Count ${toString pages} >>"
  ]
  ++ map (_: "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] >>") numbers;
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
