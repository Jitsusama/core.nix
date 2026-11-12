# Markdown files open in the reader, from the launcher, a file chooser's Open
# With or a link. The default goes in the system's list, since the account's
# own mimeapps.list is the browser's to write. It yields to another reader a
# machine imports, such as inlyne, since the graphical role brings this one
# and importing the other is the machine choosing it.
{ lib, ... }:
{
  xdg.mime.defaultApplications."text/markdown" = lib.mkDefault "reader.desktop";
}
