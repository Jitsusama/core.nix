# fd, for finding files by name, faster and simpler than find.
{ pkgs, ... }:
{
  home.packages = [ pkgs.fd ];
}
