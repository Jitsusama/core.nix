# Inkscape opens SVG drawings. GIMP also says it can, but it opens one as a
# picture, flattened to pixels, where Inkscape keeps it a drawing. The default
# goes in the system's list, since the account's own mimeapps.list is the
# browser's to write.
{
  xdg.mime.defaultApplications = {
    "image/svg+xml" = "org.inkscape.Inkscape.desktop";
    "image/svg+xml-compressed" = "org.inkscape.Inkscape.desktop";
  };
}
