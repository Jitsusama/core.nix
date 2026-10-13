# swayimg opens pictures and drawings to look at: a screenshot, a
# download, a photo, an SVG. GIMP and Inkscape also say they open them, but
# as programs to edit in, slower to start and more than a look needs, and
# swayimg's e opens either. Chrome claims them too. The defaults go in the
# system's list, since the account's own mimeapps.list is the browser's to
# write.
{ lib, ... }:
let
  kinds = [
    "image/png"
    "image/jpeg"
    "image/gif"
    "image/webp"
    "image/avif"
    "image/heif"
    "image/jxl"
    "image/tiff"
    "image/bmp"
    "image/svg+xml"
    "image/x-exr"
    "image/x-tga"
    "image/x-portable-anymap"
    "image/x-portable-bitmap"
    "image/x-portable-graymap"
    "image/x-portable-pixmap"
  ];
in
{
  xdg.mime.defaultApplications = lib.genAttrs kinds (_: "swayimg.desktop");
}
