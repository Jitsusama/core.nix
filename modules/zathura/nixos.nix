# zathura opens documents: a PDF a browser downloads or a mail attaches, an
# EPUB, an XPS, a DjVu scan, PostScript. Each kind has a desktop entry of
# its own, the plugin's that reads it, and Chrome claims PDFs too. The
# defaults go in the system's list, since the account's own mimeapps.list
# is the browser's to write.
{ lib, ... }:
let
  mupdf = "org.pwmt.zathura-pdf-mupdf.desktop";
  djvu = "org.pwmt.zathura-djvu.desktop";
  ps = "org.pwmt.zathura-ps.desktop";
in
{
  xdg.mime.defaultApplications =
    lib.genAttrs [
      "application/pdf"
      "application/epub+zip"
      "application/oxps"
      "application/x-fictionbook"
    ] (_: mupdf)
    // lib.genAttrs [
      "image/vnd.djvu"
      "image/vnd.djvu+multipage"
    ] (_: djvu)
    // lib.genAttrs [
      "application/postscript"
      "application/eps"
    ] (_: ps);
}
