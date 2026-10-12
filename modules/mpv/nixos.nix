# mpv opens video, so a recording's notice, a file manager or a download
# plays it there. The defaults go in the system's list, since the account's
# own mimeapps.list is the browser's to write.
{ lib, ... }:
let
  kinds = [
    "video/mp4"
    "video/webm"
    "video/x-matroska"
    "video/quicktime"
    "video/x-msvideo"
    "video/mpeg"
    "video/ogg"
    "video/x-flv"
    "video/3gpp"
    "video/mp2t"
  ];
in
{
  xdg.mime.defaultApplications = lib.genAttrs kinds (_: "mpv.desktop");
}
