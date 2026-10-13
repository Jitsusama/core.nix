# The smallest PDF mupdf opens without repairing it, one empty page, for
# opening documents in checks and tests: its table of where each object
# starts has to be right to the byte, so it's counted rather than written.
{ lib, pkgs }:
let
  objects = [
    "<< /Type /Catalog /Pages 2 0 R >>"
    "<< /Type /Pages /Kids [3 0 R] /Count 1 >>"
    "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] >>"
  ];
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
